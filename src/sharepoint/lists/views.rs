use serde::Serialize;
use serde_json::Value;

use super::name_literal;
use crate::{
    Result,
    http::Body,
    sharepoint::{
        View,
        web::{get_req, json_body, post},
    },
};

collection!(
    /// The views of a list.
    ViewsRequest => View
);

impl ViewsRequest {
    /// A view by GUID.
    pub fn by_id(&self, id: &str) -> ViewRequest {
        ViewRequest::new(
            self.inner.clone(),
            format!("{}/getbyid({})", self.base, name_literal(id)),
        )
    }

    /// A view by title.
    pub fn by_title(&self, title: &str) -> ViewRequest {
        ViewRequest::new(
            self.inner.clone(),
            format!("{}/getbytitle({})", self.base, name_literal(title)),
        )
    }

    /// Creates a view from its definition.
    pub async fn create<F: Serialize>(&self, definition: &F) -> Result<View> {
        post(&self.inner, self.base.clone(), json_body(definition)?).await
    }
}

entity!(
    /// One view of a list.
    ViewRequest => View
);

impl ViewRequest {
    /// Updates the given properties (MERGE).
    pub async fn update<F: Serialize>(&self, properties: &F) -> Result<()> {
        self.inner
            .tunnel("MERGE", self.base.clone(), json_body(properties)?)
            .await?;
        Ok(())
    }

    /// Deletes the view.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }

    /// The internal names of the fields shown by the view.
    pub async fn fields(&self) -> Result<Vec<String>> {
        let doc: Value = self
            .inner
            .t
            .send(&get_req(&format!("{}/viewfields", self.base), &self.query))
            .await?
            .json()?;
        let items = doc
            .get("Items")
            .or_else(|| doc.pointer("/d/Items"))
            .map(|i| i.get("results").unwrap_or(i));
        Ok(items
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use crate::sharepoint::testutil::{client, mock_get, mock_write, mount_digest};
    use serde_json::json;

    const P: &str = "/_api/web/lists/getbytitle('T')/views";

    #[tokio::test]
    async fn list_get_by_id_and_title_escaped() {
        let (c, s) = client().await;
        mock_get(&s, P, json!({"value":[{"Id":"v"}]})).await;
        mock_get(&s, &format!("{P}/getbyid('v1')"), json!({"Id":"v1"})).await;
        mock_get(
            &s,
            &format!("{P}/getbytitle('All%20Items')"),
            json!({"Id":"v2","Title":"All Items"}),
        )
        .await;
        let v = c.list_by_title("T").views();
        assert_eq!(v.list().await.unwrap().len(), 1);
        assert_eq!(v.by_id("v1").get().await.unwrap().id, "v1");
        assert_eq!(
            v.by_title("All Items")
                .get()
                .await
                .unwrap()
                .title
                .as_deref(),
            Some("All Items")
        );
    }

    #[tokio::test]
    async fn create_update_delete_and_view_fields() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            P,
            None,
            Some(json!({"Title":"Mine"})),
            json!({"Id":"n","Title":"Mine"}),
        )
        .await;
        mock_write(
            &s,
            &format!("{P}/getbyid('n')"),
            Some("MERGE"),
            Some(json!({"RowLimit":10})),
            json!(null),
        )
        .await;
        mock_write(
            &s,
            &format!("{P}/getbyid('n')"),
            Some("DELETE"),
            None,
            json!(null),
        )
        .await;
        mock_get(
            &s,
            &format!("{P}/getbyid('n')/viewfields"),
            json!({"Items":["Title","Status"]}),
        )
        .await;
        let v = c.list_by_title("T").views();
        assert_eq!(v.create(&json!({"Title":"Mine"})).await.unwrap().id, "n");
        v.by_id("n").update(&json!({"RowLimit":10})).await.unwrap();
        assert_eq!(
            v.by_id("n").fields().await.unwrap(),
            vec!["Title", "Status"]
        );
        v.by_id("n").delete().await.unwrap();
    }
}
