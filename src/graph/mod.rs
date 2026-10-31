//! Typed, fluent client for the most common Microsoft Graph resources.
//!
//! ```no_run
//! # async fn run(cred: rsp::auth::StaticTokenCredential) -> rsp::Result<()> {
//! use rsp::graph::GraphClient;
//! let graph = GraphClient::new(cred);
//! let lists = graph.sites().by_path("contoso.sharepoint.com", "/sites/team").lists().list().await?;
//! let items = graph.sites().root().list("list-id").items().expand("fields").top(10).list().await?;
//! # let _ = (lists, items); Ok(()) }
//! ```

#[macro_use]
mod macros;
mod drives;
mod lists;
mod models;
mod site_parts;
mod sites;
mod users;

use std::sync::Arc;

use reqwest::Method;
use serde::de::DeserializeOwned;

pub use drives::{DriveItemRequest, DriveRequest};
pub use lists::{
    ListColumnRequest, ListColumnsRequest, ListContentTypeRequest, ListContentTypesRequest,
    ListItemRequest, ListItemsRequest, ListRequest, ListSubscriptionsRequest, ListsRequest,
};
pub use models::{
    ActivityCounts, Analytics, Column, ContentType, DeltaPage, Drive, DriveItem, Identity,
    IdentitySet, ItemActivity, ItemAnalytics, ItemVersion, List, ListFacet, ListItem,
    NewSubscription, ParentReference, Permission, Site, SiteCollection, Subscription, User,
};
pub use site_parts::{
    ColumnRequest, ColumnsRequest, ContentTypeRequest, ContentTypesRequest, PermissionRequest,
    PermissionsRequest,
};
pub use sites::{SiteRequest, SitesRequest};
pub use users::{MeRequest, UserRequest, UsersRequest};

use crate::{
    Result,
    auth::{CachedCredential, TokenCredential},
    http::{Pager, QueryOptions, RawRequest, RequestBuilder, Transport, TransportConfig},
};

/// Production Graph endpoint.
pub const DEFAULT_BASE_URL: &str = "https://graph.microsoft.com/v1.0";
const SCOPE: &str = "https://graph.microsoft.com/.default";

/// Entry point for Microsoft Graph. Cheap to clone.
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
