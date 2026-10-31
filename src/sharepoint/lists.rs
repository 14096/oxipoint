use std::sync::Arc;

use reqwest::Method;
use serde::Serialize;

use super::{Inner, ListInfo, ListItem};
use crate::{
    Result,
    http::{Body, Pager, QueryOptions, RawRequest, query::query_methods},
};

fn get_req(path: &str, q: &QueryOptions) -> RawRequest {
    let mut r = RawRequest::new(Method::GET, path);
    r.query = q.pairs().to_vec();
    r
}

/// `/web/lists` — all lists.
#[derive(Debug, Clone)]
pub struct ListsRequest {
    inner: Arc<Inner>,
    query: QueryOptions,
}

impl ListsRequest {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            inner,
            query: QueryOptions::new(),
        }
    }

    /// Lists all lists, following paging links.
    pub async fn list(&self) -> Result<Vec<ListInfo>> {
        Pager::new(self.inner.t.clone(), get_req("/web/lists", &self.query))
            .collect_all()
            .await
    }
}
query_methods!(ListsRequest);

/// One list.
#[derive(Debug, Clone)]
pub struct ListRequest {
    inner: Arc<Inner>,
    base: String,
    query: QueryOptions,
}

impl ListRequest {
    pub(crate) fn new(inner: Arc<Inner>, base: String) -> Self {
        Self {
            inner,
            base,
            query: QueryOptions::new(),
        }
    }

    /// Fetches list properties.
    pub async fn get(&self) -> Result<ListInfo> {
        self.inner
            .t
            .send(&get_req(&self.base, &self.query))
            .await?
            .json()
    }

    /// The list's items.
    pub fn items(&self) -> ItemsRequest {
        ItemsRequest {
            inner: self.inner.clone(),
            base: format!("{}/items", self.base),
            query: QueryOptions::new(),
        }
    }

    /// One item by numeric id.
    pub fn item(&self, id: i64) -> ItemRequest {
        ItemRequest {
            inner: self.inner.clone(),
            base: format!("{}/items({id})", self.base),
            query: QueryOptions::new(),
        }
    }
}
query_methods!(ListRequest);

/// A list's items collection.
#[derive(Debug, Clone)]
pub struct ItemsRequest {
    inner: Arc<Inner>,
    base: String,
    query: QueryOptions,
}

impl ItemsRequest {
    /// Lists items, following paging links.
    pub async fn list(&self) -> Result<Vec<ListItem>> {
        self.pages().collect_all().await
    }

    /// Returns a pager.
    pub fn pages(&self) -> Pager<ListItem> {
        Pager::new(self.inner.t.clone(), get_req(&self.base, &self.query))
    }

    /// Creates an item from column values.
    pub async fn create<F: Serialize>(&self, fields: &F) -> Result<ListItem> {
        let mut req = RawRequest::new(Method::POST, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(fields)?);
        self.inner.write(req).await?.json()
    }
}
query_methods!(ItemsRequest);

/// One list item.
#[derive(Debug, Clone)]
pub struct ItemRequest {
    inner: Arc<Inner>,
    base: String,
    query: QueryOptions,
}

impl ItemRequest {
    /// Fetches the item.
    pub async fn get(&self) -> Result<ListItem> {
        self.inner
            .t
            .send(&get_req(&self.base, &self.query))
            .await?
            .json()
    }

    /// Updates the given columns (MERGE).
    pub async fn update<F: Serialize>(&self, fields: &F) -> Result<()> {
        self.inner
            .tunnel(
                "MERGE",
                self.base.clone(),
                Body::Json(serde_json::to_value(fields)?),
            )
            .await?;
        Ok(())
    }

    /// Deletes the item.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }
}
query_methods!(ItemRequest);

#[cfg(test)]
mod tests {
    use crate::{
        Error,
        sharepoint::testutil::{client, mount_digest},
    };
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, header, method, path, query_param},
    };

    #[tokio::test]
    async fn lists_and_get_by_title_and_id() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web/lists"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"Id":"a","Title":"Docs","ItemCount":3}]})),
            )
            .mount(&s)
            .await;
        Mock::given(path("/_api/web/lists/getbytitle('O%27%27Brien')"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"Id":"b","Title":"O'Brien"})),
            )
            .mount(&s)
            .await;
        Mock::given(path("/_api/web/lists(guid'1234')"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Id":"1234"})))
            .mount(&s)
            .await;
        assert_eq!(c.lists().list().await.unwrap()[0].item_count, Some(3));
        assert_eq!(c.list_by_title("O'Brien").get().await.unwrap().id, "b");
        assert_eq!(c.list_by_id("1234").get().await.unwrap().id, "1234");
    }

    #[tokio::test]
    async fn select_without_id_still_deserializes() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web/lists"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"value":[{"Title":"Docs","ItemCount":3}]})),
            )
            .mount(&s)
            .await;
        let l = c
            .lists()
            .select(["Title", "ItemCount"])
            .list()
            .await
            .unwrap();
        assert_eq!(
            (l[0].id.as_str(), l[0].title.as_deref()),
            ("", Some("Docs"))
        );
    }

    #[tokio::test]
    async fn filtered_query_is_encoded() {
        let (c, s) = client().await;
        Mock::given(path("/_api/web/lists/getbytitle('T')/items"))
            .and(query_param("$filter", "Status eq 'Open'"))
            .and(query_param("$select", "Id,Title"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"value":[{"Id":1,"Title":"x"}]})),
            )
            .mount(&s)
            .await;
        let items = c
            .list_by_title("T")
            .items()
            .filter("Status eq 'Open'")
            .select(["Id", "Title"])
            .list()
            .await
            .unwrap();
        assert_eq!(
            (items[0].id, items[0].fields["Title"].clone()),
            (1, json!("x"))
        );
    }

    #[tokio::test]
    async fn update_uses_merge_headers() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web/lists/getbytitle('T')/items(4)"))
            .and(header("x-http-method", "MERGE"))
            .and(header("if-match", "*"))
            .and(header("x-requestdigest", "DIGEST"))
            .and(body_json(json!({"Title":"new"})))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        c.list_by_title("T")
            .item(4)
            .update(&json!({"Title":"new"}))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn delete_uses_tunnel_and_reports_errors() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web/lists/getbytitle('T')/items(9)"))
            .and(header("x-http-method", "DELETE"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({"odata.error":{"code":"-2147024809","message":{"lang":"en","value":"missing"}}})))
            .mount(&s)
            .await;
        let e = c.list_by_title("T").item(9).delete().await.unwrap_err();
        assert!(matches!(e, Error::Http { status: 404, ref message, .. } if message == "missing"));
    }
}
