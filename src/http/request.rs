use std::sync::Arc;

use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};

use tokio::io::AsyncWrite;

use super::{Pager, QueryOptions, StreamResponse, Transport};
use crate::{Error, Result};

/// Request payload.
#[derive(Debug, Clone, Default)]
pub enum Body {
    /// No body.
    #[default]
    None,
    /// A JSON document (sent as `application/json`).
    Json(serde_json::Value),
    /// Raw bytes (sent as `application/octet-stream` unless a content type header is set).
    Bytes(Bytes),
}

/// A transport-independent description of an HTTP request.
#[derive(Debug, Clone)]
#[must_use = "builders do nothing until a request is sent"]
pub struct RawRequest {
    /// HTTP method.
    pub method: Method,
    /// Path relative to the client base URL (starting with `/`) or an absolute URL.
    pub target: String,
    /// Query string parameters.
    pub query: Vec<(String, String)>,
    /// Extra headers.
    pub headers: HeaderMap,
    /// Body.
    pub body: Body,
    /// Send without a bearer token and without host checks (e.g. pre-authorized upload URLs).
    pub anonymous: bool,
}

impl RawRequest {
    /// Creates a request without query, headers or body.
    pub fn new(method: Method, target: impl Into<String>) -> Self {
        Self {
            method,
            target: target.into(),
            query: Vec::new(),
            headers: HeaderMap::new(),
            body: Body::None,
            anonymous: false,
        }
    }
}

/// A successful (2xx) HTTP response.
#[derive(Debug, Clone)]
pub struct Response {
    /// Status code.
    pub status: StatusCode,
    /// Response headers.
    pub headers: HeaderMap,
    /// Raw body.
    pub body: Bytes,
}

impl Response {
    /// Deserializes the body as JSON.
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    /// Returns the body as (lossy) UTF-8 text.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Parses a header value, mapping failure to [`Error::Config`].
pub(crate) fn header_value(value: &str) -> Result<HeaderValue> {
    HeaderValue::from_str(value)
        .map_err(|e| Error::Config(format!("invalid header value '{value}': {e}")))
}

/// Fluent builder for calling any endpoint through an authenticated client.
///
/// Obtained from `GraphClient::request` or `SharePointClient::request`.
#[derive(Debug)]
#[must_use = "builders do nothing until a request is sent"]
pub struct RequestBuilder {
    transport: Arc<Transport>,
    req: RawRequest,
    error: Option<Error>,
}

impl RequestBuilder {
    /// Creates a builder bound to a transport.
    pub fn new(transport: Arc<Transport>, method: Method, target: impl Into<String>) -> Self {
        Self {
            transport,
            req: RawRequest::new(method, target),
            error: None,
        }
    }

    /// Adds a query parameter.
    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.req.query.push((key.into(), value.into()));
        self
    }

    /// Adds all OData options.
    pub fn options(mut self, opts: &QueryOptions) -> Self {
        self.req.query.extend(opts.pairs().iter().cloned());
        self
    }

    /// Adds a request header.
    ///
    /// An invalid header name or value is reported when the request is sent.
    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        match (
            HeaderName::from_bytes(name.as_ref().as_bytes()),
            header_value(value.as_ref()),
        ) {
            (Ok(n), Ok(v)) => {
                self.req.headers.append(n, v);
            }
            (Err(e), _) => {
                self.error = Some(Error::Config(format!(
                    "invalid header name '{}': {e}",
                    name.as_ref()
                )));
            }
            (_, Err(e)) => self.error = Some(e),
        }
        self
    }

    /// Sets a JSON body.
    pub fn json<T: Serialize>(mut self, body: &T) -> Self {
        match serde_json::to_value(body) {
            Ok(v) => self.req.body = Body::Json(v),
            Err(e) => self.error = Some(e.into()),
        }
        self
    }

    /// Sets a raw byte body.
    pub fn body(mut self, bytes: impl Into<Bytes>) -> Self {
        self.req.body = Body::Bytes(bytes.into());
        self
    }

    /// Sends the request, returning the successful response.
    pub async fn send(self) -> Result<Response> {
        if let Some(e) = self.error {
            return Err(e);
        }
        self.transport.send(&self.req).await
    }

    /// Sends the request and returns the response with its body still unread.
    pub async fn send_stream(self) -> Result<StreamResponse> {
        if let Some(e) = self.error {
            return Err(e);
        }
        self.transport.send_stream(&self.req).await
    }

    /// Sends the request and copies the body into `writer`, returning the bytes written.
    ///
    /// The body is written as it arrives, so large files are never held in memory.
    ///
    /// ```no_run
    /// # async fn run(graph: oxipoint::graph::GraphClient) -> oxipoint::Result<()> {
    /// use reqwest::Method;
    ///
    /// // `tokio::fs` needs tokio's `fs` feature in your own crate.
    /// let mut file = tokio::fs::File::create("report.pdf").await?;
    /// let bytes = graph
    ///     .request(Method::GET, "/drives/{drive-id}/items/{item-id}/content")
    ///     .copy_to(&mut file)
    ///     .await?;
    /// println!("{bytes} bytes written");
    /// # Ok(()) }
    /// ```
    pub async fn copy_to<W: AsyncWrite + Unpin>(self, writer: &mut W) -> Result<u64> {
        self.send_stream().await?.copy_to(writer).await
    }

    /// Sends the request and deserializes the JSON body.
    pub async fn send_json<T: DeserializeOwned>(self) -> Result<T> {
        self.send().await?.json()
    }

    /// Treats the endpoint as a paged collection of `T`.
    pub fn pager<T: DeserializeOwned>(self) -> Result<Pager<T>> {
        if let Some(e) = self.error {
            return Err(e);
        }
        Ok(Pager::new(self.transport, self.req))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use reqwest::Method;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path, query_param},
    };

    use super::*;
    use crate::auth::StaticTokenCredential;
    use crate::http::TransportConfig;

    #[tokio::test]
    async fn streaming_mode_applies_query_headers_and_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/x"))
            .and(query_param("a", "1"))
            .and(header("x-test", "yes"))
            .respond_with(ResponseTemplate::new(200).set_body_string("streamed"))
            .mount(&server)
            .await;
        let t = Arc::new(Transport::new(
            Arc::new(StaticTokenCredential::new("tok")),
            "scope",
            server.uri(),
            TransportConfig::default(),
        ));
        let mut out = Vec::new();
        let n = RequestBuilder::new(t.clone(), Method::POST, "/x")
            .query("a", "1")
            .header("x-test", "yes")
            .body("payload")
            .copy_to(&mut out)
            .await
            .unwrap();
        assert_eq!((n, out.as_slice()), (8, &b"streamed"[..]));

        let s = RequestBuilder::new(t, Method::POST, "/x")
            .query("a", "1")
            .header("x-test", "yes")
            .send_stream()
            .await
            .unwrap();
        assert_eq!(s.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn streaming_mode_reports_service_errors() {
        let server = MockServer::start().await;
        Mock::given(path("/e"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        let t = Arc::new(Transport::new(
            Arc::new(StaticTokenCredential::new("tok")),
            "scope",
            server.uri(),
            TransportConfig::default(),
        ));
        let e = RequestBuilder::new(t, Method::GET, "/e")
            .send_stream()
            .await
            .unwrap_err();
        assert_eq!(e.status(), Some(StatusCode::FORBIDDEN));
    }
}
