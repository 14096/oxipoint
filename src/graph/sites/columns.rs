use crate::{graph::*, http::encode_path};

collection!(
    /// A site's columns.
    ColumnsRequest => Column
);

creatable!(ColumnsRequest => Column);

impl ColumnsRequest {
    /// A column by id.
    pub fn by_id(&self, id: &str) -> ColumnRequest {
        ColumnRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One site column.
    ColumnRequest => Column
);
