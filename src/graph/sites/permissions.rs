use crate::{
    Result,
    graph::*,
    http::{Body, RawRequest, encode_path},
};
use reqwest::Method;
use serde::Serialize;

collection!(
    /// A site's permissions.
    PermissionsRequest => Permission
);

creatable!(PermissionsRequest => Permission);

impl PermissionsRequest {
    /// A permission by id.
    pub fn by_id(&self, id: &str) -> PermissionRequest {
        PermissionRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One site permission.
    PermissionRequest => Permission
);

impl PermissionRequest {
    /// Updates the permission (for example its `roles`) and returns the result.
    pub async fn update<F: Serialize>(&self, changes: &F) -> Result<Permission> {
        let mut req = RawRequest::new(Method::PATCH, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(changes)?);
        self.t.send(&req).await?.json()
    }
}
