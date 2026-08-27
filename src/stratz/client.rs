//! Transport for the STRATZ GraphQL API.
//!
//! Two things about this API are easy to get wrong and fail confusingly:
//!
//! - **`User-Agent: STRATZ_API` is mandatory.** Without it requests are
//!   rejected in a way that looks like an auth problem.
//! - **Everything is token-gated**, including schema introspection. There is
//!   no anonymous read path to fall back on.
//!
//! The token is a secret. It is never logged, never written to telemetry, and
//! [`StratzClient`]'s `Debug` is hand-written so it cannot leak through a
//! `{:?}` on some enclosing struct.

use std::fmt;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://api.stratz.com/graphql";

/// STRATZ requires this exact user agent.
const USER_AGENT: &str = "STRATZ_API";

/// Free-tier limits are 20/sec, 250/min, 2000/hour. The minute limit binds
/// first for a bulk refresh, so requests are spaced to stay under it with
/// margin (250/min would be 240ms; 300ms leaves room for retries).
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(300);

const MAX_RETRIES: u32 = 4;

#[derive(Debug)]
pub enum StratzError {
    /// No token configured — the caller should tell the user to set one
    /// rather than retry.
    MissingToken,
    /// Token rejected. Not retryable.
    Unauthorized,
    /// Rate limited even after backing off.
    RateLimited,
    Http(String),
    /// The API answered with a GraphQL `errors` array.
    Graphql(String),
    Decode(String),
}

impl fmt::Display for StratzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingToken => write!(
                f,
                "no STRATZ API token configured (set [stratz] api_token, or the STRATZ_TOKEN environment variable)"
            ),
            Self::Unauthorized => write!(f, "STRATZ rejected the API token"),
            Self::RateLimited => write!(f, "STRATZ rate limit reached; try again later"),
            Self::Http(e) => write!(f, "STRATZ request failed: {e}"),
            Self::Graphql(e) => write!(f, "STRATZ returned errors: {e}"),
            Self::Decode(e) => write!(f, "STRATZ response did not match the expected shape: {e}"),
        }
    }
}

impl std::error::Error for StratzError {}

pub struct StratzClient {
    http: reqwest::blocking::Client,
    token: String,
    last_request: Option<Instant>,
}

// Hand-written so the token cannot escape through a derived Debug anywhere up
// the ownership chain.
impl fmt::Debug for StratzClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StratzClient")
            .field("token", &"<redacted>")
            .finish()
    }
}

impl StratzClient {
    /// `token` may be empty; the error surfaces on first use rather than here,
    /// so construction stays infallible for callers that build one eagerly.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            http: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(45))
                .build()
                .unwrap_or_default(),
            token: token.into(),
            last_request: None,
        }
    }

    /// Resolve a token from explicit config, falling back to the environment.
    ///
    /// The environment variable exists so the secret need never be written to
    /// a config file on disk.
    pub fn resolve_token(configured: &str) -> String {
        if !configured.trim().is_empty() {
            return configured.trim().to_string();
        }
        std::env::var("STRATZ_TOKEN").unwrap_or_default().trim().to_string()
    }

    /// Run one GraphQL query, returning its `data` object.
    ///
    /// Retries on 429 and 5xx with exponential backoff; 401/403 fail
    /// immediately, since retrying a bad token only burns quota.
    pub fn query(
        &mut self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, StratzError> {
        if self.token.is_empty() {
            return Err(StratzError::MissingToken);
        }

        let body = serde_json::json!({ "query": query, "variables": variables });
        let mut backoff = Duration::from_millis(500);

        for attempt in 0..MAX_RETRIES {
            self.throttle();

            let response = self
                .http
                .post(ENDPOINT)
                .bearer_auth(&self.token)
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .json(&body)
                .send();

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status == reqwest::StatusCode::UNAUTHORIZED
                        || status == reqwest::StatusCode::FORBIDDEN
                    {
                        return Err(StratzError::Unauthorized);
                    }
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        if attempt + 1 == MAX_RETRIES {
                            return Err(StratzError::RateLimited);
                        }
                        std::thread::sleep(backoff);
                        backoff *= 2;
                        continue;
                    }
                    if status.is_server_error() {
                        if attempt + 1 == MAX_RETRIES {
                            return Err(StratzError::Http(format!("server error {status}")));
                        }
                        std::thread::sleep(backoff);
                        backoff *= 2;
                        continue;
                    }
                    if !status.is_success() {
                        return Err(StratzError::Http(format!("unexpected status {status}")));
                    }

                    let value: serde_json::Value = resp
                        .json()
                        .map_err(|e| StratzError::Decode(e.to_string()))?;
                    return extract_data(value);
                }
                Err(e) => {
                    if attempt + 1 == MAX_RETRIES {
                        // Deliberately not `{e:?}` — a reqwest error's Debug
                        // can include the request URL and headers.
                        return Err(StratzError::Http(e.to_string()));
                    }
                    std::thread::sleep(backoff);
                    backoff *= 2;
                }
            }
        }

        Err(StratzError::Http("exhausted retries".into()))
    }

    fn throttle(&mut self) {
        if let Some(last) = self.last_request {
            let elapsed = last.elapsed();
            if elapsed < MIN_REQUEST_INTERVAL {
                std::thread::sleep(MIN_REQUEST_INTERVAL - elapsed);
            }
        }
        self.last_request = Some(Instant::now());
    }
}

/// Pull `data` out of a GraphQL envelope, surfacing `errors` as a failure.
///
/// GraphQL answers HTTP 200 with an `errors` array, so a purely status-based
/// client reports success while handing back nulls.
fn extract_data(value: serde_json::Value) -> Result<serde_json::Value, StratzError> {
    if let Some(errors) = value.get("errors") {
        let summary = errors
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| errors.to_string());
        return Err(StratzError::Graphql(summary));
    }
    value
        .get("data")
        .cloned()
        .ok_or_else(|| StratzError::Decode("response had neither data nor errors".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphql_errors_are_failures_even_on_http_200() {
        // The trap this guards: STRATZ answers 200 with an errors array and a
        // null data field. Status-only handling reports success and the
        // caller silently builds an empty dataset.
        let body = serde_json::json!({
            "errors": [{ "message": "Rate limit exceeded" }],
            "data": null
        });
        match extract_data(body) {
            Err(StratzError::Graphql(msg)) => assert!(msg.contains("Rate limit")),
            other => panic!("expected a Graphql error, got {other:?}"),
        }
    }

    #[test]
    fn multiple_graphql_errors_are_all_reported() {
        let body = serde_json::json!({
            "errors": [{ "message": "first" }, { "message": "second" }]
        });
        match extract_data(body) {
            Err(StratzError::Graphql(msg)) => {
                assert!(msg.contains("first") && msg.contains("second"));
            }
            other => panic!("expected a Graphql error, got {other:?}"),
        }
    }

    #[test]
    fn data_is_returned_when_present() {
        let body = serde_json::json!({ "data": { "constants": { "heroes": [] } } });
        let data = extract_data(body).unwrap();
        assert!(data["constants"]["heroes"].is_array());
    }

    #[test]
    fn a_response_with_neither_field_is_a_decode_error() {
        match extract_data(serde_json::json!({ "unexpected": true })) {
            Err(StratzError::Decode(_)) => {}
            other => panic!("expected a Decode error, got {other:?}"),
        }
    }

    #[test]
    fn missing_token_fails_before_any_request() {
        let mut client = StratzClient::new("");
        match client.query("{ __typename }", serde_json::Value::Null) {
            Err(StratzError::MissingToken) => {}
            other => panic!("expected MissingToken, got {other:?}"),
        }
    }

    #[test]
    fn debug_never_prints_the_token() {
        let client = StratzClient::new("eyJsupersecret.token.value");
        let rendered = format!("{client:?}");
        assert!(!rendered.contains("supersecret"), "token leaked: {rendered}");
        assert!(rendered.contains("redacted"));
    }

    #[test]
    fn configured_token_wins_over_the_environment() {
        assert_eq!(StratzClient::resolve_token("  configured  "), "configured");
    }
}
