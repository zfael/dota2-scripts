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
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const ENDPOINT: &str = "https://api.stratz.com/graphql";

/// STRATZ requires this exact user agent.
const USER_AGENT: &str = "STRATZ_API";

/// Free-tier limits are 20/sec, 250/min, 2000/hour. The minute limit binds
/// first for a bulk refresh, so requests are spaced to stay under it with
/// margin (250/min would be 240ms; 300ms leaves room for retries).
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(300);

/// STRATZ returns 503 in bursts under load — measured at roughly half of all
/// requests during one such spell, hitting the cheapest query as readily as
/// the most expensive, and unaffected by slowing to one request every three
/// seconds. It is their instability, not our pacing, so the only useful
/// response is to keep trying for a while.
const MAX_RETRIES: u32 = 6;

/// First backoff step; doubles each attempt, so six attempts span ~15s.
const INITIAL_BACKOFF: Duration = Duration::from_millis(500);

/// Ceiling on a single backoff wait, so a refresh cannot stall for minutes.
const MAX_BACKOFF: Duration = Duration::from_secs(8);

#[derive(Debug)]
pub enum StratzError {
    /// No token configured — the caller should tell the user to set one
    /// rather than retry.
    MissingToken,
    /// Token rejected. Not retryable. Carries STRATZ's own explanation,
    /// which distinguishes a bad token from a token used from a different IP.
    Unauthorized(String),
    /// Rate limited even after backing off.
    RateLimited,
    /// STRATZ itself is failing (5xx) after every retry. Their problem, not
    /// the caller's — worth saying so plainly, because the natural assumption
    /// on seeing an error here is that the token or the query is wrong.
    Unavailable(u16),
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
            Self::Unauthorized(msg) if msg.is_empty() => {
                write!(f, "STRATZ rejected the API token")
            }
            Self::Unauthorized(msg) => write!(f, "STRATZ rejected the API token: {msg}"),
            Self::RateLimited => write!(f, "STRATZ rate limit reached; try again later"),
            Self::Unavailable(code) => write!(
                f,
                "STRATZ is temporarily unavailable ({code}). Nothing is wrong with your token — \
                 their service is failing requests. Retrying automatically."
            ),
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
                // STRATZ binds a token to the IP that uses it and rejects
                // requests from a different one ("You cannot use different IP
                // Addresses when using the API"). On a dual-stack machine the
                // v4 and v6 addresses are different public IPs, so letting
                // the resolver choose per-connection makes the token work or
                // fail depending on which family won the race. Pinning to
                // IPv4 makes it deterministic.
                .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
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
        let mut backoff = INITIAL_BACKOFF;

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
                        // The body says *why*, and the reasons are very
                        // different: a bad token, versus a valid token used
                        // from a different IP than it was bound to. Reporting
                        // only "rejected" sends the user hunting for a new
                        // token when the token was fine.
                        let detail = resp.text().unwrap_or_default();
                        return Err(StratzError::Unauthorized(summarise(&detail)));
                    }
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        if attempt + 1 == MAX_RETRIES {
                            return Err(StratzError::RateLimited);
                        }
                        std::thread::sleep(jittered(backoff));
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                    if status.is_server_error() {
                        if attempt + 1 == MAX_RETRIES {
                            return Err(StratzError::Unavailable(status.as_u16()));
                        }
                        std::thread::sleep(jittered(backoff));
                        backoff = (backoff * 2).min(MAX_BACKOFF);
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

/// Spread retries over a window instead of hammering in lockstep.
///
/// A bulk refresh is a long series of requests against a service that is
/// already struggling; retrying every one of them on the same schedule turns
/// a wobble into a thundering herd.
fn jittered(backoff: Duration) -> Duration {
    let millis = backoff.as_millis() as u64;
    // Cheap, dependency-free spread over [75%, 125%] of the interval.
    let spread = millis / 4;
    let nudge = if spread == 0 {
        0
    } else {
        (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0))
            % (spread * 2)
    };
    Duration::from_millis(millis.saturating_sub(spread) + nudge)
}

/// Reduce an error body to a short single-line message.
///
/// STRATZ answers with `{"message":"..."}`; anything else is truncated rather
/// than pasted wholesale into a log line.
fn summarise(body: &str) -> String {
    let text = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_string))
        .unwrap_or_else(|| body.to_string());
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.len() > 200 {
        format!("{}…", &text[..200])
    } else {
        text
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
