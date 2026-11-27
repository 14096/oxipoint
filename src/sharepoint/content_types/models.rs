use serde::{Deserialize, Serialize};

use crate::sharepoint::inner::content_type_id;

/// A content type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ContentType {
    /// Content type id such as `0x0101`.
    #[serde(default, deserialize_with = "content_type_id")]
    pub id: String,
    /// Name.
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Group.
    pub group: Option<String>,
    /// Whether the content type is hidden.
    pub hidden: Option<bool>,
}
