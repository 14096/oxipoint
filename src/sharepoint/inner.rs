//! Shared state and request helpers for all SharePoint handles.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use reqwest::Method;
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    Result,
    http::{Body, QueryOptions, RawRequest, Response, Transport, encode_path},
};

pub(crate) const ACCEPT: &str = "application/json;odata=nometadata";

/// State shared by all handles of one client.
#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) t: Arc<Transport>,
    pub(crate) digest: Mutex<Option<(String, Instant)>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContextInfo {
    form_digest_value: String,
    #[serde(default)]
    form_digest_timeout_seconds: Option<u64>,
}

impl Inner {
    /// Returns a cached or freshly requested form digest.
    pub(crate) async fn digest(&self) -> Result<String> {
        let mut cache = self.digest.lock().await;
        if let Some((v, exp)) = cache.as_ref()
            && Instant::now() < *exp
        {
            return Ok(v.clone());
        }
        let resp = self
            .t
            .send(&RawRequest::new(Method::POST, "/contextinfo"))
            .await?;
        let doc: serde_json::Value = resp.json()?;
        let info: ContextInfo = match doc.pointer("/d/GetContextWebInformation") {
            Some(v) => serde_json::from_value(v.clone())?,
            None => serde_json::from_value(doc)?,
        };
        let ttl = info
            .form_digest_timeout_seconds
            .unwrap_or(1800)
            .saturating_sub(60);
        *cache = Some((
            info.form_digest_value.clone(),
            Instant::now() + Duration::from_secs(ttl),
        ));
        Ok(info.form_digest_value)
    }

    /// Sends a write request carrying the form digest.
    pub(crate) async fn write(&self, mut req: RawRequest) -> Result<Response> {
        let digest = self.digest().await?;
        req.headers.push(("X-RequestDigest".into(), digest));
        self.t.send(&req).await
    }

    /// POST with `X-HTTP-Method` tunnelling (MERGE / DELETE) and `IF-MATCH: *`.
    pub(crate) async fn tunnel(&self, verb: &str, path: String, body: Body) -> Result<Response> {
        self.tunnel_match(verb, path, body, "*").await
    }

    /// Like [`Inner::tunnel`] with an explicit `IF-MATCH` etag.
    pub(crate) async fn tunnel_match(
        &self,
        verb: &str,
        path: String,
        body: Body,
        etag: &str,
    ) -> Result<Response> {
        let mut req = RawRequest::new(Method::POST, path);
        req.headers.push(("X-HTTP-Method".into(), verb.into()));
        req.headers.push(("IF-MATCH".into(), etag.into()));
        req.body = body;
        self.write(req).await
    }
}

/// Escapes a value for use inside an OData string literal in a URL path.
pub(crate) fn odata_str(s: &str) -> String {
    encode_path(&s.replace('\'', "''"))
}

pub(crate) fn get_req(path: &str, q: &QueryOptions) -> RawRequest {
    let mut r = RawRequest::new(Method::GET, path);
    r.query = q.pairs().to_vec();
    r
}

pub(crate) fn json_body<F: Serialize>(f: &F) -> Result<Body> {
    Ok(Body::Json(serde_json::to_value(f)?))
}

/// POSTs with the form digest and deserializes the created entity.
pub(crate) async fn post<T: DeserializeOwned>(
    inner: &Inner,
    path: String,
    body: Body,
) -> Result<T> {
    let mut req = RawRequest::new(Method::POST, path);
    req.body = body;
    inner.write(req).await?.json()
}

/// POSTs with the form digest and discards the result.
pub(crate) async fn post_action(inner: &Inner, path: String, body: Body) -> Result<()> {
    let mut req = RawRequest::new(Method::POST, path);
    req.body = body;
    inner.write(req).await?;
    Ok(())
}

pub(crate) fn id_literal(id: &str) -> String {
    format!("'{}'", odata_str(id))
}

pub(crate) fn name_literal(s: &str) -> String {
    format!("'{}'", odata_str(s))
}

pub(crate) fn flex_opt<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Ok(match Option::<Value>::deserialize(d)? {
        Some(Value::String(s)) => Some(s),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

pub(crate) fn flex_string<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<String, D::Error> {
    Ok(flex_opt(d)?.unwrap_or_default())
}

/// Accepts a plain string or the `{ "StringValue": ... }` wrapper used for content type ids.
pub(crate) fn content_type_id<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<String, D::Error> {
    Ok(match Option::<Value>::deserialize(d)? {
        Some(Value::String(s)) => s,
        Some(Value::Object(o)) => o
            .get("StringValue")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    })
}
