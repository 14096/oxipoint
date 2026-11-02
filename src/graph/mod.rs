//! Typed, fluent client for the most common Microsoft Graph resources.
//!
//! ```no_run
//! # async fn run(cred: oxipoint::auth::StaticTokenCredential) -> oxipoint::Result<()> {
//! use oxipoint::graph::GraphClient;
//! let graph = GraphClient::new(cred);
//! let lists = graph.sites().by_path("contoso.sharepoint.com", "/sites/team").lists().list().await?;
//! let items = graph.sites().root().list("list-id").items().expand("fields").top(10).list().await?;
//! let pages = graph.sites().root().pages().list().await?;
//! # let _ = (lists, items, pages); Ok(()) }
//! ```

#[macro_use]
mod macros;
mod batch;
mod common;
mod drives;
mod lists;
#[cfg(test)]
mod model_tests;
mod search;
mod sites;
mod users;

use std::sync::Arc;

use reqwest::Method;
use serde::de::DeserializeOwned;

pub use batch::{GraphBatch, GraphBatchRequest, MAX_BATCH_REQUESTS};
pub use common::{ActivityCounts, Analytics, Identity, IdentitySet, ParentReference};
pub use drives::{
    ConflictBehavior, CopyOperation, CopyProgress, CopyStatus, Drive, DriveDeltaPage, DriveItem,
    DriveItemRequest, DriveItemUpdate, DriveItemVersion, DriveRequest, Invite,
    ItemPermissionRequest, ItemPermissionsRequest, LinkOptions, LinkScope, LinkType,
};
pub use lists::{
    DeltaPage, ItemActivity, ItemAnalytics, ItemVersion, List, ListColumnRequest,
    ListColumnsRequest, ListContentTypeRequest, ListContentTypesRequest, ListFacet, ListItem,
    ListItemRequest, ListItemsRequest, ListRequest, ListSubscriptionsRequest, ListsRequest,
    NewSubscription, Subscription,
};
pub use search::{GraphSearch, SearchHit, SearchPage};
pub use sites::{
    CanvasLayout, Column, ColumnRequest, ColumnsRequest, ContentType, ContentTypeRequest,
    ContentTypesRequest, HorizontalSection, HorizontalSectionColumn, NewSitePage, PageRequest,
    PagesRequest, Permission, PermissionRequest, PermissionsRequest, PublishingState, Site,
    SiteCollection, SitePage, SiteRequest, SitesRequest, WebPart, WebPartRequest, WebPartsRequest,
};
pub use users::{MeRequest, User, UserRequest, UsersRequest};

use crate::{
    Result,
    auth::{CachedCredential, TokenCredential},
    http::{Pager, QueryOptions, RawRequest, RequestBuilder, Transport, TransportConfig},
};

/// Production Graph endpoint.
pub const DEFAULT_BASE_URL: &str = "https://graph.microsoft.com/v1.0";
const SCOPE: &str = "https://graph.microsoft.com/.default";

/// Entry point for Microsoft Graph. Cheap to clone.
///
/// # Files, sharing, search and batching
/// ```no_run
/// # async fn run(graph: oxipoint::graph::GraphClient) -> oxipoint::Result<()> {
/// use reqwest::Method;
/// use oxipoint::graph::{DriveItemUpdate, GraphBatchRequest, LinkOptions, LinkScope, LinkType};
/// use std::time::Duration;
///
/// let drive = graph.drive("<drive-id>");
///
/// // Folders, rename/move, copy (asynchronous on the service) and streamed download
/// let folder = drive.root().create_folder("Reports", None).await?;
/// let report = drive.path("Reports/q1.xlsx");
/// report.update(&DriveItemUpdate::new().name("q1-final.xlsx")).await?;
/// report.copy("<drive-id>", &folder.id, Some("backup.xlsx")).await?
///     .wait(Duration::from_secs(2), Duration::from_secs(120)).await?;
/// let mut out = tokio::fs::File::create("q1.xlsx").await?; // needs tokio's `fs` feature
/// report.download_to(&mut out).await?;
///
/// // Everything that changed in a drive since the last sync
/// let sync = drive.delta(None).await?;
/// let _later = drive.delta(sync.next_token.as_deref()).await?;
///
/// // Sharing
/// let link = report
///     .create_link(&LinkOptions::new(LinkType::View).scope(LinkScope::Organization))
///     .await?;
/// println!("{:?}", link.link_url());
///
/// // Search
/// let hits = graph.search().entity_types(["driveItem"]).query("budget").all().await?;
///
/// // Batching: up to 20 calls in one request, one result each
/// let mut batch = graph.batch();
/// batch.add(GraphBatchRequest::new(Method::GET, "/me"))?;
/// batch.add(GraphBatchRequest::new(Method::GET, "/me/drive"))?;
/// for result in batch.send().await? {
///     let _value: serde_json::Value = result.json()?; // a failed call is an Err here
/// }
/// # let _ = hits; Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct GraphClient {
    t: Arc<Transport>,
}

/// Builder for a [`GraphClient`] with non-default settings.
#[derive(Debug)]
pub struct GraphClientBuilder {
    credential: Arc<dyn TokenCredential>,
    base_url: String,
    config: TransportConfig,
}

impl GraphClientBuilder {
    /// Overrides the Graph base URL (for national clouds, beta endpoint or tests).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Sets the maximum number of retries for throttled requests.
    pub fn max_retries(mut self, n: u32) -> Self {
        self.config.max_retries = n;
        self
    }

    /// Builds the client.
    pub fn build(self) -> GraphClient {
        GraphClient {
            t: Arc::new(Transport::new(
                self.credential,
                SCOPE,
                self.base_url,
                self.config,
            )),
        }
    }
}

impl GraphClient {
    /// Creates a client using `credential` (tokens are cached automatically).
    pub fn new(credential: impl TokenCredential + 'static) -> Self {
        Self::builder(credential).build()
    }

    /// Starts a [`GraphClientBuilder`].
    pub fn builder(credential: impl TokenCredential + 'static) -> GraphClientBuilder {
        GraphClientBuilder {
            credential: Arc::new(CachedCredential::new(credential)),
            base_url: DEFAULT_BASE_URL.to_owned(),
            config: TransportConfig::default(),
        }
    }

    /// Generic request to any Graph path (relative to the base URL) or absolute Graph URL.
    pub fn request(&self, method: Method, path: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.t.clone(), method, path)
    }

    /// The signed-in user (`/me`).
    pub fn me(&self) -> MeRequest {
        MeRequest::new(self.t.clone())
    }

    /// The users collection.
    pub fn users(&self) -> UsersRequest {
        UsersRequest::new(self.t.clone())
    }

    /// A single user by id or user principal name.
    pub fn user(&self, id: &str) -> UserRequest {
        UserRequest::new(self.t.clone(), id)
    }

    /// Site lookup and search.
    pub fn sites(&self) -> SitesRequest {
        SitesRequest::new(self.t.clone())
    }

    /// Sites followed by the signed-in user, across all pages.
    pub async fn followed_sites(&self) -> Result<Vec<Site>> {
        pager(&self.t, "/me/followedSites", &QueryOptions::new())
            .collect_all()
            .await
    }

    /// Searches drive items, list items, sites and other entities (`/search/query`).
    pub fn search(&self) -> GraphSearch {
        GraphSearch::new(self.t.clone())
    }

    /// Starts a batch that sends up to 20 calls in one HTTP request.
    pub fn batch(&self) -> GraphBatch {
        GraphBatch::new(self.t.clone())
    }

    /// The item behind a sharing URL (`/shares`), without encoding the URL by hand.
    pub fn shared_item(&self, sharing_url: &str) -> DriveItemRequest {
        DriveItemRequest::new(
            self.t.clone(),
            format!(
                "/shares/{}/driveItem",
                drives::encode_share_url(sharing_url)
            ),
        )
    }

    /// A drive by id.
    pub fn drive(&self, id: &str) -> DriveRequest {
        DriveRequest::new(
            self.t.clone(),
            format!("/drives/{}", crate::http::encode_path(id)),
        )
    }
}

pub(crate) fn request_with(path: &str, q: &QueryOptions) -> RawRequest {
    let mut r = RawRequest::new(Method::GET, path);
    r.query = q.pairs().to_vec();
    r
}

pub(crate) async fn get_json<T: DeserializeOwned>(
    t: &Arc<Transport>,
    path: &str,
    q: &QueryOptions,
) -> Result<T> {
    t.send(&request_with(path, q)).await?.json()
}

pub(crate) fn pager<T: DeserializeOwned>(
    t: &Arc<Transport>,
    path: &str,
    q: &QueryOptions,
) -> Pager<T> {
    Pager::new(t.clone(), request_with(path, q))
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;
    use crate::auth::StaticTokenCredential;
    use wiremock::MockServer;

    pub async fn client() -> (GraphClient, MockServer) {
        let server = MockServer::start().await;
        let c = GraphClient::builder(StaticTokenCredential::new("tok"))
            .base_url(server.uri())
            .max_retries(0)
            .build();
        (c, server)
    }
}
