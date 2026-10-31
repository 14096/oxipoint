use serde::Serialize;

use super::name_literal;
use crate::{
    Result,
    http::Body,
    sharepoint::{
        Field,
        web::{json_body, post},
    },
};

collection!(
    /// The fields (columns) of a list.
    ListFieldsRequest => Field
);

impl ListFieldsRequest {
    /// A field by GUID.
    pub fn by_id(&self, id: &str) -> ListFieldRequest {
        self.entity("getbyid", id)
    }

    /// A field by display title.
    pub fn by_title(&self, title: &str) -> ListFieldRequest {
        self.entity("getbytitle", title)
    }

    /// A field by internal name (or title), e.g. `Status`.
    pub fn by_internal_name(&self, name: &str) -> ListFieldRequest {
        self.entity("getbyinternalnameortitle", name)
    }

    fn entity(&self, func: &str, arg: &str) -> ListFieldRequest {
        ListFieldRequest::new(
            self.inner.clone(),
            format!("{}/{func}({})", self.base, name_literal(arg)),
        )
    }

    /// Creates a field from its definition.
    pub async fn create<F: Serialize>(&self, definition: &F) -> Result<Field> {
        post(&self.inner, self.base.clone(), json_body(definition)?).await
    }
}

entity!(
    /// One field of a list.
    ListFieldRequest => Field
);

impl ListFieldRequest {
    /// Updates the given properties (MERGE).
    pub async fn update<F: Serialize>(&self, properties: &F) -> Result<()> {
        self.inner
            .tunnel("MERGE", self.base.clone(), json_body(properties)?)
            .await?;
        Ok(())
    }

    /// Deletes the field.
    pub async fn delete(&self) -> Result<()> {
        self.inner
            .tunnel("DELETE", self.base.clone(), Body::None)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::sharepoint::testutil::{client, mock_get, mock_write, mount_digest};
    use serde_json::json;

    const P: &str = "/_api/web/lists/getbytitle('T')/fields";

    #[tokio::test]
    async fn lookups_by_id_title_and_internal_name() {
        let (c, s) = client().await;
        mock_get(&s, P, json!({"value":[{"Id":"a"}]})).await;
        mock_get(&s, &format!("{P}/getbyid('g1')"), json!({"Id":"g1"})).await;
        mock_get(
            &s,
            &format!("{P}/getbytitle('Due%20Date')"),
            json!({"Id":"g2"}),
        )
        .await;
        mock_get(
            &s,
            &format!("{P}/getbyinternalnameortitle('Status')"),
            json!({"Id":"g3","InternalName":"Status"}),
        )
        .await;
        let f = c.list_by_title("T").fields();
        assert_eq!(f.list().await.unwrap().len(), 1);
        assert_eq!(f.by_id("g1").get().await.unwrap().id, "g1");
        assert_eq!(f.by_title("Due Date").get().await.unwrap().id, "g2");
        let st = f.by_internal_name("Status").get().await.unwrap();
        assert_eq!(st.internal_name.as_deref(), Some("Status"));
    }

    #[tokio::test]
    async fn create_update_delete() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_write(
            &s,
            P,
            None,
            Some(json!({"Title":"X","FieldTypeKind":2})),
            json!({"Id":"n"}),
        )
        .await;
        mock_write(
            &s,
            &format!("{P}/getbyid('n')"),
            Some("MERGE"),
            Some(json!({"Title":"Y"})),
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
        let f = c.list_by_title("T").fields();
        assert_eq!(
            f.create(&json!({"Title":"X","FieldTypeKind":2}))
                .await
                .unwrap()
                .id,
            "n"
        );
        f.by_id("n").update(&json!({"Title":"Y"})).await.unwrap();
        f.by_id("n").delete().await.unwrap();
    }
}
