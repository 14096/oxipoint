use crate::{
    Result,
    http::Body,
    sharepoint::{Comment, inner::post},
};
use serde_json::json;

collection!(
    /// The comments of an item.
    CommentsRequest => Comment
);

impl CommentsRequest {
    /// Adds a comment and returns it.
    pub async fn add(&self, text: &str) -> Result<Comment> {
        post(
            &self.inner,
            self.base.clone(),
            Body::Json(json!({ "text": text })),
        )
        .await
    }

    /// Deletes a comment by id.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.inner
            .tunnel(
                "DELETE",
                format!("{}({})", self.base, crate::http::encode_path(id)),
                Body::None,
            )
            .await?;
        Ok(())
    }
}
