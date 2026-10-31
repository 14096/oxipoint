use serde::Serialize;

use super::{id_literal, json_body, post};
use crate::{
    Result,
    http::Body,
    sharepoint::{ContentType, Field},
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

collection!(
    /// A collection of content types.
    ContentTypesRequest => ContentType
);

impl ContentTypesRequest {
    /// A content type by id such as `0x0101`.
    pub fn by_id(&self, id: &str) -> ContentTypeRequest {
        ContentTypeRequest::new(
            self.inner.clone(),
            format!("{}({})", self.base, id_literal(id)),
        )
    }

    /// Creates a content type from its definition.
    pub async fn create<F: Serialize>(&self, definition: &F) -> Result<ContentType> {
        post(&self.inner, self.base.clone(), json_body(definition)?).await
    }
}

entity!(
    /// One content type.
    ContentTypeRequest => ContentType
);

impl ContentTypeRequest {
    /// Deletes the content type.
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

    #[tokio::test]
    async fn content_type_by_id_is_escaped() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_get(
            &s,
            "/_api/web/contenttypes",
            json!({"value":[{"Id":{"StringValue":"0x01"}}]}),
        )
        .await;
        mock_get(
            &s,
            "/_api/web/contenttypes('0x0101')",
            json!({"Id":{"StringValue":"0x0101"},"Name":"Document"}),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/contenttypes",
            None,
            Some(json!({"Name":"N"})),
            json!({"Id":{"StringValue":"0x0100AB"}}),
        )
        .await;
        mock_write(
            &s,
            "/_api/web/contenttypes('0x0100AB')",
            Some("DELETE"),
            None,
            json!(null),
        )
        .await;
        let ct = c.web().content_types();
        assert_eq!(ct.list().await.unwrap()[0].id, "0x01");
        assert_eq!(ct.by_id("0x0101").get().await.unwrap().id, "0x0101");
        assert_eq!(
            ct.create(&json!({"Name":"N"})).await.unwrap().id,
            "0x0100AB"
        );
        ct.by_id("0x0100AB").delete().await.unwrap();
    }
}
