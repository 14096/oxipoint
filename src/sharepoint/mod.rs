//! Typed, fluent client for the SharePoint REST API (`/_api`).
//!
//! ```no_run
//! # async fn run(cred: rsp::auth::StaticTokenCredential) -> rsp::Result<()> {
//! use rsp::sharepoint::SharePointClient;
//! let sp = SharePointClient::new("https://contoso.sharepoint.com/sites/team", cred)?;
//! let web = sp.web().get().await?;
//! let items = sp.list_by_title("Tasks").items().filter("Status eq 'Open'").top(50).list().await?;
//! sp.folder("/sites/team/Shared Documents").upload("a.txt", "hello", true).await?;
//! let pages = sp.web().pages().list().await?;
//! # let _ = (web, items, pages); Ok(()) }
//! ```

/// Defines a handle over a paged collection of `$item`.
macro_rules! collection {
    ($(#[$m:meta])* $name:ident => $item:ty) => {
        $(#[$m])*
        #[derive(Debug, Clone)]
        pub struct $name {
            inner: ::std::sync::Arc<$crate::sharepoint::Inner>,
            base: String,
            query: $crate::http::QueryOptions,
        }

        impl $name {
            pub(crate) fn new(
                inner: ::std::sync::Arc<$crate::sharepoint::Inner>,
                base: String,
            ) -> Self {
                Self { inner, base, query: $crate::http::QueryOptions::new() }
            }

            /// Lists all items, following paging links.
            pub async fn list(&self) -> $crate::Result<Vec<$item>> {
                self.pages().collect_all().await
            }

            /// Returns a pager.
            pub fn pages(&self) -> $crate::http::Pager<$item> {
                $crate::http::Pager::new(
                    self.inner.t.clone(),
                    $crate::sharepoint::inner::get_req(&self.base, &self.query),
                )
            }
        }
        $crate::http::query::query_methods!($name);
    };
}

/// Defines a handle over a single `$item` that can be read.
macro_rules! entity {
    ($(#[$m:meta])* $name:ident => $item:ty) => {
        $(#[$m])*
        #[derive(Debug, Clone)]
        pub struct $name {
            inner: ::std::sync::Arc<$crate::sharepoint::Inner>,
            base: String,
            query: $crate::http::QueryOptions,
        }

        impl $name {
            pub(crate) fn new(
                inner: ::std::sync::Arc<$crate::sharepoint::Inner>,
                base: String,
            ) -> Self {
                Self { inner, base, query: $crate::http::QueryOptions::new() }
            }

            /// Fetches the entity.
            pub async fn get(&self) -> $crate::Result<$item> {
                self.inner
                    .t
                    .send(&$crate::sharepoint::inner::get_req(&self.base, &self.query))
                    .await?
                    .json()
            }
        }
        $crate::http::query::query_methods!($name);
    };
}

mod batch;
mod changes;
mod content_types;
mod fields;
mod files;
mod inner;
mod lists;
#[cfg(test)]
mod model_tests;
mod pages;
mod principals;
mod search;
mod web;

use std::sync::Arc;

use reqwest::Method;

use tokio::sync::Mutex;
use url::Url;

pub use batch::{SharePointBatch, SharePointBatchRequest};
pub use changes::{Change, ChangeQuery};
pub use content_types::{ContentType, ContentTypeRequest, ContentTypesRequest};
pub use fields::{Field, FieldRequest, FieldsRequest};
pub use files::{
    CheckInKind, FileInfo, FileRequest, FileVersion, FolderEntry, FolderInfo, FolderRequest,
};
pub use lists::{
    Attachment, AttachmentRequest, AttachmentsRequest, Comment, CommentsRequest, FieldResult,
    ItemRequest, ItemVersion, ItemsRequest, ListContentTypeRequest, ListContentTypesRequest,
    ListFieldRequest, ListFieldsRequest, ListInfo, ListItem, ListRequest, ListTemplate,
    ListsRequest, NewList, NewSubscription, RootFolder, Subscription, SubscriptionsRequest,
    ValidateOptions, View, ViewRequest, ViewsRequest,
};
pub use pages::{NewSitePage, PageRequest, PagesRequest, SitePage};
pub use principals::{
    BasePermissions, PermissionKind, PermissionsRequest, RoleAssignment, RoleDefinition,
    RoleDefinitionsRequest, SiteGroup, SiteGroupRequest, SiteGroupsRequest, SpUser, UserRequest,
    UsersRequest,
};
pub use search::{SpSearch, SpSearchPage, SpSearchRow};
pub use web::{
    Feature, FeaturesRequest, NavigationNode, NavigationNodeRequest, NavigationNodesRequest,
    NavigationRequest, NewWeb, RecycleBinItem, RecycleBinItemRequest, RecycleBinRequest,
    RegionalSettings, SiteInfo, TimeZone, Web, WebRequest,
};

use inner::ACCEPT;
pub(crate) use inner::{Inner, odata_str};

use crate::{
    Error, Result,
    auth::{CachedCredential, TokenCredential},
    http::{RequestBuilder, Transport, TransportConfig},
};

/// Entry point for one SharePoint site (or web). Cheap to clone.
///
/// # Files, search and batching
/// ```no_run
/// # async fn run(sp: rsp::sharepoint::SharePointClient) -> rsp::Result<()> {
/// use reqwest::Method;
/// use rsp::sharepoint::{CheckInKind, SharePointBatchRequest};
///
/// let file = sp.file("/sites/team/Shared Documents/plan.docx");
/// file.checkout().await?;
/// file.checkin("Updated the schedule", CheckInKind::Major).await?;
/// file.copy_to("/sites/team/Archive/plan.docx", false).await?;
/// let versions = file.versions().await?;
/// let mut out = tokio::fs::File::create("plan.docx").await?; // needs tokio's `fs` feature
/// file.download_to(&mut out).await?;
///
/// let everything = sp.folder("/sites/team/Shared Documents").list_recursive(false).await?;
///
/// // Search
/// let rows = sp.search("contract").select(["Title", "Path"]).all().await?;
/// for row in &rows {
///     println!("{:?}", row.get("Title"));
/// }
///
/// // Batching: reads, plus writes that succeed or fail together
/// let mut batch = sp.batch();
/// batch.add(SharePointBatchRequest::new(Method::GET, "/web/title"))?;
/// batch.add_changeset([
///     SharePointBatchRequest::new(Method::POST, "/web/lists/getbytitle('Tasks')/items")
///         .json(&serde_json::json!({ "Title": "A" }))?,
///     SharePointBatchRequest::new(Method::POST, "/web/lists/getbytitle('Tasks')/items")
///         .json(&serde_json::json!({ "Title": "B" }))?,
/// ])?;
/// for result in batch.send().await? {
///     println!("{}", result.status);
/// }
/// # let _ = (versions, everything); Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct SharePointClient {
    inner: Arc<Inner>,
}

impl SharePointClient {
    /// Creates a client for `site_url` such as `https://contoso.sharepoint.com/sites/team`.
    ///
    /// Tokens are requested for the `<origin>/.default` scope.
    pub fn new(site_url: &str, credential: impl TokenCredential + 'static) -> Result<Self> {
        Self::with_config(site_url, credential, TransportConfig::default())
    }

    /// Like [`SharePointClient::new`] with custom retry settings.
    pub fn with_config(
        site_url: &str,
        credential: impl TokenCredential + 'static,
        mut config: TransportConfig,
    ) -> Result<Self> {
        let url =
            Url::parse(site_url).map_err(|e| Error::Config(format!("invalid site URL: {e}")))?;
        if url.cannot_be_a_base() || url.host_str().is_none() {
            return Err(Error::Config("site URL must be absolute".into()));
        }
        let origin = url.origin().ascii_serialization();
        config
            .default_headers
            .push(("Accept".into(), ACCEPT.into()));
        let base = format!("{}/_api", site_url.trim_end_matches('/'));
        let t = Transport::new(
            Arc::new(CachedCredential::new(credential)),
            format!("{origin}/.default"),
            base,
            config,
        );
        Ok(Self {
            inner: Arc::new(Inner {
                t: Arc::new(t),
                digest: Mutex::new(None),
            }),
        })
    }

    /// Generic request to any `/_api` path (relative) or absolute URL on the same host.
    pub fn request(&self, method: Method, path: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.inner.t.clone(), method, path)
    }

    /// Starts a search over the site collection (`/_api/search/query`) for `query_text`.
    pub fn search(&self, query_text: &str) -> SpSearch {
        SpSearch::new(self.inner.clone(), query_text)
    }

    /// Starts a batch that sends several calls in one HTTP request.
    pub fn batch(&self) -> SharePointBatch {
        SharePointBatch::new(self.inner.clone())
    }

    /// The web (site) itself and its subwebs.
    pub fn web(&self) -> WebRequest {
        WebRequest::new(self.inner.clone())
    }

    /// All lists of the web.
    pub fn lists(&self) -> ListsRequest {
        ListsRequest::new(self.inner.clone())
    }

    /// A list by its title.
    pub fn list_by_title(&self, title: &str) -> ListRequest {
        ListRequest::new(
            self.inner.clone(),
            format!("/web/lists/getbytitle('{}')", odata_str(title)),
        )
    }

    /// A list by its GUID.
    pub fn list_by_id(&self, id: &str) -> ListRequest {
        ListRequest::new(
            self.inner.clone(),
            format!("/web/lists(guid'{}')", odata_str(id)),
        )
    }

    /// A folder by server-relative URL, e.g. `/sites/team/Shared Documents/Reports`.
    pub fn folder(&self, server_relative_url: &str) -> FolderRequest {
        FolderRequest::new(self.inner.clone(), server_relative_url)
    }

    /// A file by server-relative URL.
    pub fn file(&self, server_relative_url: &str) -> FileRequest {
        FileRequest::new(self.inner.clone(), server_relative_url)
    }
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;
    use crate::auth::StaticTokenCredential;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    pub async fn client() -> (SharePointClient, MockServer) {
        let server = MockServer::start().await;
        let c = SharePointClient::with_config(
            &server.uri(),
            StaticTokenCredential::new("tok"),
            TransportConfig {
                max_retries: 0,
                ..Default::default()
            },
        )
        .unwrap();
        (c, server)
    }

    pub async fn mock_get(s: &MockServer, p: &str, resp: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path(p))
            .respond_with(ResponseTemplate::new(200).set_body_json(resp))
            .expect(1)
            .mount(s)
            .await;
    }

    /// Mounts a digest-protected POST; `tunnel` is the expected `X-HTTP-Method`.
    pub async fn mock_write(
        s: &MockServer,
        p: &str,
        tunnel: Option<&str>,
        body: Option<serde_json::Value>,
        resp: serde_json::Value,
    ) {
        use wiremock::matchers::{body_json, header};
        let b = Mock::given(method("POST"))
            .and(path(p))
            .and(header("x-requestdigest", "DIGEST"));
        let b = match tunnel {
            Some(v) => b
                .and(header("x-http-method", v))
                .and(header("if-match", "*")),
            None => b,
        };
        let b = match body {
            Some(v) => b.and(body_json(v)),
            None => b,
        };
        b.respond_with(if resp.is_null() {
            ResponseTemplate::new(204)
        } else {
            ResponseTemplate::new(200).set_body_json(resp)
        })
        .expect(1)
        .mount(s)
        .await;
    }

    pub async fn client_retrying() -> (SharePointClient, MockServer) {
        let server = MockServer::start().await;
        let c = SharePointClient::with_config(
            &server.uri(),
            StaticTokenCredential::new("tok"),
            TransportConfig {
                max_retries: 1,
                default_retry_delay: std::time::Duration::from_millis(1),
                ..Default::default()
            },
        )
        .unwrap();
        (c, server)
    }

    pub async fn mount_digest(s: &MockServer, expected_calls: u64) {
        Mock::given(method("POST"))
            .and(path("/_api/contextinfo"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"FormDigestValue":"DIGEST","FormDigestTimeoutSeconds":1800}),
                ),
            )
            .expect(expected_calls)
            .mount(s)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::*;
    use super::*;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{header, method, path},
    };

    #[test]
    fn rejects_bad_site_url() {
        let c = || crate::auth::StaticTokenCredential::new("t");
        assert!(matches!(
            SharePointClient::new("not a url", c()),
            Err(Error::Config(_))
        ));
        assert!(matches!(
            SharePointClient::new("mailto:a@b.c", c()),
            Err(Error::Config(_))
        ));
    }

    #[test]
    fn odata_escaping() {
        assert_eq!(odata_str("it's a"), "it%27%27s%20a");
    }

    #[tokio::test]
    async fn sends_json_light_accept_header_and_scope_is_origin() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web"))
            .and(header("accept", "application/json;odata=nometadata"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"Id":"1","Title":"T","Url":"u"})),
            )
            .mount(&s)
            .await;
        c.web().get().await.unwrap();
        assert_eq!(c.inner.t.base_url(), format!("{}/_api", s.uri()));
    }

    #[test]
    fn token_scope_targets_tenant_host() {
        let c = SharePointClient::new(
            "https://contoso.sharepoint.com/sites/team",
            crate::auth::StaticTokenCredential::new("t"),
        )
        .unwrap();
        assert_eq!(
            c.inner.t.base_url(),
            "https://contoso.sharepoint.com/sites/team/_api"
        );
        let u = Url::parse("https://contoso.sharepoint.com/sites/team").unwrap();
        assert_eq!(
            format!("{}/.default", u.origin().ascii_serialization()),
            "https://contoso.sharepoint.com/.default"
        );
    }

    #[tokio::test]
    async fn digest_is_cached_across_writes() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web/lists/getbytitle('L')/items"))
            .and(header("x-requestdigest", "DIGEST"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"Id":1})))
            .expect(2)
            .mount(&s)
            .await;
        let items = c.list_by_title("L").items();
        items.create(&json!({"Title":"a"})).await.unwrap();
        items.create(&json!({"Title":"b"})).await.unwrap();
    }

    #[tokio::test]
    async fn generic_request_works() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web/currentuser"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Id":5})))
            .mount(&s)
            .await;
        let v: serde_json::Value = c
            .request(Method::GET, "/web/currentuser")
            .send_json()
            .await
            .unwrap();
        assert_eq!(v["Id"], 5);
    }
}
