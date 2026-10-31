use std::sync::Arc;

use bytes::Bytes;
use reqwest::Method;
use serde_json::json;

use super::{FileInfo, FolderInfo, Inner, odata_str};
use crate::{
    Result,
    http::{Body, Pager, QueryOptions, RawRequest, query::query_methods},
};

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

    /// Creates this folder (and returns its properties).
    pub async fn create(&self) -> Result<FolderInfo> {
        let mut req = RawRequest::new(Method::POST, "/web/folders");
        req.body = Body::Json(json!({ "ServerRelativeUrl": self.url }));
        self.inner.write(req).await?.json()
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
    base: String,
}

impl FileRequest {
    pub(crate) fn new(inner: Arc<Inner>, server_relative_url: &str) -> Self {
        let base = format!(
            "/web/GetFileByServerRelativePath(decodedurl='{}')",
            odata_str(server_relative_url)
        );
        Self { inner, base }
    }

    /// Fetches file properties.
    pub async fn get(&self) -> Result<FileInfo> {
        self.inner
            .t
            .send(&RawRequest::new(Method::GET, self.base.as_str()))
            .await?
            .json()
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

    /// Deletes the file.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::sharepoint::testutil::{client, mount_digest};
    use bytes::Bytes;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_bytes, header, method, path},
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
}
