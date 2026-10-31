mod models;

pub use models::Field;

use serde::Serialize;

use crate::{
    Result,
    http::Body,
    sharepoint::inner::{id_literal, json_body, post},
};

collection!(
    /// A collection of fields (columns).
    FieldsRequest => Field
);

impl FieldsRequest {
    /// A field by GUID.
    pub fn by_id(&self, id: &str) -> FieldRequest {
        FieldRequest::new(
            self.inner.clone(),
            format!("{}/getbyid({})", self.base, id_literal(id)),
        )
    }

    /// Creates a field from its definition.
    pub async fn create<F: Serialize>(&self, definition: &F) -> Result<Field> {
        post(&self.inner, self.base.clone(), json_body(definition)?).await
    }
}

entity!(
    /// One field (column).
    FieldRequest => Field
);

impl FieldRequest {
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

    #[tokio::test]
    async fn fields_list_get_create_delete() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_get(&s, "/_api/web/fields", json!({"value":[{"Id":"a"}]})).await;
        mock_get(
            &s,
            "/_api/web/fields/getbyid('a%27%27b')",
            json!({"Id":"a'b"}),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/fields",
            None,
            Some(json!({"Title":"X","FieldTypeKind":2})),
            json!({"Id":"n","Title":"X"}),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/fields/getbyid('n')",
            Some("DELETE"),
            None,
            json!(null),
        )
        .await;
        let f = c.web().fields();
        assert_eq!(f.list().await.unwrap()[0].id, "a");
        assert_eq!(f.by_id("a'b").get().await.unwrap().id, "a'b");
        assert_eq!(
            f.create(&json!({"Title":"X","FieldTypeKind":2}))
                .await
                .unwrap()
                .id,
            "n"
        );
        f.by_id("n").delete().await.unwrap();
    }
}
