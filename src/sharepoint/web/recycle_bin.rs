use crate::sharepoint::inner::{id_literal, post_action};
use crate::{Result, http::Body, sharepoint::RecycleBinItem};

collection!(
    /// The recycle bin of the web.
    RecycleBinRequest => RecycleBinItem
);

impl RecycleBinRequest {
    /// An item by GUID.
    pub fn item(&self, id: &str) -> RecycleBinItemRequest {
        RecycleBinItemRequest::new(
            self.inner.clone(),
            format!("{}({})", self.base, id_literal(id)),
        )
    }

    /// Permanently deletes everything in the recycle bin.
    pub async fn empty(&self) -> Result<()> {
        post_action(&self.inner, format!("{}/deleteall", self.base), Body::None).await
    }
}

entity!(
    /// One recycle bin item.
    RecycleBinItemRequest => RecycleBinItem
);

impl RecycleBinItemRequest {
    /// Restores the item to its original location.
    pub async fn restore(&self) -> Result<()> {
        post_action(&self.inner, format!("{}/restore", self.base), Body::None).await
    }

    /// Deletes the item permanently.
    pub async fn delete(&self) -> Result<()> {
        post_action(
            &self.inner,
            format!("{}/deleteobject", self.base),
            Body::None,
        )
        .await
    }
}
