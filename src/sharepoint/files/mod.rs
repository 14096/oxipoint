mod models;

pub use models::{CheckInKind, FileInfo, FileVersion, FolderEntry, FolderInfo};

use std::{collections::VecDeque, sync::Arc};

use bytes::Bytes;
use reqwest::Method;
use serde_json::{Value, json};
use tokio::io::AsyncWrite;
use url::Url;

use super::{Inner, odata_str};
use crate::{
    Error, Result,
    http::{Body, Pager, QueryOptions, RawRequest, query::query_methods},
};

/// Returns the folder part of a server-relative URL (`/a/b/c.txt` gives `/a/b`).
fn parent_of(url: &str) -> &str {
    url.trim_end_matches('/')
        .rsplit_once('/')
        .map(|(p, _)| p)
        .unwrap_or("")
}

/// Extracts the string a SharePoint action returns, in light (`value`) or verbose
/// (`d.<name>`) format.
fn action_string(doc: &Value, verbose_name: &str) -> Option<String> {
    doc.get("value")
        .or_else(|| doc.pointer(&format!("/d/{verbose_name}")))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// A folder identified by server-relative URL.
#[derive(Debug, Clone)]
pub struct FolderRequest {
    inner: Arc<Inner>,
    url: String,
    base: String,
    query: QueryOptions,
}

impl FolderRequest {
    pub(crate) fn new(inner: Arc<Inner>, server_relative_url: &str) -> Self {
        let base = format!(
            "/web/GetFolderByServerRelativePath(decodedurl='{}')",
            odata_str(server_relative_url)
        );
        Self {
            inner,
            url: server_relative_url.to_owned(),
            base,
            query: QueryOptions::new(),
        }
    }

    fn get_req(&self, path: &str) -> RawRequest {
        let mut r = RawRequest::new(Method::GET, path);
        r.query = self.query.pairs().to_vec();
        r
    }

    /// Fetches folder properties.
    pub async fn get(&self) -> Result<FolderInfo> {
        self.inner.t.send(&self.get_req(&self.base)).await?.json()
    }

    /// Lists sub-folders.
    pub async fn folders(&self) -> Result<Vec<FolderInfo>> {
        Pager::new(
            self.inner.t.clone(),
            self.get_req(&format!("{}/Folders", self.base)),
        )
        .collect_all()
        .await
    }

    /// Lists files.
    pub async fn files(&self) -> Result<Vec<FileInfo>> {
        Pager::new(
            self.inner.t.clone(),
            self.get_req(&format!("{}/Files", self.base)),
        )
        .collect_all()
        .await
    }

    /// Lists every file and folder below this folder, at any depth.
    ///
    /// Folders named `Forms` (the system folder of a library) are left out unless
    /// `include_system` is set. Query options apply to every listing request.
    pub async fn list_recursive(&self, include_system: bool) -> Result<Vec<FolderEntry>> {
        let mut out = Vec::new();
        let mut queue = VecDeque::from([self.clone()]);
        while let Some(folder) = queue.pop_front() {
            for f in folder.folders().await? {
                if !include_system && f.name.eq_ignore_ascii_case("Forms") {
                    continue;
                }
                let mut sub = FolderRequest::new(self.inner.clone(), &f.server_relative_url);
                sub.query = self.query.clone();
                queue.push_back(sub);
                out.push(FolderEntry::Folder(f));
            }
            out.extend(
                folder
                    .files()
                    .await?
                    .into_iter()
                    .map(|f| FolderEntry::File(Box::new(f))),
            );
        }
        Ok(out)
    }

    /// Creates this folder (and returns its properties).
    pub async fn create(&self) -> Result<FolderInfo> {
        let mut req = RawRequest::new(Method::POST, "/web/folders");
        req.body = Body::Json(json!({ "ServerRelativeUrl": self.url }));
        self.inner.write(req).await?.json()
    }

    /// Deletes the folder and everything in it.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }

    /// Moves the folder to the recycle bin and returns the recycle bin item id.
    pub async fn recycle(&self) -> Result<String> {
        let req = RawRequest::new(Method::POST, format!("{}/Recycle", self.base));
        let doc: Value = self.inner.write(req).await?.json()?;
        action_string(&doc, "Recycle")
            .ok_or_else(|| Error::Config("the recycle call returned no recycle bin item id".into()))
    }

    /// Moves the folder to `new_url` (server-relative), which must not exist.
    pub async fn move_to(&self, new_url: &str) -> Result<()> {
        let req = RawRequest::new(
            Method::POST,
            format!("{}/MoveTo(newurl='{}')", self.base, odata_str(new_url)),
        );
        self.inner.write(req).await?;
        Ok(())
    }

    /// Renames the folder within its parent.
    pub async fn rename(&self, new_name: &str) -> Result<()> {
        self.move_to(&format!("{}/{}", parent_of(&self.url), new_name))
            .await
    }

    /// Copies the folder to `new_url` (server-relative) with `SP.MoveCopyUtil`, because folders
    /// have no direct copy endpoint.
    pub async fn copy_to(&self, new_url: &str) -> Result<()> {
        let base = Url::parse(self.inner.t.base_url())
            .map_err(|e| Error::Config(format!("invalid site URL: {e}")))?;
        let origin = base.origin().ascii_serialization();
        let path = |p: &str| {
            json!({
                "__metadata": { "type": "SP.ResourcePath" },
                "DecodedUrl": format!("{origin}{p}"),
            })
        };
        let mut req = RawRequest::new(Method::POST, "/SP.MoveCopyUtil.CopyFolderByPath");
        req.headers.push((
            "Content-Type".into(),
            "application/json;odata=verbose".into(),
        ));
        req.body = Body::Json(json!({
            "srcPath": path(&self.url),
            "destPath": path(new_url),
            "options": {
                "__metadata": { "type": "SP.MoveCopyOptions" },
                "KeepBoth": false,
                "ShouldBypassSharedLocks": true,
            },
        }));
        self.inner.write(req).await?;
        Ok(())
    }

    /// Uploads a file into this folder, optionally overwriting an existing one.
    pub async fn upload(
        &self,
        name: &str,
        content: impl Into<Bytes>,
        overwrite: bool,
    ) -> Result<FileInfo> {
        let mut req = RawRequest::new(
            Method::POST,
            format!(
                "{}/Files/add(url='{}',overwrite={})",
                self.base,
                odata_str(name),
                overwrite
            ),
        );
        req.body = Body::Bytes(content.into());
        self.inner.write(req).await?.json()
    }
}
query_methods!(FolderRequest);

/// A file identified by server-relative URL.
#[derive(Debug, Clone)]
pub struct FileRequest {
    inner: Arc<Inner>,
    url: String,
    base: String,
    query: QueryOptions,
}

impl FileRequest {
    pub(crate) fn new(inner: Arc<Inner>, server_relative_url: &str) -> Self {
        let base = format!(
            "/web/GetFileByServerRelativePath(decodedurl='{}')",
            odata_str(server_relative_url)
        );
        Self {
            inner,
            url: server_relative_url.to_owned(),
            base,
            query: QueryOptions::new(),
        }
    }

    async fn action(&self, path: String) -> Result<()> {
        self.inner
            .write(RawRequest::new(Method::POST, path))
            .await?;
        Ok(())
    }

    /// Fetches file properties. Use `.expand("ListItemAllFields")`, `.expand("Author")` and
    /// similar to include related data.
    pub async fn get(&self) -> Result<FileInfo> {
        let mut r = RawRequest::new(Method::GET, self.base.as_str());
        r.query = self.query.pairs().to_vec();
        self.inner.t.send(&r).await?.json()
    }

    /// Downloads the file content.
    pub async fn download(&self) -> Result<Bytes> {
        Ok(self
            .inner
            .t
            .send(&RawRequest::new(
                Method::GET,
                format!("{}/$value", self.base),
            ))
            .await?
            .body)
    }

    /// Downloads the file content into `writer` as it arrives, returning the bytes written.
    ///
    /// Nothing is written when the service answers with an error status.
    pub async fn download_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> Result<u64> {
        self.inner
            .t
            .send_stream(&RawRequest::new(
                Method::GET,
                format!("{}/$value", self.base),
            ))
            .await?
            .copy_to(writer)
            .await
    }

    /// Deletes the file.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }

    /// Copies the file to `new_url` (server-relative). An existing file is replaced only when
    /// `overwrite` is set; otherwise the service answers with an error.
    pub async fn copy_to(&self, new_url: &str, overwrite: bool) -> Result<()> {
        self.action(format!(
            "{}/copyTo(strnewurl='{}',boverwrite={})",
            self.base,
            odata_str(new_url),
            overwrite
        ))
        .await
    }

    /// Moves the file to `new_url` (server-relative), replacing an existing file only when
    /// `overwrite` is set.
    pub async fn move_to(&self, new_url: &str, overwrite: bool) -> Result<()> {
        self.action(format!(
            "{}/moveTo(newurl='{}',flags={})",
            self.base,
            odata_str(new_url),
            u8::from(overwrite)
        ))
        .await
    }

    /// Renames the file within its folder, keeping its other properties and versions.
    pub async fn rename(&self, new_name: &str) -> Result<()> {
        self.move_to(&format!("{}/{}", parent_of(&self.url), new_name), false)
            .await
    }

    /// Checks the file out.
    pub async fn checkout(&self) -> Result<()> {
        self.action(format!("{}/checkout", self.base)).await
    }

    /// Checks the file in with a comment.
    pub async fn checkin(&self, comment: &str, kind: CheckInKind) -> Result<()> {
        self.action(format!(
            "{}/checkin(comment='{}',checkintype={})",
            self.base,
            odata_str(comment),
            kind.code()
        ))
        .await
    }

    /// Discards a checkout.
    pub async fn undo_checkout(&self) -> Result<()> {
        self.action(format!("{}/undocheckout", self.base)).await
    }

    /// Publishes the file with a comment.
    pub async fn publish(&self, comment: &str) -> Result<()> {
        self.action(format!(
            "{}/publish(comment='{}')",
            self.base,
            odata_str(comment)
        ))
        .await
    }

    /// Unpublishes the file with a comment.
    pub async fn unpublish(&self, comment: &str) -> Result<()> {
        self.action(format!(
            "{}/unpublish(comment='{}')",
            self.base,
            odata_str(comment)
        ))
        .await
    }

    /// Approves a file waiting for content approval.
    pub async fn approve(&self, comment: &str) -> Result<()> {
        self.action(format!(
            "{}/approve(comment='{}')",
            self.base,
            odata_str(comment)
        ))
        .await
    }

    /// Denies a file waiting for content approval.
    pub async fn deny(&self, comment: &str) -> Result<()> {
        self.action(format!(
            "{}/deny(comment='{}')",
            self.base,
            odata_str(comment)
        ))
        .await
    }

    /// Lists the versions of the file across all pages.
    pub async fn versions(&self) -> Result<Vec<FileVersion>> {
        Pager::new(
            self.inner.t.clone(),
            RawRequest::new(Method::GET, format!("{}/Versions", self.base)),
        )
        .collect_all()
        .await
    }

    /// Reads one version by its id.
    pub async fn version(&self, id: i64) -> Result<FileVersion> {
        self.inner
            .t
            .send(&RawRequest::new(
                Method::GET,
                format!("{}/Versions({id})", self.base),
            ))
            .await?
            .json()
    }

    /// Downloads the content of the version with this id.
    pub async fn download_version(&self, id: i64) -> Result<Bytes> {
        Ok(self
            .inner
            .t
            .send(&RawRequest::new(
                Method::GET,
                format!("{}/Versions({id})/$value", self.base),
            ))
            .await?
            .body)
    }

    /// Restores the version with this label, for example `"2.0"`.
    pub async fn restore_version(&self, label: &str) -> Result<()> {
        self.action(format!(
            "{}/Versions/restoreByLabel(versionlabel='{}')",
            self.base,
            odata_str(label)
        ))
        .await
    }

    /// Deletes the version with this label.
    pub async fn delete_version(&self, label: &str) -> Result<()> {
        self.action(format!(
            "{}/Versions/deleteByLabel(versionlabel='{}')",
            self.base,
            odata_str(label)
        ))
        .await
    }
}
query_methods!(FileRequest);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharepoint::testutil::{client, mock_get, mock_write, mount_digest};
    use bytes::Bytes;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_bytes, header, method, path, query_param},
    };

    #[tokio::test]
    async fn download_file() {
        let (c, s) = client().await;
        Mock::given(path(
            "/_api/web/GetFileByServerRelativePath(decodedurl='/sites/t/Docs/a.txt')/$value",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_bytes("data"))
        .mount(&s)
        .await;
        assert_eq!(
            c.file("/sites/t/Docs/a.txt").download().await.unwrap(),
            Bytes::from("data")
        );
    }

    #[tokio::test]
    async fn special_characters_are_escaped() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web/GetFileByServerRelativePath(decodedurl='/sites/t/My%20Docs/it%27%27s%20%231.txt')"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Name":"it's #1.txt","ServerRelativeUrl":"/x","Length":"12"})))
            .mount(&s)
            .await;
        let f = c.file("/sites/t/My Docs/it's #1.txt").get().await.unwrap();
        assert_eq!(f.length.as_deref(), Some("12"));
    }

    #[tokio::test]
    async fn list_folder_contents() {
        let (c, s) = client().await;
        Mock::given(path(
            "/_api/web/GetFolderByServerRelativePath(decodedurl='/sites/t/Docs')/Folders",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"value":[{"Name":"sub","ServerRelativeUrl":"/sites/t/Docs/sub"}]}),
        ))
        .mount(&s)
        .await;
        Mock::given(path(
            "/_api/web/GetFolderByServerRelativePath(decodedurl='/sites/t/Docs')/Files",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"value":[{"Name":"a.txt","ServerRelativeUrl":"/sites/t/Docs/a.txt"}]}),
        ))
        .mount(&s)
        .await;
        let f = c.folder("/sites/t/Docs");
        assert_eq!(f.folders().await.unwrap()[0].name, "sub");
        assert_eq!(f.files().await.unwrap()[0].name, "a.txt");
    }

    #[tokio::test]
    async fn upload_and_create_folder_use_digest() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web/GetFolderByServerRelativePath(decodedurl='/sites/t/Docs')/Files/add(url='n%201.txt',overwrite=true)"))
            .and(header("x-requestdigest", "DIGEST"))
            .and(body_bytes("hi"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Name":"n 1.txt","ServerRelativeUrl":"/sites/t/Docs/n 1.txt"})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/_api/web/folders"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"Name":"new","ServerRelativeUrl":"/sites/t/Docs/new"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let docs = c.folder("/sites/t/Docs");
        assert_eq!(
            docs.upload("n 1.txt", "hi", true).await.unwrap().name,
            "n 1.txt"
        );
        assert_eq!(
            c.folder("/sites/t/Docs/new").create().await.unwrap().name,
            "new"
        );
    }

    #[tokio::test]
    async fn delete_file_uses_tunnel() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(header("x-http-method", "DELETE"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&s)
            .await;
        c.file("/sites/t/Docs/a.txt").delete().await.unwrap();
    }

    const FILE: &str = "/_api/web/GetFileByServerRelativePath(decodedurl='/sites/t/Docs/a.txt')";
    const FOLDER: &str = "/_api/web/GetFolderByServerRelativePath(decodedurl='/sites/t/Docs')";

    #[tokio::test]
    async fn copy_move_and_rename_files() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            &format!("{FILE}/copyTo(strnewurl='/sites/t/Archive/a.txt',boverwrite=false)"),
            None,
            None,
            json!(null),
        )
        .await;
        mock_write(
            &s,
            &format!("{FILE}/moveTo(newurl='/sites/t/Archive/a.txt',flags=1)"),
            None,
            None,
            json!(null),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/GetFileByServerRelativePath(decodedurl='/sites/t/Docs/it%27%27s%20%231.txt')/moveTo(newurl='/sites/t/Docs/b%20c.txt',flags=0)",
            None,
            None,
            json!(null),
        )
        .await;
        let f = c.file("/sites/t/Docs/a.txt");
        f.copy_to("/sites/t/Archive/a.txt", false).await.unwrap();
        f.move_to("/sites/t/Archive/a.txt", true).await.unwrap();
        c.file("/sites/t/Docs/it's #1.txt")
            .rename("b c.txt")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn copy_onto_existing_file_fails_with_the_service_error() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path(format!(
                "{FILE}/copyTo(strnewurl='/sites/t/Archive/a.txt',boverwrite=false)"
            )))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "odata.error": {"code":"-2130575257, Microsoft.SharePoint.SPException",
                                "message":{"value":"already exists"}}
            })))
            .mount(&s)
            .await;
        let e = c
            .file("/sites/t/Docs/a.txt")
            .copy_to("/sites/t/Archive/a.txt", false)
            .await
            .unwrap_err();
        assert_eq!(e.status(), Some(500));
    }

    #[tokio::test]
    async fn folder_delete_recycle_move_rename_and_copy() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(&s, FOLDER, Some("DELETE"), None, json!(null)).await;
        mock_write(
            &s,
            &format!("{FOLDER}/Recycle"),
            None,
            None,
            json!({"value":"6b1c5b14-0000-0000-0000-000000000001"}),
        )
        .await;
        mock_write(
            &s,
            &format!("{FOLDER}/MoveTo(newurl='/sites/t/Archive/Docs')"),
            None,
            None,
            json!(null),
        )
        .await;
        mock_write(
            &s,
            &format!("{FOLDER}/MoveTo(newurl='/sites/t/New%20name')"),
            None,
            None,
            json!(null),
        )
        .await;
        let origin = s.uri();
        Mock::given(method("POST"))
            .and(path("/_api/SP.MoveCopyUtil.CopyFolderByPath"))
            .and(header("x-requestdigest", "DIGEST"))
            .and(wiremock::matchers::body_partial_json(json!({
                "srcPath": {"DecodedUrl": format!("{origin}/sites/t/Docs")},
                "destPath": {"DecodedUrl": format!("{origin}/sites/t/Copy")},
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":null})))
            .expect(1)
            .mount(&s)
            .await;
        let f = c.folder("/sites/t/Docs");
        assert_eq!(
            f.recycle().await.unwrap(),
            "6b1c5b14-0000-0000-0000-000000000001"
        );
        f.move_to("/sites/t/Archive/Docs").await.unwrap();
        f.rename("New name").await.unwrap();
        f.copy_to("/sites/t/Copy").await.unwrap();
        f.delete().await.unwrap();
    }

    async fn mount_tree(s: &wiremock::MockServer, forms: bool) {
        let get = |p: String, v: serde_json::Value| async move {
            Mock::given(method("GET"))
                .and(path(p))
                .respond_with(ResponseTemplate::new(200).set_body_json(v))
                .mount(s)
                .await;
        };
        let f = |n: &str, u: &str| json!({"Name": n, "ServerRelativeUrl": u});
        let root = "/_api/web/GetFolderByServerRelativePath(decodedurl='/sites/t/Docs')";
        let a = "/_api/web/GetFolderByServerRelativePath(decodedurl='/sites/t/Docs/A')";
        let b = "/_api/web/GetFolderByServerRelativePath(decodedurl='/sites/t/Docs/A/B')";
        get(
            format!("{root}/Folders"),
            json!({"value":[f("Forms","/sites/t/Docs/Forms"), f("A","/sites/t/Docs/A")]}),
        )
        .await;
        get(
            format!("{root}/Files"),
            json!({"value":[f("r.txt","/sites/t/Docs/r.txt")]}),
        )
        .await;
        get(
            format!("{a}/Folders"),
            json!({"value":[f("B","/sites/t/Docs/A/B")]}),
        )
        .await;
        get(
            format!("{a}/Files"),
            json!({"value":[f("a1.txt","/sites/t/Docs/A/a1.txt")]}),
        )
        .await;
        get(format!("{b}/Folders"), json!({"value":[]})).await;
        get(
            format!("{b}/Files"),
            json!({"value":[f("b.txt","/sites/t/Docs/A/B/b.txt")]}),
        )
        .await;
        if forms {
            let fm = "/_api/web/GetFolderByServerRelativePath(decodedurl='/sites/t/Docs/Forms')";
            get(format!("{fm}/Folders"), json!({"value":[]})).await;
            get(
                format!("{fm}/Files"),
                json!({"value":[f("x.aspx","/sites/t/Docs/Forms/x.aspx")]}),
            )
            .await;
        }
    }

    #[tokio::test]
    async fn recursive_listing_walks_nested_folders_and_skips_forms() {
        let (c, s) = client().await;
        mount_tree(&s, false).await;
        let mut urls: Vec<String> = c
            .folder("/sites/t/Docs")
            .list_recursive(false)
            .await
            .unwrap()
            .iter()
            .map(|e| e.server_relative_url().to_owned())
            .collect();
        urls.sort();
        assert_eq!(
            urls,
            [
                "/sites/t/Docs/A",
                "/sites/t/Docs/A/B",
                "/sites/t/Docs/A/B/b.txt",
                "/sites/t/Docs/A/a1.txt",
                "/sites/t/Docs/r.txt",
            ]
        );
    }

    #[tokio::test]
    async fn recursive_listing_can_include_the_forms_folder() {
        let (c, s) = client().await;
        mount_tree(&s, true).await;
        let all = c
            .folder("/sites/t/Docs")
            .list_recursive(true)
            .await
            .unwrap();
        assert!(
            all.iter()
                .any(|e| e.server_relative_url() == "/sites/t/Docs/Forms/x.aspx")
        );
    }

    #[tokio::test]
    async fn checkout_checkin_and_publishing() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        for tail in [
            "checkout",
            "checkin(comment='done',checkintype=1)",
            "checkin(comment='wip',checkintype=0)",
            "undocheckout",
            "publish(comment='ship')",
            "unpublish(comment='pull')",
            "approve(comment='ok')",
            "deny(comment='no')",
        ] {
            mock_write(&s, &format!("{FILE}/{tail}"), None, None, json!(null)).await;
        }
        let f = c.file("/sites/t/Docs/a.txt");
        f.checkout().await.unwrap();
        f.checkin("done", CheckInKind::Major).await.unwrap();
        f.checkin("wip", CheckInKind::Minor).await.unwrap();
        f.undo_checkout().await.unwrap();
        f.publish("ship").await.unwrap();
        f.unpublish("pull").await.unwrap();
        f.approve("ok").await.unwrap();
        f.deny("no").await.unwrap();
    }

    #[tokio::test]
    async fn file_versions() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_get(
            &s,
            &format!("{FILE}/Versions"),
            json!({"value":[
                {"ID":512,"VersionLabel":"1.0","Size":3,"IsCurrentVersion":false,"Url":"_vti_history/512/a.txt"},
                {"ID":1024,"VersionLabel":"2.0","IsCurrentVersion":true}
            ]}),
        )
        .await;
        mock_get(
            &s,
            &format!("{FILE}/Versions(512)"),
            json!({"ID":512,"VersionLabel":"1.0","CheckInComment":"first"}),
        )
        .await;
        Mock::given(path(format!("{FILE}/Versions(512)/$value")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes("old"))
            .mount(&s)
            .await;
        mock_write(
            &s,
            &format!("{FILE}/Versions/restoreByLabel(versionlabel='2.0')"),
            None,
            None,
            json!(null),
        )
        .await;
        mock_write(
            &s,
            &format!("{FILE}/Versions/deleteByLabel(versionlabel='1.0')"),
            None,
            None,
            json!(null),
        )
        .await;
        let f = c.file("/sites/t/Docs/a.txt");
        let v = f.versions().await.unwrap();
        assert_eq!((v.len(), v[1].is_current_version), (2, Some(true)));
        assert_eq!(v[0].version_label.as_deref(), Some("1.0"));
        assert_eq!(
            f.version(512).await.unwrap().check_in_comment.as_deref(),
            Some("first")
        );
        assert_eq!(f.download_version(512).await.unwrap(), Bytes::from("old"));
        f.restore_version("2.0").await.unwrap();
        f.delete_version("1.0").await.unwrap();
    }

    #[tokio::test]
    async fn file_properties_through_expand() {
        let (c, s) = client().await;
        Mock::given(path(FILE))
            .and(query_param(
                "$expand",
                "ListItemAllFields,Author,CheckedOutByUser",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Name":"a.txt","ServerRelativeUrl":"/sites/t/Docs/a.txt","CheckInComment":"v1",
                "ListItemAllFields":{"Id":7,"Title":"T"},
                "Author":{"Id":3,"Title":"Ann"},
                "CheckedOutByUser":{"Id":4,"Title":"Bob"}
            })))
            .mount(&s)
            .await;
        let f = c
            .file("/sites/t/Docs/a.txt")
            .expand("ListItemAllFields,Author,CheckedOutByUser")
            .get()
            .await
            .unwrap();
        assert_eq!(f.list_item.unwrap()["Title"], "T");
        assert_eq!(f.author.unwrap().title.as_deref(), Some("Ann"));
        assert_eq!(f.checked_out_by_user.unwrap().id, 4);
        assert_eq!(f.check_in_comment.as_deref(), Some("v1"));
        assert!(f.modified_by.is_none());
    }

    #[tokio::test]
    async fn download_to_streams_and_writes_nothing_on_error() {
        let (c, s) = client().await;
        let data = vec![9u8; 600_000];
        Mock::given(path(format!("{FILE}/$value")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(data.clone()))
            .mount(&s)
            .await;
        Mock::given(path(
            "/_api/web/GetFileByServerRelativePath(decodedurl='/sites/t/Docs/gone.txt')/$value",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&s)
        .await;
        let mut out = Vec::new();
        let n = c
            .file("/sites/t/Docs/a.txt")
            .download_to(&mut out)
            .await
            .unwrap();
        assert_eq!((n, out), (600_000, data));
        let mut empty = Vec::new();
        let e = c
            .file("/sites/t/Docs/gone.txt")
            .download_to(&mut empty)
            .await
            .unwrap_err();
        assert_eq!(e.status(), Some(404));
        assert!(empty.is_empty());
    }
}
