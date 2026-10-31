use std::time::Duration;

use bytes::Bytes;
use reqwest::header::{HeaderMap, RETRY_AFTER};
#[cfg(any(feature = "graph", feature = "sharepoint"))]
use reqwest::header::{HeaderName, HeaderValue};
use serde::de::DeserializeOwned;

use super::transport::parse_service_error;
use crate::{Error, Result};

/// The outcome of one sub-request of a batch.
///
/// A failing sub-request does not fail the batch call: its status is kept here and
/// [`BatchResult::error`] turns it into the same [`Error::Http`] a single call would give.
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// HTTP status of the sub-request.
    pub status: u16,
    /// Response headers of the sub-request.
    pub headers: HeaderMap,
    /// Raw response body.
    pub body: Bytes,
}

impl BatchResult {
    #[cfg(any(feature = "graph", feature = "sharepoint"))]
    pub(crate) fn new(status: u16, headers: HeaderMap, body: Bytes) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    /// Whether the status is 2xx.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The service error for a failed sub-request, or `None` when it succeeded.
    pub fn error(&self) -> Option<Error> {
        (!self.is_success()).then(|| parse_service_error(self.status, &self.headers, &self.body))
    }

    /// The `Retry-After` delay of a throttled sub-request, when given in seconds.
    pub fn retry_after(&self) -> Option<Duration> {
        self.headers
            .get(RETRY_AFTER)?
            .to_str()
            .ok()?
            .trim()
            .parse()
            .ok()
            .map(Duration::from_secs)
    }

    /// Reads a successful result as JSON; a failed one is returned as its error.
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        match self.error() {
            Some(e) => Err(e),
            None => Ok(serde_json::from_slice(&self.body)?),
        }
    }

    /// The body as (lossy) UTF-8 text.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Builds a header map from name/value pairs, skipping any that are not valid HTTP headers.
#[cfg(any(feature = "graph", feature = "sharepoint"))]
pub(crate) fn header_map<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (k, v) in pairs {
        if let (Ok(k), Ok(v)) = (
            HeaderName::from_bytes(k.trim().as_bytes()),
            HeaderValue::from_str(v.trim()),
        ) {
            map.append(k, v);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_reads_json_and_failure_is_an_error() {
        let ok = BatchResult::new(200, HeaderMap::new(), Bytes::from(r#"{"a":1}"#));
        assert!(ok.is_success() && ok.error().is_none());
        assert_eq!(ok.json::<serde_json::Value>().unwrap()["a"], 1);

        let bad = BatchResult::new(
            404,
            HeaderMap::new(),
            Bytes::from(r#"{"error":{"code":"itemNotFound","message":"gone"}}"#),
        );
        let e = bad.json::<serde_json::Value>().unwrap_err();
        assert_eq!(e.status(), Some(404));
        assert!(
            matches!(bad.error(), Some(Error::Http { code: Some(c), .. }) if c == "itemNotFound")
        );
    }

    #[test]
    fn retry_after_in_seconds() {
        let h = header_map([("Retry-After", "7")]);
        assert_eq!(
            BatchResult::new(429, h, Bytes::new()).retry_after(),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            BatchResult::new(429, HeaderMap::new(), Bytes::new()).retry_after(),
            None
        );
    }
}
