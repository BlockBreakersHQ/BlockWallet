//! The one place HTTP requests to chain nodes are made.
//!
//! Every call in this app talks to an endpoint the user chose, and the defaults are public
//! third-party services. `reqwest`'s defaults have no timeout at all and no bound on response
//! size, which on a phone means a node that stops responding pins a thread forever, and a node
//! that answers with a multi-gigabyte body takes the process out on a device with 3 GB of RAM.
//!
//! Both are ordinary failure modes for a public endpoint under load, not just attacks, which
//! is why the bounds live here rather than at any individual call site.

use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use crate::configuration::block_error;

/// Whole-request budget. Generous enough for a slow Esplora address query over a phone's
/// mobile data, short enough that a wedged node surfaces as an error rather than a hang.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Separate, tighter budget for establishing the connection.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Ceiling on a response body. The largest thing this wallet legitimately fetches is an
/// address transaction list; 8 MiB is far above that and far below what would hurt.
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

fn shared() -> Result<&'static reqwest::blocking::Client, block_error::Error> {
    static CLIENT: OnceLock<Result<reqwest::blocking::Client, String>> = OnceLock::new();
    match CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            // Connection reuse matters here: the sync loop makes several requests to the same
            // host in a row, and a fresh TLS handshake each time is slow on this hardware.
            .pool_idle_timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| e.to_string())
    }) {
        Ok(client) => Ok(client),
        Err(why) => Err(block_error::Error::new(format!("could not build http client: {why}"))),
    }
}

/// Read a response body, refusing anything over [`MAX_RESPONSE_BYTES`].
///
/// Reads one byte past the limit so a body sitting exactly on the boundary is reported as
/// oversized rather than silently truncated into a parse error further up.
fn read_capped(response: reqwest::blocking::Response) -> Result<String, block_error::Error> {
    let status = response.status();
    let host = response
        .url()
        .host_str()
        .unwrap_or("the server")
        .to_string();

    let mut buf = Vec::new();
    let mut limited = response.take((MAX_RESPONSE_BYTES + 1) as u64);
    limited
        .read_to_end(&mut buf)
        .map_err(|e| block_error::Error::new(format!("could not read response from {host}: {e}")))?;
    if buf.len() > MAX_RESPONSE_BYTES {
        return Err(block_error::Error::new(
            "response from the node was too large to process".to_string(),
        ));
    }
    let text = String::from_utf8(buf)
        .map_err(|_| block_error::Error::new("response from the node was not valid UTF-8".to_string()))?;

    // A failed status whose body is JSON is handed back rather than rejected. Several of the
    // APIs here answer 400 with their own explanation (THORChain says "swapping is halted",
    // KyberSwap reports insufficient liquidity), and those messages are far more useful than
    // the status code. The caller parses them.
    //
    // A failed status whose body is *not* JSON is a different thing: a rate-limit page, a
    // gateway error page, an HTML block. Passing that to a JSON parser produced
    // "expected value at line 1 column 1", which is what a KyberSwap failure looked like on
    // the phone and says nothing about the actual problem.
    if !status.is_success() && !looks_like_json(&text) {
        return Err(block_error::Error::new(format!(
            "{host} returned HTTP {}{}",
            status.as_u16(),
            excerpt(&text)
        )));
    }
    Ok(text)
}

fn looks_like_json(text: &str) -> bool {
    matches!(text.trim_start().as_bytes().first(), Some(b'{') | Some(b'['))
}

/// A short, single-line taste of a response body, for diagnosing a non-JSON reply.
///
/// Bounded and stripped of control characters so an HTML page cannot smear across the UI or
/// inject line breaks into a message that is rendered as a single row.
fn excerpt(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return " with an empty body".to_string();
    }
    let mut short: String = trimmed.chars().take(80).collect();
    if trimmed.chars().count() > 80 {
        short.push('…');
    }
    format!(": {short}")
}

/// GET a URL and return the body as text, bounded in both time and size.
pub fn get_text(url: &str) -> Result<String, block_error::Error> {
    let response = shared()?.get(url).send()?;
    read_capped(response)
}

/// POST a body to a URL and return the response as text, bounded in both time and size.
pub fn post_text(url: &str, body: String) -> Result<String, block_error::Error> {
    let response = shared()?.post(url).body(body).send()?;
    read_capped(response)
}

/// POST a JSON value and return the response as text, bounded in both time and size.
pub fn post_json(url: &str, body: &serde_json::Value) -> Result<String, block_error::Error> {
    let response = shared()?.post(url).json(body).send()?;
    read_capped(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_client_builds_with_bounds_applied() {
        assert!(shared().is_ok());
        assert!(REQUEST_TIMEOUT >= CONNECT_TIMEOUT);
    }

    #[test]
    fn an_unreachable_host_errors_rather_than_hanging() {
        // `.invalid` is reserved by RFC 2606 and never resolves, so this exercises the
        // failure path without touching the network.
        assert!(get_text("https://blockwallet-test.invalid/api").is_err());
    }

    #[test]
    fn a_failed_status_with_an_html_body_names_the_status_not_a_parse_error() {
        // What a rate-limit or gateway page looks like. Handing this to a JSON parser gave
        // "expected value at line 1 column 1", which is what a KyberSwap failure showed on
        // the phone and explains nothing.
        assert!(!looks_like_json("<!DOCTYPE html><html><body>429</body></html>"));
        assert!(!looks_like_json("   Too Many Requests"));
        // A JSON error body is still passed through, because the API's own message is better
        // than the status code: THORChain says "swapping is halted", Kyber names the reason.
        assert!(looks_like_json("{\"error\":\"swapping is halted\"}"));
        assert!(looks_like_json("  [1,2,3]"));
        assert!(looks_like_json("\n\t{\"code\":4001}"));
    }

    #[test]
    fn an_excerpt_is_short_single_line_and_survives_an_empty_body() {
        let html = "<!DOCTYPE html>\n<html>\n  <head><title>502 Bad Gateway</title></head>\n</html>";
        let out = excerpt(html);
        assert!(!out.contains('\n'), "must not break the row it is rendered in");
        assert!(out.chars().count() <= 84, "bounded: {out}");
        assert!(out.contains("502"), "keeps the useful part: {out}");

        assert_eq!(excerpt(""), " with an empty body");
        assert_eq!(excerpt("   \n\t "), " with an empty body");

        // Long bodies are cut with an ellipsis rather than smeared across the screen.
        let long = "x".repeat(500);
        assert!(excerpt(&long).ends_with('…'));
    }
}
