use std::sync::Arc;

use bytes::Bytes;
use reqwest::{Method, header::CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;

use super::{Drive, DriveItem, get_json, pager};
use crate::{
    Result,
    http::{Body, QueryOptions, RawRequest, Transport, encode_path, query::query_methods},
};

/// Largest payload sent with a single PUT; bigger files use an upload session.
const SIMPLE_UPLOAD_LIMIT: usize = 4 * 1024 * 1024;
/// Upload session chunk size (must be a multiple of 320 KiB).
const DEFAULT_CHUNK: usize = 320 * 1024 * 10;

/// A drive (document library or OneDrive).
#[derive(Debug, Clone)]
pub struct DriveRequest {
    t: Arc<Transport>,
    base: String,
    query: QueryOptions,
}

impl DriveRequest {
    pub(crate) fn new(t: Arc<Transport>, base: String) -> Self {
        Self {
            t,
            base,
            query: QueryOptions::new(),
        }
    }

    /// Fetches drive metadata.
    pub async fn get(&self) -> Result<Drive> {
        get_json(&self.t, &self.base, &self.query).await
    }

    /// The drive's root folder.
    pub fn root(&self) -> DriveItemRequest {
        DriveItemRequest::new(self.t.clone(), format!("{}/root", self.base))
    }

    /// An item by id.
    pub fn item(&self, id: &str) -> DriveItemRequest {
        DriveItemRequest::new(
            self.t.clone(),
            format!("{}/items/{}", self.base, encode_path(id)),
        )
    }

    /// An item by path relative to the drive root, e.g. `"Reports/q1.xlsx"`.
    pub fn path(&self, path: &str) -> DriveItemRequest {
        DriveItemRequest::new(
            self.t.clone(),
            format!(
                "{}/root:/{}:",
                self.base,
                encode_path(path.trim_matches('/'))
            ),
        )
    }
}
query_methods!(DriveRequest);

/// A file or folder in a drive.
#[derive(Debug, Clone)]
pub struct DriveItemRequest {
    t: Arc<Transport>,
    base: String,
    query: QueryOptions,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadSession {
    upload_url: String,
}

impl DriveItemRequest {
    pub(crate) fn new(t: Arc<Transport>, base: String) -> Self {
        Self {
            t,
            base,
            query: QueryOptions::new(),
        }
    }

    /// Fetches item metadata.
    pub async fn get(&self) -> Result<DriveItem> {
        get_json(&self.t, &self.base, &self.query).await
    }

    /// Lists the children of a folder.
    pub async fn children(&self) -> Result<Vec<DriveItem>> {
        pager(&self.t, &format!("{}/children", self.base), &self.query)
            .collect_all()
            .await
    }

    /// Downloads the file content.
    pub async fn download(&self) -> Result<Bytes> {
        Ok(self
            .t
            .send(&RawRequest::new(
                Method::GET,
                format!("{}/content", self.base),
            ))
            .await?
            .body)
    }

    /// Deletes the item.
    pub async fn delete(&self) -> Result<()> {
        self.t
            .send(&RawRequest::new(Method::DELETE, self.base.as_str()))
            .await?;
        Ok(())
    }

    /// Uploads (creating or replacing) the file at this path.
    ///
    /// Content up to 4 MiB is sent in one request; larger content uses a resumable session.
    pub async fn upload(&self, content: impl Into<Bytes>) -> Result<DriveItem> {
        let content = content.into();
        if content.len() <= SIMPLE_UPLOAD_LIMIT {
            let mut req = RawRequest::new(Method::PUT, format!("{}/content", self.base));
            req.headers
                .push((CONTENT_TYPE.to_string(), "application/octet-stream".into()));
            req.body = Body::Bytes(content);
            self.t.send(&req).await?.json()
        } else {
            self.upload_large(content, DEFAULT_CHUNK).await
        }
    }

    /// Uploads through a resumable session in chunks of `chunk_size` bytes.
    ///
    /// Graph requires chunk sizes to be a multiple of 320 KiB.
    pub async fn upload_large(&self, content: Bytes, chunk_size: usize) -> Result<DriveItem> {
        let chunk_size = chunk_size.max(1);
        let mut start_req =
            RawRequest::new(Method::POST, format!("{}/createUploadSession", self.base));
        start_req.body =
            Body::Json(json!({"item": {"@microsoft.graph.conflictBehavior": "replace"}}));
        let session: UploadSession = self.t.send(&start_req).await?.json()?;

        let total = content.len();
        let mut offset = 0;
        loop {
            let end = (offset + chunk_size).min(total);
            let mut req = RawRequest::new(Method::PUT, session.upload_url.as_str());
            req.anonymous = true;
            req.headers.push((
                "Content-Range".into(),
                format!("bytes {}-{}/{}", offset, end.saturating_sub(1), total),
            ));
            req.body = Body::Bytes(content.slice(offset..end));
            let resp = self.t.send(&req).await?;
            offset = end;
            if offset >= total {
                return resp.json();
            }
        }
    }
}
query_methods!(DriveItemRequest);

#[cfg(test)]
mod tests {
    use crate::graph::testutil::client;
    use bytes::Bytes;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_bytes, header, method, path},
    };

    #[tokio::test]
    async fn browse_and_download() {
        let (c, s) = client().await;
        Mock::given(path("/drives/d1/root/children"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"f","name":"Docs","folder":{}},{"id":"g","name":"a.txt","size":3,"file":{}}]})))
            .mount(&s)
            .await;
        Mock::given(path("/drives/d1/root:/Docs/a%20b.txt:/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes("abc"))
            .mount(&s)
            .await;
        let children = c.drive("d1").root().children().await.unwrap();
        assert!(children[0].is_folder() && !children[1].is_folder());
        assert_eq!(
            c.drive("d1")
                .path("/Docs/a b.txt")
                .download()
                .await
                .unwrap(),
            Bytes::from("abc")
        );
    }

    #[tokio::test]
    async fn small_upload_is_single_put() {
        let (c, s) = client().await;
        Mock::given(method("PUT"))
            .and(path("/me/drive/root:/a.txt:/content"))
            .and(body_bytes("hello"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(json!({"id":"n","name":"a.txt"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let item = c.me().drive().path("a.txt").upload("hello").await.unwrap();
        assert_eq!(item.id, "n");
    }

    #[tokio::test]
    async fn large_upload_uses_session_in_chunks_without_auth() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/drives/d/root:/big.bin:/createUploadSession"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"uploadUrl": format!("{}/upload/xyz", s.uri())})),
            )
            .mount(&s)
            .await;
        Mock::given(method("PUT"))
            .and(path("/upload/xyz"))
            .and(header("content-range", "bytes 0-3/10"))
            .respond_with(ResponseTemplate::new(202))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("PUT"))
            .and(path("/upload/xyz"))
            .and(header("content-range", "bytes 4-7/10"))
            .respond_with(ResponseTemplate::new(202))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("PUT"))
            .and(path("/upload/xyz"))
            .and(header("content-range", "bytes 8-9/10"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"done"})))
            .expect(1)
            .mount(&s)
            .await;
        let item = c
            .drive("d")
            .path("big.bin")
            .upload_large(Bytes::from_static(b"0123456789"), 4)
            .await
            .unwrap();
        assert_eq!(item.id, "done");
        let puts: Vec<_> = s
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|r| r.url.path() == "/upload/xyz")
            .collect();
        assert!(
            puts.iter()
                .all(|r| !r.headers.contains_key("authorization"))
        );
    }

    #[tokio::test]
    async fn delete_item() {
        let (c, s) = client().await;
        Mock::given(method("DELETE"))
            .and(path("/drives/d/items/i1"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        c.drive("d").item("i1").delete().await.unwrap();
    }
}
