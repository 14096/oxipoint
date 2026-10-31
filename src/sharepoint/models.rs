use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A SharePoint web (site).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Web {
    /// Web GUID (empty when excluded by `$select`).
    #[serde(default)]
    pub id: String,
    /// Title.
    #[serde(default)]
    pub title: Option<String>,
    /// Absolute URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Server-relative URL.
    #[serde(default)]
    pub server_relative_url: Option<String>,
}

/// A list or document library.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ListInfo {
    /// List GUID (empty when excluded by `$select`).
    #[serde(default)]
    pub id: String,
    /// Title.
    #[serde(default)]
    pub title: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Number of items.
    #[serde(default)]
    pub item_count: Option<u64>,
}

/// A list item; columns other than `Id` are available in `fields`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ListItem {
    /// Item id (0 when excluded by `$select`).
    #[serde(rename = "Id", alias = "ID", default)]
    pub id: i64,
    /// All other returned columns.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

/// A folder.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FolderInfo {
    /// Name.
    pub name: String,
    /// Server-relative URL.
    pub server_relative_url: String,
    /// Number of children.
    #[serde(default)]
    pub item_count: Option<u64>,
}

/// A file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FileInfo {
    /// Name.
    pub name: String,
    /// Server-relative URL.
    pub server_relative_url: String,
    /// Size in bytes (SharePoint returns it as a string).
    #[serde(default)]
    pub length: Option<String>,
}
