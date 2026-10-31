mod columns;
mod content_types;
mod models;
mod pages;
mod permissions;

pub use columns::{ColumnRequest, ColumnsRequest};
pub use content_types::{ContentTypeRequest, ContentTypesRequest};
pub use models::{
    CanvasLayout, Column, ContentType, HorizontalSection, HorizontalSectionColumn, NewSitePage,
    Permission, PublishingState, Site, SiteCollection, SitePage, WebPart,
};
pub use pages::{PageRequest, PagesRequest, WebPartRequest, WebPartsRequest};
pub use permissions::{PermissionRequest, PermissionsRequest};

use std::sync::Arc;

use crate::graph::*;
use crate::{
    Result,
    http::{Pager, QueryOptions, Transport, encode_path, query::query_methods},
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
        ListsRequest::new(self.t.clone(), format!("{}/lists", self.base))
    }

    /// A list by id or display name.
    pub fn list(&self, id_or_name: &str) -> ListRequest {
        ListRequest::new(
            self.t.clone(),
            format!("{}/lists/{}", self.base, encode_path(id_or_name)),
        )
    }

    /// The site's subsites, across all pages.
    pub async fn sites(&self) -> Result<Vec<Site>> {
        pager(
            &self.t,
            &format!("{}/sites", self.base),
            &QueryOptions::new(),
        )
        .collect_all()
        .await
    }

    /// A subsite by id.
    pub fn subsite(&self, id: &str) -> SiteRequest {
        SiteRequest::new(
            self.t.clone(),
            format!("{}/sites/{}", self.base, encode_path(id)),
        )
    }

    /// A subsite by path relative to this site, e.g. `"child"` or `"child/grandchild"`.
    ///
    /// Uses the `getByPath(path='...')` form.
    pub fn site_by_path(&self, path: &str) -> SiteRequest {
        let p = encode_path(&path.trim_matches('/').replace('\'', "''"));
        SiteRequest::new(
            self.t.clone(),
            format!("{}/getByPath(path='{p}')", self.base),
        )
    }

    /// The site's columns.
    pub fn columns(&self) -> ColumnsRequest {
        ColumnsRequest::new(self.t.clone(), format!("{}/columns", self.base))
    }

    /// The site's content types.
    pub fn content_types(&self) -> ContentTypesRequest {
        ContentTypesRequest::new(self.t.clone(), format!("{}/contentTypes", self.base))
    }

    /// The site's modern pages.
    pub fn pages(&self) -> PagesRequest {
        PagesRequest::new(self.t.clone(), format!("{}/pages", self.base))
    }

    /// The site's permissions.
    pub fn permissions(&self) -> PermissionsRequest {
        PermissionsRequest::new(self.t.clone(), format!("{}/permissions", self.base))
    }

    /// All-time activity statistics of the site.
    pub async fn analytics(&self) -> Result<Analytics> {
        get_json(
            &self.t,
            &format!("{}/analytics/allTime", self.base),
            &QueryOptions::new(),
        )
        .await
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

#[cfg(test)]
mod tests {
    use crate::graph::testutil::client;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{method, path, query_param},
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
    async fn subsites_list_pages_and_by_path() {
        let (c, s) = client().await;
        let next = format!("{}/sites/s1/sites?p=2", s.uri());
        Mock::given(path("/sites/s1/sites"))
            .and(query_param("p", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"b"}]})))
            .mount(&s)
            .await;
        Mock::given(path("/sites/s1/sites"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"a"}],"@odata.nextLink":next})),
            )
            .mount(&s)
            .await;
        Mock::given(path("/sites/s1/sites/sub1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"sub1"})))
            .mount(&s)
            .await;
        Mock::given(path("/sites/s1/getByPath(path='child')"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"c","displayName":"Child"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let site = c.sites().by_id("s1");
        assert_eq!(site.sites().await.unwrap().len(), 2);
        assert_eq!(site.subsite("sub1").get().await.unwrap().id, "sub1");
        let child = site.site_by_path("/child/").get().await.unwrap();
        assert_eq!(child.display_name.as_deref(), Some("Child"));
    }

    #[tokio::test]
    async fn followed_sites_and_analytics() {
        let (c, s) = client().await;
        Mock::given(path("/me/followedSites"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"f1"}]})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/sites/s1/analytics/allTime"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"access":{"actionCount":10,"actorCount":4}})),
            )
            .expect(1)
            .mount(&s)
            .await;
        assert_eq!(c.followed_sites().await.unwrap()[0].id, "f1");
        let a = c.sites().by_id("s1").analytics().await.unwrap();
        assert_eq!(a.access.unwrap().actor_count, Some(4));
    }
}

#[cfg(test)]
mod part_tests {
    use crate::graph::testutil::client;
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, method, path, query_param},
    };

    async fn get(s: &wiremock::MockServer, p: &str, body: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path(p))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(s)
            .await;
    }

    #[tokio::test]
    async fn columns_list_get_create_delete() {
        let (c, s) = client().await;
        get(
            &s,
            "/sites/s1/columns",
            json!({"value":[{"id":"c1","name":"Title"}]}),
        )
        .await;
        get(&s, "/sites/s1/columns/c%201", json!({"id":"c 1"})).await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/columns"))
            .and(body_json(json!({"name":"N","text":{}})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"c2","name":"N"})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/columns/c2"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let cols = c.sites().by_id("s1").columns();
        assert_eq!(cols.list().await.unwrap()[0].name.as_deref(), Some("Title"));
        assert_eq!(cols.by_id("c 1").get().await.unwrap().id, "c 1");
        assert_eq!(
            cols.create(&json!({"name":"N","text":{}}))
                .await
                .unwrap()
                .id,
            "c2"
        );
        cols.by_id("c2").delete().await.unwrap();
    }

    #[tokio::test]
    async fn content_types_list_get_create_delete() {
        let (c, s) = client().await;
        get(
            &s,
            "/sites/s1/contentTypes",
            json!({"value":[{"id":"0x01"}]}),
        )
        .await;
        get(
            &s,
            "/sites/s1/contentTypes/0x0101",
            json!({"id":"0x0101","name":"Document"}),
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/contentTypes"))
            .and(body_json(json!({"name":"Mine","base":{"id":"0x01"}})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"0x01AB"})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/contentTypes/0x01AB"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let cts = c.sites().by_id("s1").content_types();
        assert_eq!(cts.list().await.unwrap()[0].id, "0x01");
        assert_eq!(
            cts.by_id("0x0101").get().await.unwrap().name.as_deref(),
            Some("Document")
        );
        let made = cts
            .create(&json!({"name":"Mine","base":{"id":"0x01"}}))
            .await
            .unwrap();
        assert_eq!(made.id, "0x01AB");
        cts.by_id("0x01AB").delete().await.unwrap();
    }

    #[tokio::test]
    async fn permissions_crud() {
        let (c, s) = client().await;
        let next = format!("{}/sites/s1/permissions?p=2", s.uri());
        Mock::given(path("/sites/s1/permissions"))
            .and(query_param("p", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"b"}]})))
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/sites/s1/permissions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"value":[{"id":"a","roles":["read"]}],"@odata.nextLink":next}),
            ))
            .mount(&s)
            .await;
        get(
            &s,
            "/sites/s1/permissions/a",
            json!({"id":"a","roles":["read"]}),
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/permissions"))
            .and(body_json(json!({"roles":["write"]})))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(json!({"id":"n","roles":["write"]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/sites/s1/permissions/a"))
            .and(body_json(json!({"roles":["write"]})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"a","roles":["write"]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/permissions/a"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let perms = c.sites().by_id("s1").permissions();
        let all = perms.list().await.unwrap();
        assert_eq!(
            (all.len(), all[0].roles.clone()),
            (2, vec!["read".to_owned()])
        );
        assert_eq!(perms.by_id("a").get().await.unwrap().id, "a");
        assert_eq!(
            perms.create(&json!({"roles":["write"]})).await.unwrap().id,
            "n"
        );
        let up = perms
            .by_id("a")
            .update(&json!({"roles":["write"]}))
            .await
            .unwrap();
        assert_eq!(up.roles, vec!["write".to_owned()]);
        let unit: () = perms.by_id("a").delete().await.unwrap();
        assert_eq!(unit, ());
    }
}
