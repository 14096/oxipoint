use std::sync::Arc;

use bytes::Bytes;
use reqwest::Method;
use serde::{Serialize, de::Error as _};

use super::{Inner, inner::ACCEPT};
use crate::{
    Error, Result,
    http::{BatchResult, Body, RawRequest, header_map},
};

const CRLF: &str = "\r\n";

/// One sub-request of a [`SharePointBatch`].
#[derive(Debug, Clone)]
pub struct SharePointBatchRequest {
    method: Method,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

impl SharePointBatchRequest {
    /// A sub-request for `path`, relative to `<site>/_api` (for example `/web/title`).
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Adds a header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Sets a JSON body.
    pub fn json<T: Serialize>(mut self, body: &T) -> Result<Self> {
        self.body = Some(serde_json::to_string(body)?);
        Ok(self)
    }

    fn is_write(&self) -> bool {
        self.method != Method::GET
    }
}

#[derive(Debug)]
enum Group {
    Single(usize),
    Changeset(Vec<usize>),
}

/// Several SharePoint calls sent in one `multipart/mixed` request. Get one from
/// `SharePointClient::batch`.
#[derive(Debug)]
pub struct SharePointBatch {
    inner: Arc<Inner>,
    requests: Vec<SharePointBatchRequest>,
    groups: Vec<Group>,
}

impl SharePointBatch {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            inner,
            requests: Vec::new(),
            groups: Vec::new(),
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

    fn check(request: &SharePointBatchRequest) -> Result<()> {
        if request.path.starts_with("http://") || request.path.starts_with("https://") {
            return Err(Error::Config(
                "batch request paths must be relative to the site's _api".into(),
            ));
        }
        Ok(())
    }

    /// Adds a sub-request and returns its position in the result list.
    pub fn add(&mut self, request: SharePointBatchRequest) -> Result<usize> {
        Self::check(&request)?;
        let idx = self.requests.len();
        self.requests.push(request);
        self.groups.push(Group::Single(idx));
        Ok(idx)
    }

    /// Adds write sub-requests that the service applies all or nothing, and returns their
    /// positions in the result list. When one fails, every request of the changeset reports that
    /// failure. Reads cannot be part of a changeset.
    pub fn add_changeset(
        &mut self,
        requests: impl IntoIterator<Item = SharePointBatchRequest>,
    ) -> Result<Vec<usize>> {
        let requests: Vec<_> = requests.into_iter().collect();
        for r in &requests {
            Self::check(r)?;
            if !r.is_write() {
                return Err(Error::Config(
                    "only writes can be part of a changeset".into(),
                ));
            }
        }
        let start = self.requests.len();
        let idxs: Vec<usize> = (start..start + requests.len()).collect();
        self.requests.extend(requests);
        if !idxs.is_empty() {
            self.groups.push(Group::Changeset(idxs.clone()));
        }
        Ok(idxs)
    }

    fn write_part(&self, out: &mut String, r: &SharePointBatchRequest, digest: Option<&str>) {
        out.push_str("Content-Type: application/http");
        out.push_str(CRLF);
        out.push_str("Content-Transfer-Encoding: binary");
        out.push_str(CRLF);
        out.push_str(CRLF);
        let path = if r.path.starts_with('/') {
            r.path.clone()
        } else {
            format!("/{}", r.path)
        };
        out.push_str(&format!(
            "{} {}{} HTTP/1.1{CRLF}",
            r.method.as_str(),
            self.inner.t.base_url(),
            path
        ));
        let has = |name: &str| r.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));
        if !has("accept") {
            out.push_str(&format!("Accept: {ACCEPT}{CRLF}"));
        }
        if r.body.is_some() && !has("content-type") {
            out.push_str(&format!("Content-Type: {ACCEPT}{CRLF}"));
        }
        if let Some(d) = digest
            && r.is_write()
            && !has("x-requestdigest")
        {
            out.push_str(&format!("X-RequestDigest: {d}{CRLF}"));
        }
        for (k, v) in &r.headers {
            out.push_str(&format!("{k}: {v}{CRLF}"));
        }
        out.push_str(CRLF);
        if let Some(b) = &r.body {
            out.push_str(b);
            out.push_str(CRLF);
        }
    }

    fn build_body(&self, batch_id: &str, digest: Option<&str>) -> String {
        let mut out = String::new();
        let mut cs_no = 0;
        for g in &self.groups {
            out.push_str(&format!("--batch_{batch_id}{CRLF}"));
            match g {
                Group::Single(i) => self.write_part(&mut out, &self.requests[*i], digest),
                Group::Changeset(idxs) => {
                    cs_no += 1;
                    let cs = format!("changeset_{batch_id}_{cs_no}");
                    out.push_str(&format!(
                        "Content-Type: multipart/mixed; boundary=\"{cs}\"{CRLF}{CRLF}"
                    ));
                    for i in idxs {
                        out.push_str(&format!("--{cs}{CRLF}"));
                        self.write_part(&mut out, &self.requests[*i], digest);
                    }
                    out.push_str(&format!("--{cs}--{CRLF}"));
                }
            }
        }
        out.push_str(&format!("--batch_{batch_id}--{CRLF}"));
        out
    }

    /// Sends the batch and returns one result per sub-request, in the order they were added.
    ///
    /// A failing sub-request does not fail this call; read its [`BatchResult`]. Writes carry the
    /// form digest.
    pub async fn send(self) -> Result<Vec<BatchResult>> {
        if self.requests.is_empty() {
            return Ok(Vec::new());
        }
        let has_write = self.requests.iter().any(SharePointBatchRequest::is_write);
        let digest = if has_write {
            Some(self.inner.digest().await?)
        } else {
            None
        };
        let batch_id = format!("{:032x}", rand::random::<u128>());
        let mut req = RawRequest::new(Method::POST, "/$batch");
        req.headers.push((
            "Content-Type".into(),
            format!("multipart/mixed; boundary=batch_{batch_id}"),
        ));
        if let Some(d) = &digest {
            req.headers.push(("X-RequestDigest".into(), d.clone()));
        }
        req.body = Body::Bytes(Bytes::from(self.build_body(&batch_id, digest.as_deref())));
        let resp = self.inner.t.send(&req).await?;
        let content_type = resp
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        let boundary = boundary_of(&content_type).ok_or_else(|| {
            serde_json::Error::custom("the batch response is not multipart (no boundary)")
        })?;
        let parts = parse_multipart(&resp.text(), &boundary)?;
        self.assign(parts)
    }

    fn assign(&self, parts: Vec<Part>) -> Result<Vec<BatchResult>> {
        if parts.len() != self.groups.len() {
            return Err(serde_json::Error::custom(format!(
                "the batch response has {} parts, expected {}",
                parts.len(),
                self.groups.len()
            ))
            .into());
        }
        let mut out: Vec<Option<BatchResult>> = vec![None; self.requests.len()];
        for (group, part) in self.groups.iter().zip(parts) {
            match (group, part) {
                (Group::Single(i), Part::Single(r)) => out[*i] = Some(r),
                (Group::Single(i), Part::Group(mut rs)) if !rs.is_empty() => {
                    out[*i] = Some(rs.swap_remove(0));
                }
                (Group::Changeset(idxs), part) => {
                    let results = match part {
                        Part::Single(r) => vec![r],
                        Part::Group(rs) => rs,
                    };
                    if let Some(failed) = results.iter().find(|r| !r.is_success()) {
                        for i in idxs {
                            out[*i] = Some(failed.clone());
                        }
                    } else if results.len() == idxs.len() {
                        for (i, r) in idxs.iter().zip(results) {
                            out[*i] = Some(r);
                        }
                    } else {
                        return Err(serde_json::Error::custom(format!(
                            "a changeset of {} requests got {} results",
                            idxs.len(),
                            results.len()
                        ))
                        .into());
                    }
                }
                _ => {
                    return Err(
                        serde_json::Error::custom("an empty part in the batch response").into(),
                    );
                }
            }
        }
        out.into_iter()
            .map(|r| {
                r.ok_or_else(|| {
                    serde_json::Error::custom("the batch response is missing a result").into()
                })
            })
            .collect()
    }
}

#[derive(Debug)]
enum Part {
    Single(BatchResult),
    Group(Vec<BatchResult>),
}

fn boundary_of(content_type: &str) -> Option<String> {
    let (_, rest) = content_type.split_once("boundary=")?;
    let b = rest.split(';').next()?.trim().trim_matches('"');
    (!b.is_empty()).then(|| b.to_owned())
}

/// Splits a `multipart/mixed` body into its parts. Nested multipart parts (changesets) become a
/// [`Part::Group`].
fn parse_multipart(text: &str, boundary: &str) -> Result<Vec<Part>> {
    let text = text.replace("\r\n", "\n");
    let delimiter = format!("--{boundary}");
    let mut parts = Vec::new();
    for segment in text.split(&delimiter).skip(1) {
        if segment.starts_with("--") {
            break;
        }
        let segment = segment.strip_prefix('\n').unwrap_or(segment);
        let (head, content) = segment
            .split_once("\n\n")
            .ok_or_else(|| serde_json::Error::custom("a batch part has no headers"))?;
        let nested = head
            .lines()
            .find(|l| {
                l.to_ascii_lowercase().starts_with("content-type:") && l.contains("multipart/")
            })
            .and_then(boundary_of);
        if let Some(inner) = nested {
            // `content` starts after the blank line; re-add the delimiter lookup context.
            let mut results = Vec::new();
            for p in parse_multipart(content, &inner)? {
                match p {
                    Part::Single(r) => results.push(r),
                    Part::Group(rs) => results.extend(rs),
                }
            }
            parts.push(Part::Group(results));
        } else {
            parts.push(Part::Single(parse_http_response(content)?));
        }
    }
    Ok(parts)
}

fn parse_http_response(content: &str) -> Result<BatchResult> {
    let (head, body) = content
        .split_once("\n\n")
        .unwrap_or((content.trim_end_matches('\n'), ""));
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| serde_json::Error::custom("a batch part has no HTTP status line"))?;
    let headers = header_map(lines.filter_map(|l| l.split_once(':')));
    let body = body.trim_end_matches('\n');
    Ok(BatchResult::new(
        status,
        headers,
        Bytes::from(body.to_owned()),
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{header, method, path},
    };

    use super::*;
    use crate::sharepoint::testutil::{client, client_retrying, mount_digest};

    const MIME: &str = "multipart/mixed; boundary=batchresponse_1";

    fn part(status: &str, headers: &str, body: &str) -> String {
        format!(
            "Content-Type: application/http\r\nContent-Transfer-Encoding: binary\r\n\r\nHTTP/1.1 {status}\r\n{headers}\r\n{body}\r\n"
        )
    }

    fn batch_response(parts: &[String]) -> String {
        let mut out = String::new();
        for p in parts {
            out.push_str("--batchresponse_1\r\n");
            out.push_str(p);
        }
        out.push_str("--batchresponse_1--\r\n");
        out
    }

    fn changeset_response(parts: &[String]) -> String {
        let mut out =
            String::from("Content-Type: multipart/mixed; boundary=changesetresponse_1\r\n\r\n");
        for p in parts {
            out.push_str("--changesetresponse_1\r\n");
            out.push_str(p);
        }
        out.push_str("--changesetresponse_1--\r\n");
        out
    }

    fn json_hdr() -> &'static str {
        "CONTENT-TYPE: application/json;odata=nometadata\r\n"
    }

    async fn mount_batch(s: &wiremock::MockServer, body: String) {
        Mock::given(method("POST"))
            .and(path("/_api/$batch"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, MIME))
            .expect(1)
            .mount(s)
            .await;
    }

    fn get(p: &str) -> SharePointBatchRequest {
        SharePointBatchRequest::new(Method::GET, p)
    }

    fn post(p: &str, body: Value) -> SharePointBatchRequest {
        SharePointBatchRequest::new(Method::POST, p)
            .json(&body)
            .unwrap()
    }

    #[tokio::test]
    async fn read_batch_is_one_multipart_post_without_a_digest() {
        let (c, s) = client().await;
        mount_digest(&s, 0).await;
        mount_batch(
            &s,
            batch_response(&[
                part("200 OK", json_hdr(), r#"{"value":"Team"}"#),
                part("200 OK", json_hdr(), r#"{"value":"en-US"}"#),
                part("200 OK", json_hdr(), r#"{"Id":1}"#),
            ]),
        )
        .await;
        let mut b = c.batch();
        b.add(get("/web/title")).unwrap();
        b.add(get("/web/language")).unwrap();
        b.add(get("/web/currentuser")).unwrap();
        let r = b.send().await.unwrap();
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].json::<Value>().unwrap()["value"], "Team");
        assert_eq!(r[2].json::<Value>().unwrap()["Id"], 1);

        let reqs = s.received_requests().await.unwrap();
        let sent = &reqs[0];
        let ct = sent.headers.get("content-type").unwrap().to_str().unwrap();
        assert!(ct.starts_with("multipart/mixed; boundary=batch_"));
        let body = String::from_utf8(sent.body.clone()).unwrap();
        for p in ["/web/title", "/web/language", "/web/currentuser"] {
            assert!(
                body.contains(&format!("GET {}/_api{p} HTTP/1.1\r\n", s.uri())),
                "{body}"
            );
        }
        assert!(!body.contains("X-RequestDigest"));
        assert!(body.trim_end().ends_with("--"));
    }

    #[tokio::test]
    async fn writes_in_a_batch_carry_the_digest() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/$batch"))
            .and(header("x-requestdigest", "DIGEST"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                batch_response(&[part("201 Created", json_hdr(), r#"{"Id":5}"#)]),
                MIME,
            ))
            .expect(1)
            .mount(&s)
            .await;
        let mut b = c.batch();
        b.add(post(
            "/web/lists/getbytitle('T')/items",
            json!({"Title":"x"}),
        ))
        .unwrap();
        let r = b.send().await.unwrap();
        assert_eq!(r[0].status, 201);
        let reqs = s.received_requests().await.unwrap();
        let body = String::from_utf8(reqs.last().unwrap().body.clone()).unwrap();
        assert!(body.contains("X-RequestDigest: DIGEST\r\n"));
        assert!(body.contains(r#"{"Title":"x"}"#));
        assert!(body.contains("Content-Type: application/json;odata=nometadata"));
    }

    #[tokio::test]
    async fn mixed_outcomes_do_not_fail_the_batch() {
        let (c, s) = client().await;
        mount_batch(
            &s,
            batch_response(&[
                part("200 OK", json_hdr(), r#"{"value":"ok"}"#),
                part(
                    "404 Not Found",
                    json_hdr(),
                    r#"{"odata.error":{"code":"-2147024809, System.ArgumentException","message":{"lang":"en-US","value":"List does not exist"}}}"#,
                ),
            ]),
        )
        .await;
        let mut b = c.batch();
        b.add(get("/web/title")).unwrap();
        b.add(get("/web/lists/getbytitle('Nope')")).unwrap();
        let r = b.send().await.unwrap();
        assert!(r[0].is_success());
        let e = r[1].error().unwrap();
        assert_eq!(e.status(), Some(404));
        assert!(e.to_string().contains("List does not exist"));
    }

    #[tokio::test]
    async fn an_unexpected_number_of_parts_is_a_deserialization_error() {
        let (c, s) = client().await;
        mount_batch(
            &s,
            batch_response(&[
                part("200 OK", json_hdr(), r#"{}"#),
                part("200 OK", json_hdr(), r#"{}"#),
            ]),
        )
        .await;
        let mut b = c.batch();
        for _ in 0..3 {
            b.add(get("/web/title")).unwrap();
        }
        assert!(matches!(b.send().await, Err(Error::Deserialize(_))));
    }

    #[tokio::test]
    async fn a_non_multipart_answer_is_a_deserialization_error() {
        let (c, s) = client().await;
        Mock::given(path("/_api/$batch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":1})))
            .mount(&s)
            .await;
        let mut b = c.batch();
        b.add(get("/web/title")).unwrap();
        assert!(matches!(b.send().await, Err(Error::Deserialize(_))));
    }

    #[tokio::test]
    async fn a_successful_changeset_returns_one_result_per_write() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mount_batch(
            &s,
            batch_response(&[
                part("200 OK", json_hdr(), r#"{"value":"Team"}"#),
                changeset_response(&[
                    part("201 Created", json_hdr(), r#"{"Id":1}"#),
                    part("201 Created", json_hdr(), r#"{"Id":2}"#),
                ]),
            ]),
        )
        .await;
        let mut b = c.batch();
        b.add(get("/web/title")).unwrap();
        let idx = b
            .add_changeset([
                post("/web/lists/getbytitle('T')/items", json!({"Title":"a"})),
                post("/web/lists/getbytitle('T')/items", json!({"Title":"b"})),
            ])
            .unwrap();
        assert_eq!(idx, [1, 2]);
        let r = b.send().await.unwrap();
        assert_eq!(r.len(), 3);
        assert_eq!(r[1].json::<Value>().unwrap()["Id"], 1);
        assert_eq!(r[2].json::<Value>().unwrap()["Id"], 2);
        let reqs = s.received_requests().await.unwrap();
        let body = String::from_utf8(reqs.last().unwrap().body.clone()).unwrap();
        assert!(body.contains("boundary=\"changeset_"));
        assert_eq!(body.matches("POST ").count(), 2);
    }

    #[tokio::test]
    async fn a_failing_changeset_fails_every_request_in_it() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mount_batch(
            &s,
            batch_response(&[changeset_response(&[part(
                "400 Bad Request",
                json_hdr(),
                r#"{"odata.error":{"code":"-1, Bad","message":{"value":"second failed"}}}"#,
            )])]),
        )
        .await;
        let mut b = c.batch();
        b.add_changeset([
            post("/web/lists/getbytitle('T')/items", json!({"Title":"a"})),
            post("/web/lists/getbytitle('T')/items", json!({"Title":"b"})),
        ])
        .unwrap();
        let r = b.send().await.unwrap();
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|x| x.status == 400));
        assert!(r[0].error().unwrap().to_string().contains("second failed"));
    }

    #[tokio::test]
    async fn reads_cannot_join_a_changeset_and_absolute_paths_are_refused() {
        let (c, s) = client().await;
        let mut b = c.batch();
        assert!(matches!(
            b.add_changeset([get("/web/title")]),
            Err(Error::Config(_))
        ));
        assert!(matches!(
            b.add(get("https://evil.example.org/_api/web")),
            Err(Error::Config(_))
        ));
        assert!(b.is_empty());
        assert!(b.send().await.unwrap().is_empty());
        assert!(s.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_throttled_batch_call_is_retried() {
        let (c, s) = client_retrying().await;
        Mock::given(path("/_api/$batch"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .up_to_n_times(1)
            .mount(&s)
            .await;
        Mock::given(path("/_api/$batch"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                batch_response(&[part("200 OK", json_hdr(), r#"{"value":"Team"}"#)]),
                MIME,
            ))
            .mount(&s)
            .await;
        let mut b = c.batch();
        b.add(get("/web/title")).unwrap();
        let r = b.send().await.unwrap();
        assert_eq!(r[0].json::<Value>().unwrap()["value"], "Team");
        assert_eq!(s.received_requests().await.unwrap().len(), 2);
    }

    #[test]
    fn parses_headers_and_bodies_of_a_golden_response() {
        let text = "--b\r\nContent-Type: application/http\r\nContent-Transfer-Encoding: binary\r\n\r\nHTTP/1.1 200 OK\r\nCONTENT-TYPE: application/json;odata=nometadata\r\nETag: \"1\"\r\n\r\n{\"a\":\"line1\\nline2\"}\r\n--b\r\nContent-Type: application/http\r\nContent-Transfer-Encoding: binary\r\n\r\nHTTP/1.1 204 No Content\r\n\r\n\r\n--b--\r\n";
        let parts = parse_multipart(text, "b").unwrap();
        assert_eq!(parts.len(), 2);
        let Part::Single(first) = &parts[0] else {
            panic!()
        };
        assert_eq!(first.status, 200);
        assert_eq!(first.headers.get("etag").unwrap(), "\"1\"");
        assert_eq!(first.json::<Value>().unwrap()["a"], "line1\nline2");
        let Part::Single(second) = &parts[1] else {
            panic!()
        };
        assert_eq!((second.status, second.body.len()), (204, 0));
    }
}
