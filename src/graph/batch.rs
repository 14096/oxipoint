use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use reqwest::Method;
use serde::{Serialize, de::Error as _};
use serde_json::{Value, json};

use crate::{
    Error, Result,
    http::{BatchResult, Body, RawRequest, Transport, header_map},
};

/// Most sub-requests Graph accepts in one batch.
pub const MAX_BATCH_REQUESTS: usize = 20;

/// One sub-request of a [`GraphBatch`].
#[derive(Debug, Clone)]
pub struct GraphBatchRequest {
    method: Method,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<Value>,
    depends_on: Vec<String>,
}

impl GraphBatchRequest {
    /// A sub-request for `path`, relative to the Graph base URL (for example `/me`).
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: Vec::new(),
            body: None,
            depends_on: Vec::new(),
        }
    }

    /// Adds a header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sets a JSON body.
    pub fn json<T: Serialize>(mut self, body: &T) -> Result<Self> {
        self.body = Some(serde_json::to_value(body)?);
        Ok(self)
    }

    /// Runs this request only after the sub-request with this id (returned by
    /// [`GraphBatch::add`]) succeeded.
    pub fn depends_on(mut self, id: impl Into<String>) -> Self {
        self.depends_on.push(id.into());
        self
    }
}

/// Several Graph calls sent in one HTTP request. Get one from `GraphClient::batch`.
#[derive(Debug)]
pub struct GraphBatch {
    t: Arc<Transport>,
    requests: Vec<Value>,
    ids: Vec<String>,
}

impl GraphBatch {
    pub(crate) fn new(t: Arc<Transport>) -> Self {
        Self {
            t,
            requests: Vec::new(),
            ids: Vec::new(),
        }
    }

    /// Number of sub-requests added so far.
    pub fn len(&self) -> usize {
        self.requests.len()
    }

    /// Whether no sub-request was added.
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    /// Adds a sub-request and returns its id.
    ///
    /// Fails with [`Error::Config`] when the batch is full ([`MAX_BATCH_REQUESTS`]), the path is
    /// an absolute URL, or a dependency is not an earlier sub-request of this batch.
    pub fn add(&mut self, request: GraphBatchRequest) -> Result<String> {
        if self.requests.len() >= MAX_BATCH_REQUESTS {
            return Err(Error::Config(format!(
                "a Graph batch holds at most {MAX_BATCH_REQUESTS} requests"
            )));
        }
        if request.path.starts_with("http://") || request.path.starts_with("https://") {
            return Err(Error::Config(
                "batch request paths must be relative to the Graph base URL".into(),
            ));
        }
        if let Some(unknown) = request.depends_on.iter().find(|d| !self.ids.contains(d)) {
            return Err(Error::Config(format!(
                "batch request depends on unknown id '{unknown}'"
            )));
        }
        let id = (self.requests.len() + 1).to_string();
        let path = if request.path.starts_with('/') {
            request.path
        } else {
            format!("/{}", request.path)
        };
        let mut item = json!({ "id": id, "method": request.method.as_str(), "url": path });
        let mut headers = serde_json::Map::new();
        for (k, v) in request.headers {
            headers.insert(k, Value::String(v));
        }
        if let Some(body) = request.body {
            if !headers
                .keys()
                .any(|k| k.eq_ignore_ascii_case("content-type"))
            {
                headers.insert("Content-Type".into(), "application/json".into());
            }
            item["body"] = body;
        }
        if !headers.is_empty() {
            item["headers"] = Value::Object(headers);
        }
        if !request.depends_on.is_empty() {
            item["dependsOn"] = json!(request.depends_on);
        }
        self.ids.push(id.clone());
        self.requests.push(item);
        Ok(id)
    }

    /// Sends the batch and returns one result per sub-request, in the order they were added.
    ///
    /// A failing sub-request does not fail this call; read its [`BatchResult`]. Throttled
    /// sub-requests (status 429) are reported, not retried: resend them in a new batch after
    /// [`BatchResult::retry_after`].
    pub async fn send(self) -> Result<Vec<BatchResult>> {
        if self.requests.is_empty() {
            return Ok(Vec::new());
        }
        let mut req = RawRequest::new(Method::POST, "/$batch");
        req.body = Body::Json(json!({ "requests": self.requests }));
        let doc: Value = self.t.send(&req).await?.json()?;
        let mut by_id: HashMap<String, BatchResult> = HashMap::new();
        for r in doc
            .get("responses")
            .and_then(Value::as_array)
            .ok_or_else(|| serde_json::Error::custom("batch response has no 'responses'"))?
        {
            let id = r
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| serde_json::Error::custom("batch response without an id"))?;
            let status = r.get("status").and_then(Value::as_u64).unwrap_or(0) as u16;
            let headers = r
                .get("headers")
                .and_then(Value::as_object)
                .map(|m| {
                    header_map(
                        m.iter()
                            .filter_map(|(k, v)| Some((k.as_str(), v.as_str()?))),
                    )
                })
                .unwrap_or_default();
            let body = match r.get("body") {
                Some(b) if !b.is_null() => Bytes::from(serde_json::to_vec(b)?),
                _ => Bytes::new(),
            };
            by_id.insert(id.to_owned(), BatchResult::new(status, headers, body));
        }
        self.ids
            .iter()
            .map(|id| {
                by_id.remove(id).ok_or_else(|| {
                    serde_json::Error::custom(format!("batch response is missing id '{id}'")).into()
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, method, path},
    };

    use super::*;
    use crate::graph::testutil::client;

    fn get(p: &str) -> GraphBatchRequest {
        GraphBatchRequest::new(Method::GET, p)
    }

    #[tokio::test]
    async fn ids_are_distinct_and_the_limit_is_enforced() {
        let (c, s) = client().await;
        let mut b = c.batch();
        let a = b.add(get("/me")).unwrap();
        let bb = b.add(get("/users")).unwrap();
        assert_ne!(a, bb);
        for _ in 2..MAX_BATCH_REQUESTS {
            b.add(get("/me")).unwrap();
        }
        assert_eq!(b.len(), 20);
        let e = b.add(get("/me")).unwrap_err();
        assert!(matches!(e, Error::Config(_)));
        assert!(s.received_requests().await.unwrap().is_empty());
    }

    #[test]
    fn absolute_paths_and_unknown_dependencies_are_refused() {
        let t = Arc::new(Transport::new(
            Arc::new(crate::auth::StaticTokenCredential::new("t")),
            "s",
            "https://graph.example.com/v1.0",
            Default::default(),
        ));
        let mut b = GraphBatch::new(t);
        assert!(matches!(
            b.add(get("https://evil.example.org/x")),
            Err(Error::Config(_))
        ));
        assert!(matches!(
            b.add(get("/me").depends_on("9")),
            Err(Error::Config(_))
        ));
        assert!(b.is_empty());
    }

    #[tokio::test]
    async fn sends_one_document_with_dependencies_and_bodies() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/$batch"))
            .and(body_json(json!({"requests":[
                {"id":"1","method":"POST","url":"/sites/s/lists",
                 "headers":{"Content-Type":"application/json"},"body":{"displayName":"L"}},
                {"id":"2","method":"GET","url":"/me","dependsOn":["1"]}
            ]})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"responses":[
                    {"id":"1","status":201,"body":{"id":"l1"}},
                    {"id":"2","status":200,"body":{"displayName":"Me"}}
                ]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let mut b = c.batch();
        let first = b
            .add(
                GraphBatchRequest::new(Method::POST, "sites/s/lists")
                    .json(&json!({"displayName":"L"}))
                    .unwrap(),
            )
            .unwrap();
        b.add(get("/me").depends_on(first)).unwrap();
        let r = b.send().await.unwrap();
        assert_eq!(r[0].json::<Value>().unwrap()["id"], "l1");
        assert_eq!(r[1].json::<Value>().unwrap()["displayName"], "Me");
    }

    #[tokio::test]
    async fn mixed_outcomes_do_not_fail_the_batch_and_order_is_restored() {
        let (c, s) = client().await;
        Mock::given(path("/$batch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"responses":[
                {"id":"2","status":404,"body":{"error":{"code":"itemNotFound","message":"gone"}}},
                {"id":"1","status":200,"headers":{"Content-Type":"application/json"},"body":{"ok":true}}
            ]})))
            .mount(&s)
            .await;
        let mut b = c.batch();
        b.add(get("/a")).unwrap();
        b.add(get("/b")).unwrap();
        let r = b.send().await.unwrap();
        assert!(r[0].is_success());
        assert_eq!(r[0].json::<Value>().unwrap()["ok"], true);
        assert_eq!(r[1].status, 404);
        let e = r[1].json::<Value>().unwrap_err();
        assert_eq!(e.status(), Some(404));
        assert!(
            matches!(r[1].error(), Some(Error::Http { code: Some(c), .. }) if c == "itemNotFound")
        );
    }

    #[tokio::test]
    async fn a_failing_batch_call_is_an_http_error() {
        let (c, s) = client().await;
        Mock::given(path("/$batch"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(json!({"error":{"code":"BadRequest","message":"bad batch"}})),
            )
            .mount(&s)
            .await;
        let mut b = c.batch();
        b.add(get("/me")).unwrap();
        assert_eq!(b.send().await.unwrap_err().status(), Some(400));
    }

    #[tokio::test]
    async fn throttled_sub_requests_are_reported_not_retried() {
        let (c, s) = client().await;
        Mock::given(path("/$batch"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"responses":[
                    {"id":"1","status":429,"headers":{"Retry-After":"5"},
                     "body":{"error":{"code":"activityLimitReached","message":"slow down"}}},
                    {"id":"2","status":200,"body":{"ok":1}}
                ]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let mut b = c.batch();
        b.add(get("/a")).unwrap();
        b.add(get("/b")).unwrap();
        let r = b.send().await.unwrap();
        assert_eq!(r[0].status, 429);
        assert_eq!(r[0].retry_after(), Some(Duration::from_secs(5)));
        assert!(r[1].is_success());
    }

    #[tokio::test]
    async fn a_missing_response_is_a_deserialization_error() {
        let (c, s) = client().await;
        Mock::given(path("/$batch"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"responses":[{"id":"1","status":200}]})),
            )
            .mount(&s)
            .await;
        let mut b = c.batch();
        b.add(get("/a")).unwrap();
        b.add(get("/b")).unwrap();
        assert!(matches!(b.send().await, Err(Error::Deserialize(_))));
    }

    #[tokio::test]
    async fn an_empty_batch_sends_nothing() {
        let (c, s) = client().await;
        assert!(c.batch().send().await.unwrap().is_empty());
        assert!(s.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_throttled_batch_call_is_retried() {
        let s = wiremock::MockServer::start().await;
        let c = crate::graph::GraphClient::builder(crate::auth::StaticTokenCredential::new("tok"))
            .base_url(s.uri())
            .max_retries(1)
            .build();
        Mock::given(path("/$batch"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .up_to_n_times(1)
            .mount(&s)
            .await;
        Mock::given(path("/$batch"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"responses":[{"id":"1","status":200,"body":{"ok":true}}]}),
                ),
            )
            .mount(&s)
            .await;
        let mut b = c.batch();
        b.add(get("/me")).unwrap();
        let r = b.send().await.unwrap();
        assert!(r[0].is_success());
        assert_eq!(s.received_requests().await.unwrap().len(), 2);
    }
}
