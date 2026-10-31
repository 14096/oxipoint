use std::sync::Arc;

use reqwest::Method;
use serde::Serialize;
use serde_json::{Map, Value, json};

use super::{Drive, DriveRequest, List, ListItem, Site, get_json, pager};
use crate::{
    Result,
    http::{Body, Pager, QueryOptions, RawRequest, Transport, encode_path, query::query_methods},
};

/// `/sites` — site lookup and search.
#[derive(Debug, Clone)]
pub struct SitesRequest {
    t: Arc<Transport>,
}

impl SitesRequest {
    pub(crate) fn new(t: Arc<Transport>) -> Self {
        Self { t }
    }

    /// The tenant root site.
    pub fn root(&self) -> SiteRequest {
        SiteRequest::new(self.t.clone(), "/sites/root".to_owned())
    }

    /// A site by composite id.
    pub fn by_id(&self, id: &str) -> SiteRequest {
        SiteRequest::new(self.t.clone(), format!("/sites/{}", encode_path(id)))
    }

    /// A site by host name and server-relative path, e.g. `("contoso.sharepoint.com", "/sites/team")`.
    pub fn by_path(&self, host: &str, path: &str) -> SiteRequest {
        let p = path.trim_matches('/');
        SiteRequest::new(self.t.clone(), format!("/sites/{host}:/{}", encode_path(p)))
    }

    /// Searches sites by keyword.
    pub async fn search(&self, text: &str) -> Result<Vec<Site>> {
        let q = QueryOptions::new();
        let mut req = super::request_with("/sites", &q);
        req.query.push(("search".into(), text.to_owned()));
        Pager::<Site>::new(self.t.clone(), req).collect_all().await
    }
}

/// One site.
#[derive(Debug, Clone)]
pub struct SiteRequest {
    t: Arc<Transport>,
    base: String,
    query: QueryOptions,
}

impl SiteRequest {
    fn new(t: Arc<Transport>, base: String) -> Self {
        Self {
            t,
            base,
            query: QueryOptions::new(),
        }
    }

    /// Fetches the site.
    pub async fn get(&self) -> Result<Site> {
        get_json(&self.t, &self.base, &self.query).await
    }

    /// The site's lists.
    pub fn lists(&self) -> ListsRequest {
        ListsRequest {
            t: self.t.clone(),
            base: format!("{}/lists", self.base),
            query: QueryOptions::new(),
        }
    }

    /// A list by id or display name.
    pub fn list(&self, id_or_name: &str) -> ListRequest {
        ListRequest {
            t: self.t.clone(),
            base: format!("{}/lists/{}", self.base, encode_path(id_or_name)),
            query: QueryOptions::new(),
        }
    }

    /// The default document library.
    pub fn drive(&self) -> DriveRequest {
        DriveRequest::new(self.t.clone(), format!("{}/drive", self.base))
    }

    /// All document libraries of the site.
    pub async fn drives(&self) -> Result<Vec<Drive>> {
        pager(
            &self.t,
            &format!("{}/drives", self.base),
            &QueryOptions::new(),
        )
        .collect_all()
        .await
    }
}
query_methods!(SiteRequest);

/// A site's lists.
#[derive(Debug, Clone)]
pub struct ListsRequest {
    t: Arc<Transport>,
    base: String,
    query: QueryOptions,
}

impl ListsRequest {
    /// Lists all lists, following paging links.
    pub async fn list(&self) -> Result<Vec<List>> {
        self.pages().collect_all().await
    }

    /// Returns a pager.
    pub fn pages(&self) -> Pager<List> {
        pager(&self.t, &self.base, &self.query)
    }
}
query_methods!(ListsRequest);

/// One list.
#[derive(Debug, Clone)]
pub struct ListRequest {
    t: Arc<Transport>,
    base: String,
    query: QueryOptions,
}

impl ListRequest {
    /// Fetches the list.
    pub async fn get(&self) -> Result<List> {
        get_json(&self.t, &self.base, &self.query).await
    }

    /// The list's items.
    pub fn items(&self) -> ListItemsRequest {
        ListItemsRequest {
            t: self.t.clone(),
            base: format!("{}/items", self.base),
            query: QueryOptions::new(),
        }
    }

    /// One item by id.
    pub fn item(&self, id: &str) -> ListItemRequest {
        ListItemRequest {
            t: self.t.clone(),
            base: format!("{}/items/{}", self.base, encode_path(id)),
            query: QueryOptions::new(),
        }
    }
}
query_methods!(ListRequest);

/// A list's items collection.
#[derive(Debug, Clone)]
pub struct ListItemsRequest {
    t: Arc<Transport>,
    base: String,
    query: QueryOptions,
}

impl ListItemsRequest {
    /// Lists items, following paging links. Use `.expand("fields")` to include column values.
    pub async fn list(&self) -> Result<Vec<ListItem>> {
        self.pages().collect_all().await
    }

    /// Returns a pager.
    pub fn pages(&self) -> Pager<ListItem> {
        pager(&self.t, &self.base, &self.query)
    }

    /// Creates an item from a serializable set of column values.
    pub async fn create<F: Serialize>(&self, fields: &F) -> Result<ListItem> {
        let mut req = RawRequest::new(Method::POST, self.base.as_str());
        req.body = Body::Json(json!({ "fields": serde_json::to_value(fields)? }));
        self.t.send(&req).await?.json()
    }
}
query_methods!(ListItemsRequest);

/// One list item.
#[derive(Debug, Clone)]
pub struct ListItemRequest {
    t: Arc<Transport>,
    base: String,
    query: QueryOptions,
}

impl ListItemRequest {
    /// Fetches the item.
    pub async fn get(&self) -> Result<ListItem> {
        get_json(&self.t, &self.base, &self.query).await
    }

    /// Updates the given columns and returns the resulting column values.
    pub async fn update<F: Serialize>(&self, fields: &F) -> Result<Map<String, Value>> {
        let mut req = RawRequest::new(Method::PATCH, format!("{}/fields", self.base));
        req.body = Body::Json(serde_json::to_value(fields)?);
        self.t.send(&req).await?.json()
    }

    /// Deletes the item.
    pub async fn delete(&self) -> Result<()> {
        self.t
            .send(&RawRequest::new(Method::DELETE, self.base.as_str()))
            .await?;
        Ok(())
    }
}
query_methods!(ListItemRequest);

#[cfg(test)]
mod tests {
    use crate::{Error, graph::testutil::client};
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, method, path, query_param},
    };

    #[tokio::test]
    async fn chained_navigation_hits_expected_url() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path(
                "/sites/contoso.sharepoint.com:/sites/team/lists/L1/items",
            ))
            .and(query_param("$expand", "fields"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"1","fields":{"Title":"x"}}]})),
            )
            .mount(&s)
            .await;
        let items = c
            .sites()
            .by_path("contoso.sharepoint.com", "/sites/team/")
            .list("L1")
            .items()
            .expand("fields")
            .list()
            .await
            .unwrap();
        assert_eq!(items[0].fields.as_ref().unwrap()["Title"], "x");
    }

    #[tokio::test]
    async fn site_root_and_search_and_lists() {
        let (c, s) = client().await;
        Mock::given(path("/sites/root"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"r","displayName":"Root"})),
            )
            .mount(&s)
            .await;
        Mock::given(path("/sites"))
            .and(query_param("search", "team"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"s1"}]})))
            .mount(&s)
            .await;
        Mock::given(path("/sites/s1/lists"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"l1","displayName":"Docs"}]})),
            )
            .mount(&s)
            .await;
        assert_eq!(
            c.sites()
                .root()
                .get()
                .await
                .unwrap()
                .display_name
                .as_deref(),
            Some("Root")
        );
        assert_eq!(c.sites().search("team").await.unwrap().len(), 1);
        assert_eq!(
            c.sites().by_id("s1").lists().list().await.unwrap()[0].id,
            "l1"
        );
    }

    #[tokio::test]
    async fn create_item_returns_id() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/lists/L/items"))
            .and(body_json(json!({"fields":{"Title":"New"}})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"42"})))
            .mount(&s)
            .await;
        let item = c
            .sites()
            .by_id("s1")
            .list("L")
            .items()
            .create(&json!({"Title":"New"}))
            .await
            .unwrap();
        assert_eq!(item.id, "42");
    }

    #[tokio::test]
    async fn update_and_delete_item() {
        let (c, s) = client().await;
        Mock::given(method("PATCH"))
            .and(path("/sites/s1/lists/L/items/7/fields"))
            .and(body_json(json!({"Title":"T2"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Title":"T2"})))
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/lists/L/items/7"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&s)
            .await;
        let item = c.sites().by_id("s1").list("L").item("7");
        assert_eq!(
            item.update(&json!({"Title":"T2"})).await.unwrap()["Title"],
            "T2"
        );
        item.delete().await.unwrap();
    }

    #[tokio::test]
    async fn delete_missing_item_is_404() {
        let (c, s) = client().await;
        Mock::given(method("DELETE"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(json!({"error":{"code":"itemNotFound","message":"x"}})),
            )
            .mount(&s)
            .await;
        let e = c
            .sites()
            .by_id("s1")
            .list("L")
            .item("9")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(e, Error::Http { status: 404, .. }));
    }
}
