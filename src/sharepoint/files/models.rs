use serde::{Deserialize, Serialize};

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
