use serde_json::json;

use crate::sharepoint::inner::name_literal;
use crate::{
    Result,
    http::Body,
    sharepoint::{ContentType, inner::post},
};

collection!(
    /// The content types of a list.
    ListContentTypesRequest => ContentType
);

impl ListContentTypesRequest {
    /// A content type by id such as `0x0101`.
    pub fn by_id(&self, id: &str) -> ListContentTypeRequest {
        ListContentTypeRequest::new(
            self.inner.clone(),
            format!("{}({})", self.base, name_literal(id)),
        )
    }

    /// Adds one of the web's content types to the list.
    pub async fn add_from_web(&self, content_type_id: &str) -> Result<ContentType> {
        post(
            &self.inner,
            format!("{}/addavailablecontenttype", self.base),
            Body::Json(json!({ "contentTypeId": content_type_id })),
        )
        .await
    }
}

entity!(
    /// One content type of a list.
    ListContentTypeRequest => ContentType
);

impl ListContentTypeRequest {
    /// Removes the content type from the list.
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

    const P: &str = "/_api/web/lists/getbytitle('T')/contenttypes";

    #[tokio::test]
    async fn list_get_add_delete() {
        let (c, s) = client().await;
        mount_digest(&s, 1).await;
        mock_get(&s, P, json!({"value":[{"Id":{"StringValue":"0x01"}}]})).await;
        mock_get(
            &s,
            &format!("{P}('0x0101')"),
            json!({"Id":{"StringValue":"0x0101"}}),
        )
        .await;
        mock_write(
            &s,
            &format!("{P}/addavailablecontenttype"),
            None,
            Some(json!({"contentTypeId":"0x0100AB"})),
            json!({"Id":{"StringValue":"0x0100AB"}}),
        )
        .await;
        mock_write(
            &s,
            &format!("{P}('0x0100AB')"),
            Some("DELETE"),
            None,
            json!(null),
        )
        .await;
        let ct = c.list_by_title("T").content_types();
        assert_eq!(ct.list().await.unwrap()[0].id, "0x01");
        assert_eq!(ct.by_id("0x0101").get().await.unwrap().id, "0x0101");
        assert_eq!(ct.add_from_web("0x0100AB").await.unwrap().id, "0x0100AB");
        ct.by_id("0x0100AB").delete().await.unwrap();
    }
}
