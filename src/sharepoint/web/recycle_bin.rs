use crate::sharepoint::inner::{id_literal, post_action};
use crate::{Result, http::Body, sharepoint::RecycleBinItem};
use serde_json::json;

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

    /// Only items in the first stage bin.
    pub fn first_stage(mut self) -> Self {
        self.query = self.query.filter("ItemState eq 1");
        self
    }

    /// Only items in the second stage bin.
    pub fn second_stage(mut self) -> Self {
        self.query = self.query.filter("ItemState eq 2");
        self
    }

    /// Restores several items by GUID in one call. Does nothing for an empty list.
    pub async fn restore_many(&self, ids: &[&str]) -> Result<()> {
        self.by_ids("restoreByIds", ids).await
    }

    /// Permanently deletes several items by GUID in one call. Does nothing for an empty list.
    pub async fn delete_many(&self, ids: &[&str]) -> Result<()> {
        self.by_ids("deleteByIds", ids).await
    }

    /// Moves several items to the second stage bin, one call per item. Does nothing for an
    /// empty list.
    pub async fn move_many_to_second_stage(&self, ids: &[&str]) -> Result<()> {
        for id in ids {
            self.item(id).move_to_second_stage().await?;
        }
        Ok(())
    }

    async fn by_ids(&self, action: &str, ids: &[&str]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        post_action(
            &self.inner,
            format!("{}/{action}", self.base),
            Body::Json(json!({ "ids": ids })),
        )
        .await
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

    /// Moves the item from the first stage to the second stage bin.
    pub async fn move_to_second_stage(&self) -> Result<()> {
        post_action(
            &self.inner,
            format!("{}/movetosecondstage", self.base),
            Body::None,
        )
        .await
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
