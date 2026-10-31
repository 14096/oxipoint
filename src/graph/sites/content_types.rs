use crate::{graph::*, http::encode_path};

collection!(
    /// A site's content types.
    ContentTypesRequest => ContentType
);

creatable!(ContentTypesRequest => ContentType);

impl ContentTypesRequest {
    /// A content type by id.
    pub fn by_id(&self, id: &str) -> ContentTypeRequest {
        ContentTypeRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One site content type.
    ContentTypeRequest => ContentType
);
