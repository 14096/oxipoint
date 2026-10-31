mod copy;
mod models;
mod sharing;

pub use copy::{CopyOperation, CopyProgress, CopyStatus};
pub use models::{
    ConflictBehavior, Drive, DriveDeltaPage, DriveItem, DriveItemUpdate, DriveItemVersion,
};
pub(crate) use sharing::encode_share_url;
pub use sharing::{
    Invite, ItemPermissionRequest, ItemPermissionsRequest, LinkOptions, LinkScope, LinkType,
};

use std::sync::Arc;

use bytes::Bytes;
use reqwest::{Method, header::CONTENT_TYPE};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::AsyncWrite;
use url::Url;

use crate::graph::*;
use crate::{
    Error, Result,
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

    /// Reads the items changed since `token` (everything when `None`), following paging links.
    ///
    /// The returned [`DriveDeltaPage::next_token`] resumes the query on the next call. An
    /// expired token is reported by the service as an HTTP error.
    pub async fn delta(&self, token: Option<&str>) -> Result<DriveDeltaPage> {
        let mut first = request_with(&format!("{}/root/delta", self.base), &self.query);
        if let Some(t) = token {
            first.query.push(("token".into(), t.to_owned()));
        }
        let mut out = DriveDeltaPage::default();
        let mut next = Some(first);
        while let Some(req) = next.take() {
            let doc: Value = self.t.send(&req).await?.json()?;
            if let Some(v) = doc.get("value") {
                out.items
                    .extend(serde_json::from_value::<Vec<DriveItem>>(v.clone())?);
            }
            if let Some(link) = doc.get("@odata.nextLink").and_then(Value::as_str) {
                next = Some(RawRequest::new(Method::GET, link));
            } else if let Some(link) = doc.get("@odata.deltaLink").and_then(Value::as_str) {
                out.next_token = Url::parse(link).ok().and_then(|u| {
                    u.query_pairs()
                        .find(|(k, _)| k == "token")
                        .map(|(_, v)| v.into_owned())
                });
            }
        }
        Ok(out)
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

    /// Downloads the file content into `writer` as it arrives, returning the bytes written.
    ///
    /// Nothing is written when the service answers with an error status.
    pub async fn download_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<u64> {
        self.t
            .send_stream(&RawRequest::new(
                Method::GET,
                format!("{}/content", self.base),
            ))
            .await?
            .copy_to(writer)
            .await
    }

    /// Creates a child folder. Without `conflict` the service default (fail) applies.
    pub async fn create_folder(
        &self,
        name: &str,
        conflict: Option<ConflictBehavior>,
    ) -> Result<DriveItem> {
        let mut body = json!({ "name": name, "folder": {} });
        if let Some(c) = conflict {
            body["@microsoft.graph.conflictBehavior"] = c.as_str().into();
        }
        let mut req = RawRequest::new(Method::POST, format!("{}/children", self.base));
        req.body = Body::Json(body);
        self.t.send(&req).await?.json()
    }

    /// Renames, changes the description of, or moves the item, and returns the result.
    pub async fn update(&self, changes: &DriveItemUpdate) -> Result<DriveItem> {
        let mut req = RawRequest::new(Method::PATCH, self.base.as_str());
        req.body = Body::Json(changes.to_json());
        self.t.send(&req).await?.json()
    }

    /// Starts copying the item into folder `parent_id` of drive `drive_id`, optionally under a
    /// new name. The service copies asynchronously; use the returned handle to follow it.
    pub async fn copy(
        &self,
        drive_id: &str,
        parent_id: &str,
        name: Option<&str>,
    ) -> Result<CopyOperation> {
        let mut body = json!({ "parentReference": { "driveId": drive_id, "id": parent_id } });
        if let Some(n) = name {
            body["name"] = n.into();
        }
        let mut req = RawRequest::new(Method::POST, format!("{}/copy", self.base));
        req.body = Body::Json(body);
        let resp = self.t.send(&req).await?;
        let url = resp
            .headers
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                Error::Config(
                    "the copy was accepted without a monitor URL (Location header)".into(),
                )
            })?;
        Ok(CopyOperation::new(self.t.clone(), url.to_owned()))
    }

    /// Lists the versions of the item across all pages.
    pub async fn versions(&self) -> Result<Vec<DriveItemVersion>> {
        pager(
            &self.t,
            &format!("{}/versions", self.base),
            &QueryOptions::new(),
        )
        .collect_all()
        .await
    }

    /// Restores a version, for example `"2.0"`.
    pub async fn restore_version(&self, version_id: &str) -> Result<()> {
        self.t
            .send(&RawRequest::new(
                Method::POST,
                format!(
                    "{}/versions/{}/restoreVersion",
                    self.base,
                    encode_path(version_id)
                ),
            ))
            .await?;
        Ok(())
    }

    /// Downloads the content of a version.
    pub async fn download_version(&self, version_id: &str) -> Result<Bytes> {
        Ok(self
            .t
            .send(&RawRequest::new(
                Method::GET,
                format!("{}/versions/{}/content", self.base, encode_path(version_id)),
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
    use super::*;
    use crate::graph::testutil::client;
    use bytes::Bytes;
    use serde_json::json;
    use std::time::Duration;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_bytes, body_json, header, method, path, query_param},
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

    #[test]
    fn drive_item_metadata_present_and_absent() {
        let i: DriveItem = serde_json::from_value(json!({
            "id":"1","name":"a.txt","eTag":"\"e\"","cTag":"\"c\"",
            "createdDateTime":"2020-01-01T00:00:00Z","lastModifiedDateTime":"2020-02-01T00:00:00Z",
            "createdBy":{"user":{"displayName":"A"}},"lastModifiedBy":{"user":{"displayName":"B"}},
            "parentReference":{"id":"p","driveId":"d","path":"/drive/root:"},
            "file":{"mimeType":"text/plain"},"unknown":1
        }))
        .unwrap();
        assert_eq!(i.mime_type(), Some("text/plain"));
        assert_eq!(i.e_tag.as_deref(), Some("\"e\""));
        assert_eq!(i.c_tag.as_deref(), Some("\"c\""));
        assert!(!i.is_deleted());
        assert_eq!(
            i.parent_reference.clone().unwrap().drive_id.as_deref(),
            Some("d")
        );
        assert_eq!(
            i.created_by.unwrap().user.unwrap().display_name.as_deref(),
            Some("A")
        );

        let i: DriveItem = serde_json::from_value(json!({"id":"2"})).unwrap();
        assert!(i.mime_type().is_none() && i.parent_reference.is_none() && i.e_tag.is_none());
    }

    #[tokio::test]
    async fn create_folder_with_and_without_conflict_behavior() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/drives/d/items/p/children"))
            .and(body_json(json!({"name":"Reports","folder":{}})))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!({"id":"f1","name":"Reports","folder":{}})),
            )
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/drives/d/root/children"))
            .and(body_json(json!({
                "name":"Reports","folder":{},"@microsoft.graph.conflictBehavior":"rename"
            })))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!({"id":"f2","name":"Reports 1","folder":{}})),
            )
            .mount(&s)
            .await;
        let f = c
            .drive("d")
            .item("p")
            .create_folder("Reports", None)
            .await
            .unwrap();
        assert!(f.is_folder() && f.id == "f1");
        let f = c
            .drive("d")
            .root()
            .create_folder("Reports", Some(ConflictBehavior::Rename))
            .await
            .unwrap();
        assert_eq!(f.name.as_deref(), Some("Reports 1"));
    }

    #[tokio::test]
    async fn create_folder_conflict_is_an_http_error() {
        let (c, s) = client().await;
        Mock::given(path("/drives/d/root/children"))
            .respond_with(
                ResponseTemplate::new(409).set_body_json(
                    json!({"error":{"code":"nameAlreadyExists","message":"exists"}}),
                ),
            )
            .mount(&s)
            .await;
        let e = c
            .drive("d")
            .root()
            .create_folder("X", None)
            .await
            .unwrap_err();
        assert_eq!(e.status(), Some(409));
    }

    #[tokio::test]
    async fn rename_and_move_use_one_patch() {
        let (c, s) = client().await;
        Mock::given(method("PATCH"))
            .and(path("/drives/d/items/i"))
            .and(body_json(json!({"name":"b.txt"})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"i","name":"b.txt"})),
            )
            .mount(&s)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/drives/d/items/j"))
            .and(body_json(
                json!({"name":"c.txt","description":"x","parentReference":{"id":"dest"}}),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"j","name":"c.txt"})),
            )
            .mount(&s)
            .await;
        let r = c
            .drive("d")
            .item("i")
            .update(&DriveItemUpdate::new().name("b.txt"))
            .await
            .unwrap();
        assert_eq!(r.name.as_deref(), Some("b.txt"));
        c.drive("d")
            .item("j")
            .update(
                &DriveItemUpdate::new()
                    .name("c.txt")
                    .description("x")
                    .move_to("dest"),
            )
            .await
            .unwrap();
    }

    async fn mount_copy(s: &wiremock::MockServer, location: Option<String>) {
        let mut r = ResponseTemplate::new(202);
        if let Some(l) = location {
            r = r.insert_header("Location", l.as_str());
        }
        Mock::given(method("POST"))
            .and(path("/drives/d/items/i/copy"))
            .and(body_json(
                json!({"parentReference":{"driveId":"d2","id":"dest"},"name":"copy.txt"}),
            ))
            .respond_with(r)
            .mount(s)
            .await;
    }

    #[tokio::test]
    async fn copy_waits_for_completion_without_a_token_on_the_monitor() {
        let (c, s) = client().await;
        mount_copy(&s, Some(format!("{}/monitor/1", s.uri()))).await;
        Mock::given(path("/monitor/1"))
            .respond_with(
                ResponseTemplate::new(202)
                    .set_body_json(json!({"status":"inProgress","percentageComplete":10.0})),
            )
            .up_to_n_times(1)
            .mount(&s)
            .await;
        Mock::given(path("/monitor/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"status":"completed","percentageComplete":100.0,"resourceId":"new1"}),
            ))
            .mount(&s)
            .await;
        let op = c
            .drive("d")
            .item("i")
            .copy("d2", "dest", Some("copy.txt"))
            .await
            .unwrap();
        let first = op.status().await.unwrap();
        assert_eq!(first.status, CopyStatus::InProgress);
        let done = op
            .wait(Duration::from_millis(5), Duration::from_secs(5))
            .await
            .unwrap();
        assert_eq!(done.status, CopyStatus::Completed);
        assert_eq!(done.resource_id.as_deref(), Some("new1"));
        let monitor_calls: Vec<_> = s
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|r| r.url.path() == "/monitor/1")
            .collect();
        assert!(!monitor_calls.is_empty());
        assert!(
            monitor_calls
                .iter()
                .all(|r| !r.headers.contains_key("authorization"))
        );
    }

    #[tokio::test]
    async fn copy_failure_is_an_error() {
        let (c, s) = client().await;
        mount_copy(&s, Some(format!("{}/monitor/2", s.uri()))).await;
        Mock::given(path("/monitor/2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"status":"failed","error":{"message":"quota"}})),
            )
            .mount(&s)
            .await;
        let op = c
            .drive("d")
            .item("i")
            .copy("d2", "dest", Some("copy.txt"))
            .await
            .unwrap();
        let e = op
            .wait(Duration::from_millis(5), Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(
            matches!(&e, Error::Http { code: Some(c), message, .. } if c == "copyFailed" && message == "quota")
        );
    }

    #[tokio::test]
    async fn copy_without_location_is_an_error() {
        let (c, s) = client().await;
        mount_copy(&s, None).await;
        let e = c
            .drive("d")
            .item("i")
            .copy("d2", "dest", Some("copy.txt"))
            .await
            .unwrap_err();
        assert!(matches!(e, Error::Config(_)));
    }

    #[tokio::test]
    async fn copy_wait_times_out() {
        let (c, s) = client().await;
        mount_copy(&s, Some(format!("{}/monitor/3", s.uri()))).await;
        Mock::given(path("/monitor/3"))
            .respond_with(ResponseTemplate::new(202).set_body_json(json!({"status":"weird"})))
            .mount(&s)
            .await;
        let op = c
            .drive("d")
            .item("i")
            .copy("d2", "dest", Some("copy.txt"))
            .await
            .unwrap();
        let e = op
            .wait(Duration::from_millis(10), Duration::from_millis(40))
            .await
            .unwrap_err();
        assert!(matches!(e, Error::Io(ref io) if io.kind() == std::io::ErrorKind::TimedOut));
    }

    #[tokio::test]
    async fn versions_list_restore_and_download() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/drives/d/items/i/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value":[{"id":"2.0","size":5},{"id":"1.0","size":3}],
                "@odata.nextLink": format!("{}/drives/d/items/i/versions?page=2", s.uri())
            })))
            .up_to_n_times(1)
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/drives/d/items/i/versions"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"0.5"}]})))
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/drives/d/items/i/versions/1.0/restoreVersion"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/drives/d/items/i/versions/1.0/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes("old"))
            .mount(&s)
            .await;
        let item = c.drive("d").item("i");
        let v = item.versions().await.unwrap();
        assert_eq!(v.len(), 3);
        item.restore_version("1.0").await.unwrap();
        assert_eq!(
            item.download_version("1.0").await.unwrap(),
            Bytes::from("old")
        );
    }

    #[tokio::test]
    async fn delta_follows_pages_resumes_and_flags_deleted_items() {
        let (c, s) = client().await;
        Mock::given(path("/drives/d/root/delta"))
            .and(query_param("token", "t0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value":[{"id":"1","name":"a"}],
                "@odata.nextLink": format!("{}/drives/d/root/delta?page=2", s.uri())
            })))
            .mount(&s)
            .await;
        Mock::given(path("/drives/d/root/delta"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value":[{"id":"2","deleted":{"state":"deleted"}}],
                "@odata.deltaLink": format!("{}/drives/d/root/delta?token=t1", s.uri())
            })))
            .mount(&s)
            .await;
        Mock::given(path("/drives/d/root/delta"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value":[{"id":"1"},{"id":"2"},{"id":"3"}],
                "@odata.deltaLink": format!("{}/drives/d/root/delta?token=t0", s.uri())
            })))
            .mount(&s)
            .await;
        let initial = c.drive("d").delta(None).await.unwrap();
        assert_eq!(initial.items.len(), 3);
        assert_eq!(initial.next_token.as_deref(), Some("t0"));
        let resumed = c.drive("d").delta(Some("t0")).await.unwrap();
        assert_eq!(resumed.items.len(), 2);
        assert!(resumed.items[1].is_deleted() && !resumed.items[0].is_deleted());
        assert_eq!(resumed.next_token.as_deref(), Some("t1"));
    }

    #[tokio::test]
    async fn expired_delta_token_is_an_http_error() {
        let (c, s) = client().await;
        Mock::given(path("/drives/d/root/delta"))
            .respond_with(
                ResponseTemplate::new(410)
                    .set_body_json(json!({"error":{"code":"resyncRequired","message":"expired"}})),
            )
            .mount(&s)
            .await;
        let e = c.drive("d").delta(Some("old")).await.unwrap_err();
        assert_eq!(e.status(), Some(410));
    }

    #[tokio::test]
    async fn download_to_streams_into_a_writer() {
        let (c, s) = client().await;
        let data = vec![1u8; 700_000];
        Mock::given(path("/drives/d/items/i/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(data.clone()))
            .mount(&s)
            .await;
        let mut out = Vec::new();
        let n = c.drive("d").item("i").download_to(&mut out).await.unwrap();
        assert_eq!(n, 700_000);
        assert_eq!(out, data);
    }

    #[tokio::test]
    async fn download_to_writes_nothing_on_error() {
        let (c, s) = client().await;
        Mock::given(path("/drives/d/items/gone/content"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(json!({"error":{"code":"itemNotFound","message":"x"}})),
            )
            .mount(&s)
            .await;
        let mut out = Vec::new();
        let e = c
            .drive("d")
            .item("gone")
            .download_to(&mut out)
            .await
            .unwrap_err();
        assert_eq!(e.status(), Some(404));
        assert!(out.is_empty());
    }
}
