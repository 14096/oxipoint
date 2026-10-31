use std::{sync::Arc, time::Duration};

use reqwest::header::{HeaderMap, RETRY_AFTER};
use serde_json::Value;
use url::Url;

use super::{Body, RawRequest, Response};
use crate::{Error, Result, auth::TokenCredential};

/// Tunable behavior of a [`Transport`].
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Maximum number of retries for 429/503/504 responses.
    pub max_retries: u32,
    /// Delay used when a throttled response has no `Retry-After` header.
    pub default_retry_delay: Duration,
    /// Headers added to every request.
    pub default_headers: Vec<(String, String)>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            default_retry_delay: Duration::from_secs(1),
            default_headers: Vec::new(),
        }
    }
}

/// Authenticated HTTP transport shared by all API handles.
///
/// It attaches a bearer token from the [`TokenCredential`], retries throttled
/// requests and converts error responses to [`Error::Http`].
#[derive(Debug)]
pub struct Transport {
    client: reqwest::Client,
    credential: Arc<dyn TokenCredential>,
    scope: String,
    base_url: String,
    config: TransportConfig,
}

impl Transport {
    /// Creates a transport that requests tokens for `scope` and resolves relative
    /// targets against `base_url`.
    pub fn new(
        credential: Arc<dyn TokenCredential>,
        scope: impl Into<String>,
        base_url: impl Into<String>,
        config: TransportConfig,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            credential,
            scope: scope.into(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            config,
        }
    }

    /// The base URL relative targets are resolved against.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) fn build_url(&self, req: &RawRequest) -> Result<Url> {
        let full = if req.target.starts_with("http://") || req.target.starts_with("https://") {
            req.target.clone()
        } else if req.target.starts_with('/') {
            format!("{}{}", self.base_url, req.target)
        } else {
            format!("{}/{}", self.base_url, req.target)
        };
        let mut url =
            Url::parse(&full).map_err(|e| Error::Config(format!("invalid URL '{full}': {e}")))?;
        if !req.anonymous {
            let base = Url::parse(&self.base_url).map_err(|e| Error::Config(e.to_string()))?;
            let same_host = url.scheme() == base.scheme()
                && url.host_str() == base.host_str()
                && url.port_or_known_default() == base.port_or_known_default();
            if !same_host {
                return Err(Error::Config(format!(
                    "refusing to send credentials to a different host: {}",
                    url.host_str().unwrap_or("")
                )));
            }
        }
        if !req.query.is_empty() {
            url.query_pairs_mut().extend_pairs(req.query.iter());
        }
        Ok(url)
    }

    /// Sends a request, retrying throttled responses, and returns the 2xx response.
    pub async fn send(&self, req: &RawRequest) -> Result<Response> {
        let url = self.build_url(req)?;
        let mut attempt = 0;
        loop {
            let mut rb = self.client.request(req.method.clone(), url.clone());
            if !req.anonymous {
                let token = self.credential.get_token(&self.scope).await?;
                rb = rb.bearer_auth(token.secret().expose());
            }
            for (k, v) in self.config.default_headers.iter().chain(req.headers.iter()) {
                rb = rb.header(k, v);
            }
            rb = match &req.body {
                Body::None => rb,
                Body::Json(v) => rb.json(v),
                Body::Bytes(b) => rb.body(b.clone()),
            };
            let resp = rb.send().await?;
            let status = resp.status();
            let headers = resp.headers().clone();
            let body = resp.bytes().await?;
            if status.is_success() {
                return Ok(Response {
                    status: status.as_u16(),
                    headers,
                    body,
                });
            }
            let retryable = matches!(status.as_u16(), 429 | 503 | 504);
            if retryable && attempt < self.config.max_retries {
                attempt += 1;
                tokio::time::sleep(retry_delay(&headers, self.config.default_retry_delay)).await;
                continue;
            }
            return Err(parse_service_error(status.as_u16(), &headers, &body));
        }
    }
}

fn retry_delay(headers: &HeaderMap, default: Duration) -> Duration {
    headers
        .get(RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(default)
}

/// Converts an error response (Graph, SharePoint light or verbose format) to [`Error::Http`].
pub(crate) fn parse_service_error(status: u16, headers: &HeaderMap, body: &[u8]) -> Error {
    let request_id = ["request-id", "client-request-id", "sprequestguid"]
        .iter()
        .find_map(|h| headers.get(*h))
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let mut code = None;
    let mut message = None;
    if let Ok(doc) = serde_json::from_slice::<Value>(body)
        && let Some(err) = doc.get("error").or_else(|| doc.get("odata.error"))
    {
        code = err.get("code").and_then(Value::as_str).map(str::to_owned);
        message = match err.get("message") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(m) => m.get("value").and_then(Value::as_str).map(str::to_owned),
            None => None,
        };
    }
    let message = message.unwrap_or_else(|| String::from_utf8_lossy(body).into_owned());
    Error::Http {
        status,
        code,
        message,
        request_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::StaticTokenCredential;
    use reqwest::Method;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    fn transport(base: &str, retries: u32) -> Transport {
        Transport::new(
            Arc::new(StaticTokenCredential::new("tok")),
            "scope",
            base,
            TransportConfig {
                max_retries: retries,
                ..Default::default()
            },
        )
    }

    #[tokio::test]
    async fn attaches_bearer_and_returns_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/a"))
            .and(header("authorization", "Bearer tok"))
            .respond_with(ResponseTemplate::new(200).set_body_string("hi"))
            .mount(&server)
            .await;
        let r = transport(&server.uri(), 0)
            .send(&RawRequest::new(Method::GET, "/a"))
            .await
            .unwrap();
        assert_eq!((r.status, r.text()), (200, "hi".to_string()));
    }

    #[tokio::test]
    async fn parses_graph_error() {
        let server = MockServer::start().await;
        Mock::given(path("/x"))
            .respond_with(
                ResponseTemplate::new(404)
                    .insert_header("request-id", "rid")
                    .set_body_json(
                        serde_json::json!({"error":{"code":"itemNotFound","message":"gone"}}),
                    ),
            )
            .mount(&server)
            .await;
        let e = transport(&server.uri(), 0)
            .send(&RawRequest::new(Method::GET, "/x"))
            .await
            .unwrap_err();
        match e {
            Error::Http {
                status,
                code,
                message,
                request_id,
            } => {
                assert_eq!(status, 404);
                assert_eq!(code.as_deref(), Some("itemNotFound"));
                assert_eq!(message, "gone");
                assert_eq!(request_id.as_deref(), Some("rid"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parses_sharepoint_errors() {
        let h = HeaderMap::new();
        let verbose = br#"{"error":{"code":"-1, Sys","message":{"lang":"en","value":"bad"}}}"#;
        let light = br#"{"odata.error":{"code":"c","message":{"value":"m"}}}"#;
        assert!(
            matches!(parse_service_error(400, &h, verbose), Error::Http { message, .. } if message == "bad")
        );
        assert!(
            matches!(parse_service_error(400, &h, light), Error::Http { code: Some(c), .. } if c == "c")
        );
        assert!(
            matches!(parse_service_error(500, &h, b"plain"), Error::Http { message, .. } if message == "plain")
        );
    }

    #[tokio::test]
    async fn retries_429_with_retry_after() {
        let server = MockServer::start().await;
        Mock::given(path("/r"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(path("/r"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let r = transport(&server.uri(), 2)
            .send(&RawRequest::new(Method::GET, "/r"))
            .await
            .unwrap();
        assert_eq!(r.status, 200);
    }

    #[tokio::test]
    async fn gives_up_after_max_retries() {
        let server = MockServer::start().await;
        Mock::given(path("/r"))
            .respond_with(ResponseTemplate::new(503).insert_header("Retry-After", "0"))
            .mount(&server)
            .await;
        let e = transport(&server.uri(), 2)
            .send(&RawRequest::new(Method::GET, "/r"))
            .await
            .unwrap_err();
        assert_eq!(e.status(), Some(503));
        assert_eq!(server.received_requests().await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn rejects_foreign_host_but_allows_anonymous() {
        let t = transport("https://graph.example.com/v1", 0);
        let e = t
            .build_url(&RawRequest::new(Method::GET, "https://evil.example.org/x"))
            .unwrap_err();
        assert!(matches!(e, Error::Config(_)));
        let mut r = RawRequest::new(Method::PUT, "https://upload.example.org/x");
        r.anonymous = true;
        assert!(t.build_url(&r).is_ok());
    }

    #[test]
    fn builds_url_with_query() {
        let t = transport("https://g.example.com/v1/", 0);
        let mut r = RawRequest::new(Method::GET, "/me");
        r.query.push(("$select".into(), "a b".into()));
        assert_eq!(
            t.build_url(&r).unwrap().as_str(),
            "https://g.example.com/v1/me?%24select=a+b"
        );
    }
}
