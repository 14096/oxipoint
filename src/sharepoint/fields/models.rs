use serde::{Deserialize, Serialize};

/// A field (column).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Field {
    /// Field GUID.
    #[serde(default)]
    pub id: String,
    /// Display title.
    #[serde(default)]
    pub title: Option<String>,
    /// Internal name.
    #[serde(default)]
    pub internal_name: Option<String>,
    /// Field type such as `Text`.
    #[serde(default)]
    pub type_as_string: Option<String>,
    /// Group.
    #[serde(default)]
    pub group: Option<String>,
    /// Whether a value is required.
    #[serde(default)]
    pub required: Option<bool>,
    /// Whether the field is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
}
