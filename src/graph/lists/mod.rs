mod columns;
mod content_types;
mod items;
mod models;
mod subscriptions;

pub use columns::{ListColumnRequest, ListColumnsRequest};
pub use content_types::{ListContentTypeRequest, ListContentTypesRequest};
pub use items::{ListItemRequest, ListItemsRequest};
pub use models::{
    DeltaPage, ItemActivity, ItemAnalytics, ItemVersion, List, ListFacet, ListItem,
    NewSubscription, Subscription,
};
pub use subscriptions::ListSubscriptionsRequest;

use std::sync::Arc;

use reqwest::Method;
use serde::Serialize;

use crate::graph::*;
use crate::{
    Result,
    http::{Body, Pager, QueryOptions, RawRequest, Transport, encode_path, query::query_methods},
};

/// A site's lists.
#[derive(Debug, Clone)]
pub struct ListsRequest {
    t: Arc<Transport>,
    base: String,
    query: QueryOptions,
}

impl ListsRequest {
    pub(crate) fn new(t: Arc<Transport>, base: String) -> Self {
        Self {
            t,
            base,
            query: QueryOptions::new(),
        }
    }

    /// Lists all lists, following paging links.
    pub async fn list(&self) -> Result<Vec<List>> {
        self.pages().collect_all().await
    }

    /// Returns a pager.
    pub fn pages(&self) -> Pager<List> {
        pager(&self.t, &self.base, &self.query)
    }

    /// Creates a list from a definition (display name and `list.template`) and returns it.
    pub async fn create<F: Serialize>(&self, definition: &F) -> Result<List> {
        let mut req = RawRequest::new(Method::POST, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(definition)?);
        self.t.send(&req).await?.json()
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
    pub(crate) fn new(t: Arc<Transport>, base: String) -> Self {
        Self {
            t,
            base,
            query: QueryOptions::new(),
        }
    }

    /// Fetches the list.
    pub async fn get(&self) -> Result<List> {
        get_json(&self.t, &self.base, &self.query).await
    }

    /// Updates list properties (PATCH) and returns the updated list.
    pub async fn update<F: Serialize>(&self, changes: &F) -> Result<List> {
        let mut req = RawRequest::new(Method::PATCH, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(changes)?);
        self.t.send(&req).await?.json()
    }

    /// Deletes the list.
    pub async fn delete(&self) -> Result<()> {
        self.t
            .send(&RawRequest::new(Method::DELETE, self.base.as_str()))
            .await?;
        Ok(())
    }

    /// The list's items.
    pub fn items(&self) -> ListItemsRequest {
        ListItemsRequest {
            t: self.t.clone(),
            base: format!("{}/items", self.base),
            query: QueryOptions::new(),
            unindexed: false,
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

    /// The list's columns.
    pub fn columns(&self) -> ListColumnsRequest {
        ListColumnsRequest::new(self.t.clone(), format!("{}/columns", self.base))
    }

    /// The list's content types.
    pub fn content_types(&self) -> ListContentTypesRequest {
        ListContentTypesRequest::new(self.t.clone(), format!("{}/contentTypes", self.base))
    }

    /// Change-notification subscriptions on this list.
    ///
    /// Graph subscriptions are global; this handle watches `sites/<site>/lists/<list>`, so
    /// address the site and list by id for it to work.
    pub fn subscriptions(&self) -> ListSubscriptionsRequest {
        ListSubscriptionsRequest {
            t: self.t.clone(),
            resource: self.base.trim_start_matches('/').to_owned(),
        }
    }
}
query_methods!(ListRequest);

#[cfg(test)]
mod tests {
    use crate::{
        Error,
        graph::{NewSubscription, testutil::client},
    };
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_json, header, method, path, query_param},
    };

    async fn get(s: &MockServer, p: &str, body: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path(p))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(s)
            .await;
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

    #[tokio::test]
    async fn list_create_and_delete() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/lists"))
            .and(body_json(
                json!({"displayName":"Books","list":{"template":"genericList"}}),
            ))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(json!({"id":"l9","displayName":"Books"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/lists/l9"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let site = c.sites().by_id("s1");
        let l = site
            .lists()
            .create(&json!({"displayName":"Books","list":{"template":"genericList"}}))
            .await
            .unwrap();
        assert_eq!(l.id, "l9");
        site.list("l9").delete().await.unwrap();
    }

    #[tokio::test]
    async fn update_list_patches_and_returns_list() {
        let (c, s) = client().await;
        Mock::given(method("PATCH"))
            .and(path("/sites/s1/lists/L"))
            .and(body_json(json!({"description":"new"})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"L","description":"new"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let l = c
            .sites()
            .by_id("s1")
            .list("L")
            .update(&json!({"description":"new"}))
            .await
            .unwrap();
        assert_eq!(l.description.as_deref(), Some("new"));
    }

    #[tokio::test]
    async fn columns_crud() {
        let (c, s) = client().await;
        get(
            &s,
            "/sites/s1/lists/L/columns",
            json!({"value":[{"id":"c1"}]}),
        )
        .await;
        get(
            &s,
            "/sites/s1/lists/L/columns/c1",
            json!({"id":"c1","name":"Status"}),
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/lists/L/columns"))
            .and(body_json(json!({"name":"N","text":{}})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"c2"})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/sites/s1/lists/L/columns/c2"))
            .and(body_json(json!({"displayName":"New"})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"c2","displayName":"New"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/lists/L/columns/c2"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let cols = c.sites().by_id("s1").list("L").columns();
        assert_eq!(cols.list().await.unwrap().len(), 1);
        assert_eq!(
            cols.by_id("c1").get().await.unwrap().name.as_deref(),
            Some("Status")
        );
        assert_eq!(
            cols.create(&json!({"name":"N","text":{}}))
                .await
                .unwrap()
                .id,
            "c2"
        );
        let up = cols
            .by_id("c2")
            .update(&json!({"displayName":"New"}))
            .await
            .unwrap();
        assert_eq!(up.display_name.as_deref(), Some("New"));
        cols.by_id("c2").delete().await.unwrap();
    }

    #[tokio::test]
    async fn content_types_list_get_add_copy_delete() {
        let (c, s) = client().await;
        get(
            &s,
            "/sites/s1/lists/L/contentTypes",
            json!({"value":[{"id":"0x01"}]}),
        )
        .await;
        get(
            &s,
            "/sites/s1/lists/L/contentTypes/0x0101",
            json!({"id":"0x0101"}),
        )
        .await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/lists/L/contentTypes/addCopy"))
            .and(body_json(json!({"contentType":"0x0100AB"})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"0x0100AB"})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/sites/s1/lists/L/contentTypes/0x0100AB"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let ct = c.sites().by_id("s1").list("L").content_types();
        assert_eq!(ct.list().await.unwrap()[0].id, "0x01");
        assert_eq!(ct.by_id("0x0101").get().await.unwrap().id, "0x0101");
        assert_eq!(ct.add("0x0100AB").await.unwrap().id, "0x0100AB");
        ct.by_id("0x0100AB").delete().await.unwrap();
    }

    #[tokio::test]
    async fn delta_follows_pages_then_resumes_from_token() {
        let (c, s) = client().await;
        let next = format!("{}/sites/s1/lists/L/items/delta?page=2", s.uri());
        let done = format!("{}/sites/s1/lists/L/items/delta?token=T1", s.uri());
        Mock::given(path("/sites/s1/lists/L/items/delta"))
            .and(query_param("page", "2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"2"}],"@odata.deltaLink":done})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/sites/s1/lists/L/items/delta"))
            .and(query_param("token", "T1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"value":[{"id":"3"}],"@odata.deltaLink":format!("{}/sites/s1/lists/L/items/delta?token=T2", s.uri())}),
            ))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/sites/s1/lists/L/items/delta"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"1"}],"@odata.nextLink":next})),
            )
            .up_to_n_times(1)
            .expect(1)
            .mount(&s)
            .await;
        let items = c.sites().by_id("s1").list("L").items();
        let first = items.delta(None).await.unwrap();
        assert_eq!(
            first
                .items
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>(),
            vec!["1", "2"]
        );
        assert_eq!(first.next_token.as_deref(), Some("T1"));
        let second = items.delta(first.next_token.as_deref()).await.unwrap();
        assert_eq!(
            (second.items.len(), second.next_token.as_deref()),
            (1, Some("T2"))
        );
    }

    #[tokio::test]
    async fn item_versions_page() {
        let (c, s) = client().await;
        let next = format!("{}/sites/s1/lists/L/items/7/versions?p=2", s.uri());
        Mock::given(path("/sites/s1/lists/L/items/7/versions"))
            .and(query_param("p", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"1.0"}]})))
            .mount(&s)
            .await;
        Mock::given(path("/sites/s1/lists/L/items/7/versions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"2.0"}],"@odata.nextLink":next})),
            )
            .mount(&s)
            .await;
        let v = c
            .sites()
            .by_id("s1")
            .list("L")
            .item("7")
            .versions()
            .await
            .unwrap();
        assert_eq!(
            v.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
            vec!["2.0", "1.0"]
        );
    }

    #[tokio::test]
    async fn subscriptions_use_list_resource_path() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/subscriptions"))
            .and(body_json(json!({
                "changeType":"updated","notificationUrl":"https://hook",
                "resource":"sites/s1/lists/L","expirationDateTime":"2030-01-01T00:00:00Z"
            })))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!({"id":"sub1","resource":"sites/s1/lists/L"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        get(&s, "/subscriptions", json!({"value":[
            {"id":"sub1","resource":"sites/s1/lists/L"},{"id":"other","resource":"sites/s1/lists/M"}
        ]})).await;
        Mock::given(method("PATCH"))
            .and(path("/subscriptions/sub1"))
            .and(body_json(
                json!({"expirationDateTime":"2031-01-01T00:00:00Z"}),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"id":"sub1","expirationDateTime":"2031-01-01T00:00:00Z"}),
                ),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/subscriptions/sub1"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let subs = c.sites().by_id("s1").list("L").subscriptions();
        let made = subs
            .create(&NewSubscription::new(
                "https://hook",
                "2030-01-01T00:00:00Z",
            ))
            .await
            .unwrap();
        assert_eq!(made.id, "sub1");
        let mine = subs.list().await.unwrap();
        assert_eq!(mine.len(), 1);
        let up = subs
            .update(
                "sub1",
                &json!({"expirationDateTime":"2031-01-01T00:00:00Z"}),
            )
            .await
            .unwrap();
        assert_eq!(
            up.expiration_date_time.as_deref(),
            Some("2031-01-01T00:00:00Z")
        );
        subs.delete("sub1").await.unwrap();
    }

    #[tokio::test]
    async fn typed_fields_are_accepted_as_item_body() {
        use crate::fields::{FieldValue, Fields};
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/sites/s1/lists/L/items"))
            .and(body_json(
                json!({"fields":{"Title":"a","ProjectLookupId":"3"}}),
            ))
            .respond_with(ResponseTemplate::new(201).set_body_json(
                json!({"id":"1","fields":{"ProjectLookupId":"3","Project":"Apollo"}}),
            ))
            .expect(1)
            .mount(&s)
            .await;
        let items = c.sites().by_id("s1").list("L").items();
        let f = Fields::new().text("Title", "a").lookup("Project", 3);
        let item = items.create(&f.for_graph()).await.unwrap();
        let l = item.lookup("Project").unwrap();
        assert_eq!((l.id, l.value.as_deref()), (3, Some("Apollo")));
        assert_eq!(item.text("Missing"), None);
        let bad = Fields::new().push(
            "D",
            FieldValue::Taxonomy {
                label: "a".into(),
                term_guid: "g".into(),
            },
        );
        assert!(items.create(&bad.for_graph()).await.is_err());
    }

    #[tokio::test]
    async fn drive_item_analytics_activities_restore() {
        let (c, s) = client().await;
        let p = "/sites/s1/lists/L/items/7";
        get(
            &s,
            &format!("{p}/driveItem"),
            json!({"id":"d1","name":"a.docx"}),
        )
        .await;
        get(
            &s,
            &format!("{p}/analytics"),
            json!({"allTime":{"access":{"actionCount":9}}}),
        )
        .await;
        let next = format!("{}{p}/activities?p=2", s.uri());
        Mock::given(path(format!("{p}/activities")))
            .and(query_param("p", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"b"}]})))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path(format!("{p}/activities")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"id":"a"}],"@odata.nextLink":next})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("{p}/versions/1.0/restoreVersion")))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let item = c.sites().by_id("s1").list("L").item("7");
        assert_eq!(
            item.drive_item().get().await.unwrap().name.as_deref(),
            Some("a.docx")
        );
        assert_eq!(
            item.analytics()
                .await
                .unwrap()
                .all_time
                .unwrap()
                .access
                .unwrap()
                .action_count,
            Some(9)
        );
        assert_eq!(item.activities().await.unwrap().len(), 2);
        let unit: () = item.restore_version("1.0").await.unwrap();
        assert_eq!(unit, ());
    }

    #[tokio::test]
    async fn update_if_match_sends_etag_and_reports_412() {
        let (c, s) = client().await;
        Mock::given(method("PATCH"))
            .and(path("/sites/s1/lists/L/items/7/fields"))
            .and(header("if-match", "\"x\""))
            .and(body_json(json!({"Title":"n"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Title":"n"})))
            .up_to_n_times(1)
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/sites/s1/lists/L/items/7/fields"))
            .and(header("if-match", "\"old\""))
            .respond_with(
                ResponseTemplate::new(412).set_body_json(
                    json!({"error":{"code":"preconditionFailed","message":"changed"}}),
                ),
            )
            .expect(1)
            .mount(&s)
            .await;
        let item = c.sites().by_id("s1").list("L").item("7");
        assert_eq!(
            item.update_if_match(&json!({"Title":"n"}), "\"x\"")
                .await
                .unwrap()["Title"],
            "n"
        );
        let e = item
            .update_if_match(&json!({"Title":"n"}), "\"old\"")
            .await
            .unwrap_err();
        assert!(matches!(e, Error::Http { status: 412, .. }));
    }

    #[tokio::test]
    async fn filter_unindexed_sends_filter_and_prefer_header() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/sites/s1/lists/L/items"))
            .and(query_param("$filter", "fields/Status eq 'Open'"))
            .and(header(
                "prefer",
                "HonorNonIndexedQueriesWarningMayFailRandomly",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"1"}]})))
            .expect(1)
            .mount(&s)
            .await;
        let items = c
            .sites()
            .by_id("s1")
            .list("L")
            .items()
            .filter_unindexed("fields/Status eq 'Open'")
            .list()
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn plain_filter_sends_no_prefer_header() {
        let (c, s) = client().await;
        Mock::given(path("/sites/s1/lists/L/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[]})))
            .mount(&s)
            .await;
        c.sites()
            .by_id("s1")
            .list("L")
            .items()
            .filter("a eq 1")
            .list()
            .await
            .unwrap();
        let reqs = s.received_requests().await.unwrap();
        assert!(reqs[0].headers.get("prefer").is_none());
    }
}
