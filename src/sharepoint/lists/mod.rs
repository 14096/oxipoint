use std::sync::Arc;

use serde::Serialize;

use super::{Change, ChangeQuery, FolderInfo, Inner, ListInfo, PermissionsRequest, odata_str};
use crate::{
    Result,
    http::{Body, Pager, QueryOptions, query::query_methods},
    sharepoint::web::{get_req, json_body, post, post_action},
};
use serde_json::Value;

mod content_types;
mod fields;
mod items;
mod subscriptions;
mod views;

pub use content_types::{ListContentTypeRequest, ListContentTypesRequest};
pub use fields::{ListFieldRequest, ListFieldsRequest};
pub use items::{
    AttachmentRequest, AttachmentsRequest, CommentsRequest, ItemRequest, ItemsRequest,
};
pub use subscriptions::SubscriptionsRequest;
pub use views::{ViewRequest, ViewsRequest};

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

    /// Creates a list (see [`NewList`](super::NewList)) and returns it.
    pub async fn create<F: Serialize>(&self, definition: &F) -> Result<ListInfo> {
        post(&self.inner, "/web/lists".to_owned(), json_body(definition)?).await
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

    /// Updates the given properties of the list (MERGE).
    pub async fn update<F: Serialize>(&self, properties: &F) -> Result<()> {
        self.inner
            .tunnel("MERGE", self.base.clone(), json_body(properties)?)
            .await?;
        Ok(())
    }

    /// Deletes the list permanently.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }

    /// Moves the list to the recycle bin.
    pub async fn recycle(&self) -> Result<()> {
        post_action(&self.inner, format!("{}/recycle", self.base), Body::None).await
    }

    /// The list's root folder.
    pub async fn root_folder(&self) -> Result<FolderInfo> {
        self.inner
            .t
            .send(&get_req(&format!("{}/rootfolder", self.base), &self.query))
            .await?
            .json()
    }

    /// The list's forms.
    pub async fn forms(&self) -> Result<Vec<Value>> {
        Pager::new(
            self.inner.t.clone(),
            get_req(&format!("{}/forms", self.base), &self.query),
        )
        .collect_all()
        .await
    }

    /// Reads the changes selected by `query` (`GetChanges`).
    pub async fn get_changes(&self, query: &ChangeQuery) -> Result<Vec<Change>> {
        let doc: Value = post(
            &self.inner,
            format!("{}/getchanges", self.base),
            Body::Json(query.to_body()),
        )
        .await?;
        let items = doc
            .get("value")
            .or_else(|| doc.pointer("/d/results"))
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        Ok(serde_json::from_value(items)?)
    }

    /// The list's items.
    pub fn items(&self) -> ItemsRequest {
        ItemsRequest::new(
            self.inner.clone(),
            format!("{}/items", self.base),
            self.base.clone(),
        )
    }

    /// One item by numeric id.
    pub fn item(&self, id: i64) -> ItemRequest {
        ItemRequest::new(self.inner.clone(), format!("{}/items({id})", self.base))
    }

    /// The list's fields.
    pub fn fields(&self) -> ListFieldsRequest {
        ListFieldsRequest::new(self.inner.clone(), format!("{}/fields", self.base))
    }

    /// The list's views.
    pub fn views(&self) -> ViewsRequest {
        ViewsRequest::new(self.inner.clone(), format!("{}/views", self.base))
    }

    /// The list's content types.
    pub fn content_types(&self) -> ListContentTypesRequest {
        ListContentTypesRequest::new(self.inner.clone(), format!("{}/contenttypes", self.base))
    }

    /// The list's webhook subscriptions.
    pub fn subscriptions(&self) -> SubscriptionsRequest {
        SubscriptionsRequest::new(self.inner.clone(), self.base.clone())
    }

    /// Role assignments and inheritance of the list.
    pub fn permissions(&self) -> PermissionsRequest {
        PermissionsRequest::new(self.inner.clone(), self.base.clone())
    }
}
query_methods!(ListRequest);

pub(crate) fn name_literal(s: &str) -> String {
    format!("'{}'", odata_str(s))
}

#[cfg(test)]
mod tests {
    use crate::sharepoint::{
        ListTemplate, NewList,
        testutil::{client, client_retrying, mock_get, mock_write, mount_digest},
    };
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{header, method, path},
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
    async fn create_list_posts_definition_with_digest() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            "/_api/web/lists",
            None,
            Some(json!({"Title":"Tasks","BaseTemplate":100})),
            json!({"Id":"n","Title":"Tasks","BaseTemplate":100}),
        )
        .await;
        let l = c
            .lists()
            .create(&NewList::new("Tasks", ListTemplate::GenericList))
            .await
            .unwrap();
        assert_eq!((l.id.as_str(), l.base_template), ("n", Some(100)));
    }

    #[tokio::test]
    async fn update_delete_recycle_list() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        let p = "/_api/web/lists/getbytitle('T')";
        mock_write(
            &s,
            p,
            Some("MERGE"),
            Some(json!({"Description":"d"})),
            json!(null),
        )
        .await;
        mock_write(&s, p, Some("DELETE"), None, json!(null)).await;
        mock_write(&s, &format!("{p}/recycle"), None, None, json!(null)).await;
        let l = c.list_by_title("T");
        l.update(&json!({"Description":"d"})).await.unwrap();
        l.delete().await.unwrap();
        l.recycle().await.unwrap();
    }

    #[tokio::test]
    async fn root_folder_and_forms() {
        let (c, s) = client().await;
        let p = "/_api/web/lists/getbytitle('T')";
        mock_get(
            &s,
            &format!("{p}/rootfolder"),
            json!({"Name":"T","ServerRelativeUrl":"/Lists/T"}),
        )
        .await;
        mock_get(
            &s,
            &format!("{p}/forms"),
            json!({"value":[{"ResourcePath":{"DecodedUrl":"/f"}}]}),
        )
        .await;
        let l = c.list_by_title("T");
        assert_eq!(
            l.root_folder().await.unwrap().server_relative_url,
            "/Lists/T"
        );
        assert_eq!(l.forms().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn throttled_list_write_is_retried_with_same_digest() {
        let (c, s) = client_retrying().await;
        mount_digest(&s, 1).await;
        Mock::given(method("POST"))
            .and(path("/_api/web/lists"))
            .and(header("x-requestdigest", "DIGEST"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .up_to_n_times(1)
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("POST"))
            .and(path("/_api/web/lists"))
            .and(header("x-requestdigest", "DIGEST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Id":"n"})))
            .expect(1)
            .mount(&s)
            .await;
        c.lists()
            .create(&NewList::new("T", ListTemplate::GenericList))
            .await
            .unwrap();
    }
}
