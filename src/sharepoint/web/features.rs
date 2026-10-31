use serde_json::json;

use crate::sharepoint::inner::post_action;
use crate::{Result, http::Body, sharepoint::Feature};

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
