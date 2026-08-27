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

/// Published limits for a STRATZ Default Token, as `(window_ms, max_calls)`.
///
/// All four are enforced together by [`RateLimiter`]. Enforcing them as
/// *sliding* windows is strictly stricter than the fixed windows a server
/// typically uses — never exceeding N in any sliding window implies never
/// exceeding N in any fixed one — so matching the published numbers exactly
/// is safe rather than borderline.
const RATE_LIMITS: [(u64, usize); 4] = [
    (1_000, 20),         // 20 / second
    (60_000, 250),       // 250 / minute
    (3_600_000, 2_000),  // 2,000 / hour
    (86_400_000, 10_000) // 10,000 / day
];

/// STRATZ returns 503 in bursts under load — measured at roughly half of all
/// requests during one such spell, hitting the cheapest query as readily as
/// the most expensive, and unaffected by slowing to one request every three
/// seconds. It is their instability, not our pacing, so the only useful
/// response is to keep trying for a while.
const MAX_RETRIES: u32 = 6;

/// Attempts for a request the whole refresh depends on. At an observed 80%
/// failure rate, six attempts lose everything 26% of the time; sixteen brings
/// that under 3%, spread across roughly two minutes of backoff.
const PERSISTENT_RETRIES: u32 = 16;

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

/// Sliding-window limiter enforcing every published tier at once.
///
/// Every outbound request passes through this, **including retries** — a
/// retry is an API call and counts against the same budget. The previous
/// fixed 300ms spacing did not account for them, so a burst of 503s could
/// quietly multiply the real call rate by six.
#[derive(Debug, Default)]
struct RateLimiter {
    /// Millisecond timestamps of recent calls, oldest first.
    history: std::collections::VecDeque<u64>,
}

impl RateLimiter {
    /// How long to wait before a call at `now_ms` would be within every
    /// window. Pure, so the policy is testable without sleeping.
    fn delay_ms(history: &std::collections::VecDeque<u64>, now_ms: u64) -> u64 {
        let mut wait = 0u64;
        for (window, max) in RATE_LIMITS {
            // Measured as age rather than against a `now - window` cutoff:
            // that form saturates at the clock's origin, where it silently
            // treats the calls already made as having aged out.
            let in_window = history
                .iter()
                .rev()
                .take_while(|&&t| now_ms.saturating_sub(t) < window)
                .count();
            if in_window < max {
                continue;
            }
            // The oldest call inside this window has to age out before
            // another is allowed.
            let nth_newest = history.len().saturating_sub(max);
            if let Some(&oldest) = history.get(nth_newest) {
                wait = wait.max((oldest + window).saturating_sub(now_ms));
            }
        }
        wait
    }

    /// Block until a call is permitted, then record it.
    fn acquire(&mut self, started: Instant) {
        loop {
            let now = started.elapsed().as_millis() as u64;
            self.prune(now);
            let wait = Self::delay_ms(&self.history, now);
            if wait == 0 {
                self.history.push_back(now);
                return;
            }
            std::thread::sleep(Duration::from_millis(wait.min(60_000)));
        }
    }

    /// Drop calls older than the longest window, bounding memory at the
    /// daily cap.
    ///
    /// Age-based for the same reason as [`Self::delay_ms`]: a `now - window`
    /// cutoff saturates at zero, and since the clock restarts with every
    /// client, that is where a refresh actually runs. In that form the second
    /// call of every refresh discarded the first, and so on — the limiter
    /// would have tracked almost nothing.
    fn prune(&mut self, now_ms: u64) {
        let longest = RATE_LIMITS.iter().map(|(w, _)| *w).max().unwrap_or(0);
        while self
            .history
            .front()
            .is_some_and(|&t| now_ms.saturating_sub(t) >= longest)
        {
            self.history.pop_front();
        }
    }
}

pub struct StratzClient {
    http: reqwest::blocking::Client,
    token: String,
    limiter: RateLimiter,
    /// Monotonic origin for the limiter's timestamps.
    started: Instant,
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
            limiter: RateLimiter::default(),
            started: Instant::now(),
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
        self.query_with_retries(query, variables, MAX_RETRIES)
    }

    /// Like [`Self::query`], but keeps trying for much longer.
    ///
    /// For the one request everything else depends on. During a spell where
    /// STRATZ failed 80% of requests, the standard six attempts had a 26%
    /// chance of losing all six — and losing the hero list aborts the whole
    /// refresh, where losing any single hero's matchups costs only that hero.
    pub fn query_persistent(
        &mut self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, StratzError> {
        self.query_with_retries(query, variables, PERSISTENT_RETRIES)
    }

    fn query_with_retries(
        &mut self,
        query: &str,
        variables: serde_json::Value,
        max_retries: u32,
    ) -> Result<serde_json::Value, StratzError> {
        if self.token.is_empty() {
            return Err(StratzError::MissingToken);
        }

        let body = serde_json::json!({ "query": query, "variables": variables });
        let mut backoff = INITIAL_BACKOFF;

        for attempt in 0..max_retries {
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
                        if attempt + 1 == max_retries {
                            return Err(StratzError::RateLimited);
                        }
                        // The server knows when the window reopens; prefer
                        // its answer to guessing with a backoff curve.
                        let wait = retry_after(resp.headers()).unwrap_or_else(|| jittered(backoff));
                        std::thread::sleep(wait.min(MAX_RETRY_AFTER));
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                    if status.is_server_error() {
                        if attempt + 1 == max_retries {
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
                    if attempt + 1 == max_retries {
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
        self.limiter.acquire(self.started);
    }
}

/// Cap on an honoured `Retry-After`, so a wildly long value cannot park a
/// refresh for an hour.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

/// `Retry-After` as a delay, when the server sends one in delta-seconds form.
fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
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

    /// Replay `count` calls through the limiter's policy starting at
    /// `start_ms`, returning the timestamp of each. Pure — no sleeping.
    fn simulate(count: usize, start_ms: u64) -> Vec<u64> {
        let mut history = std::collections::VecDeque::new();
        let mut now = start_ms;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            now += RateLimiter::delay_ms(&history, now);
            history.push_back(now);
            out.push(now);
            // Assume requests return instantly, the worst case for rate.
        }
        out
    }

    /// Highest number of calls found in any sliding window of `window` ms.
    fn peak_in_window(times: &[u64], window: u64) -> usize {
        times
            .iter()
            .map(|&t| times.iter().filter(|&&o| o > t.saturating_sub(window) && o <= t).count())
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn every_published_limit_is_respected() {
        // A day's worth of calls, checked against all four tiers at once.
        let times = simulate(3_000, 0);
        for (window, max) in RATE_LIMITS {
            let peak = peak_in_window(&times, window);
            assert!(
                peak <= max,
                "{peak} calls in a {window}ms window exceeds the limit of {max}"
            );
        }
    }

    #[test]
    fn a_full_refresh_is_not_slowed_more_than_necessary() {
        // ~132 requests is one dataset refresh. The minute tier binds first
        // (250/min = 240ms apart), so it should take about half a minute --
        // if this regresses to minutes, the limiter is over-throttling.
        let times = simulate(132, 0);
        let elapsed = times.last().copied().unwrap_or(0);
        assert!(elapsed < 45_000, "a refresh took {elapsed}ms");
        assert!(peak_in_window(&times, 1_000) <= 20);
    }

    #[test]
    fn the_per_second_burst_is_capped() {
        // 20/second is the tightest tier and the easiest to breach, since a
        // fresh limiter will happily fire the first calls back to back.
        let times = simulate(25, 0);
        assert!(peak_in_window(&times, 1_000) <= 20);
        // The 21st call must wait for the first to age out of the window.
        assert!(times[20] >= 1_000, "21st call at {}ms", times[20]);
    }

    #[test]
    fn retries_count_against_the_budget() {
        // The bug this replaces: retries went through a fixed 300ms sleep
        // that tracked nothing, so a burst of 503s could multiply the real
        // call rate several-fold while appearing to be throttled.
        let mut history = std::collections::VecDeque::new();
        let mut now = 0u64;
        for _ in 0..20 {
            now += RateLimiter::delay_ms(&history, now);
            history.push_back(now);
        }
        // Twenty calls used the whole second; a retry is a call like any
        // other and must wait rather than slipping through.
        assert!(RateLimiter::delay_ms(&history, now) > 0);
    }

    #[test]
    fn pruning_bounds_memory_at_the_daily_window() {
        let mut limiter = RateLimiter::default();
        limiter.history.extend([0u64, 1, 2, 3]);
        // A day and a bit later, none of those calls still count.
        limiter.prune(86_400_000 + 10);
        assert!(limiter.history.is_empty());
    }

    #[test]
    fn pruning_keeps_recent_calls_near_the_clock_origin() {
        // The limiter's clock restarts with every client, so a refresh runs
        // near zero. A `now - window` cutoff saturates there and prunes the
        // calls just recorded, leaving the limiter tracking nothing.
        let mut limiter = RateLimiter::default();
        limiter.history.extend([0u64, 50, 100]);
        limiter.prune(150);
        assert_eq!(limiter.history.len(), 3, "recent calls were pruned away");
    }

    #[test]
    fn a_fresh_client_still_enforces_the_burst_limit() {
        // End-to-end over the real acquire path, which prunes then decides.
        // This is the shape of every refresh: a brand new client, clock at
        // zero, firing requests as fast as it is allowed.
        let mut limiter = RateLimiter::default();
        for _ in 0..20 {
            limiter.prune(0);
            limiter.history.push_back(0);
        }
        limiter.prune(0);
        assert_eq!(limiter.history.len(), 20);
        assert!(
            RateLimiter::delay_ms(&limiter.history, 0) >= 1_000,
            "the 21st call in the first second was allowed through"
        );
    }

    #[test]
    fn an_idle_client_never_waits() {
        assert_eq!(RateLimiter::delay_ms(&Default::default(), 0), 0);
        let mut history = std::collections::VecDeque::new();
        history.push_back(0u64);
        // Well past every window: no reason to hold the next call.
        assert_eq!(RateLimiter::delay_ms(&history, 90_000_000), 0);
    }

    #[test]
    fn retry_after_is_read_when_the_server_sends_one() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "30".parse().unwrap());
        assert_eq!(retry_after(&headers), Some(Duration::from_secs(30)));

        // HTTP-date form is valid but not parsed; falling back to our own
        // backoff is correct, and must not panic.
        headers.insert(
            reqwest::header::RETRY_AFTER,
            "Wed, 21 Oct 2026 07:28:00 GMT".parse().unwrap(),
        );
        assert_eq!(retry_after(&headers), None);
        assert_eq!(retry_after(&reqwest::header::HeaderMap::new()), None);
    }

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
