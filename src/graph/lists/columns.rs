use crate::{
    Result,
    graph::*,
    http::{Body, RawRequest, encode_path},
};
use reqwest::Method;
use serde::Serialize;

collection!(
    /// A list's columns.
    ListColumnsRequest => Column
);
creatable!(ListColumnsRequest => Column);

impl ListColumnsRequest {
    /// A column by id.
    pub fn by_id(&self, id: &str) -> ListColumnRequest {
        ListColumnRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One list column.
    ListColumnRequest => Column
);

impl ListColumnRequest {
    /// Updates the column (PATCH) and returns it.
    pub async fn update<F: Serialize>(&self, changes: &F) -> Result<Column> {
        let mut req = RawRequest::new(Method::PATCH, self.base.as_str());
        req.body = Body::Json(serde_json::to_value(changes)?);
        self.t.send(&req).await?.json()
    }
}
