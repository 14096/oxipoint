use serde_json::json;

use super::{id_literal, post_action};
use crate::{
    Result,
    http::Body,
    sharepoint::{Feature, RecycleBinItem},
};

collection!(
    /// The activated features of the web.
    FeaturesRequest => Feature
);

impl FeaturesRequest {
    /// Activates a feature by definition id.
    pub async fn activate(&self, feature_id: &str) -> Result<()> {
        post_action(
            &self.inner,
            format!("{}/add", self.base),
            Body::Json(json!({ "featureId": feature_id, "force": false })),
        )
        .await
    }

    /// Deactivates a feature by definition id.
    pub async fn deactivate(&self, feature_id: &str) -> Result<()> {
        post_action(
            &self.inner,
            format!("{}/remove", self.base),
            Body::Json(json!({ "featureId": feature_id, "force": false })),
        )
        .await
    }
}

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
