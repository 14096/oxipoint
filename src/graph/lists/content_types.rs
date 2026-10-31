use crate::{
    Result,
    graph::*,
    http::{Body, RawRequest, encode_path},
};
use reqwest::Method;
use serde_json::json;

collection!(
    /// A list's content types.
    ListContentTypesRequest => ContentType
);

impl ListContentTypesRequest {
    /// A content type by id.
    pub fn by_id(&self, id: &str) -> ListContentTypeRequest {
        ListContentTypeRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }

    /// Adds a copy of a site content type to the list (`addCopy`).
    ///
    /// `content_type` is the site content type reference Graph expects, such as
    /// `https://graph.microsoft.com/v1.0/sites/<site>/contentTypes/<id>`, or its id.
    pub async fn add(&self, content_type: &str) -> Result<ContentType> {
        let mut req = RawRequest::new(Method::POST, format!("{}/addCopy", self.base));
        req.body = Body::Json(json!({ "contentType": content_type }));
        self.t.send(&req).await?.json()
    }
}

entity!(
    /// One list content type.
    ListContentTypeRequest => ContentType
);
