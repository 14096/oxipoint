use serde::{Deserialize, Serialize};

/// A field (column).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Field {
    /// Field GUID.
    #[serde(default)]
    pub id: String,
    /// Display title.
    pub title: Option<String>,
    /// Internal name.
    pub internal_name: Option<String>,
    /// Field type such as `Text`.
    pub type_as_string: Option<String>,
    /// Group.
    pub group: Option<String>,
    /// Whether a value is required.
    pub required: Option<bool>,
    /// Whether the field is hidden.
    pub hidden: Option<bool>,
}
