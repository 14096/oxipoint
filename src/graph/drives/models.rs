use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A document library or drive.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// Drive id.
    pub id: String,
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Kind such as `documentLibrary`.
    #[serde(default)]
    pub drive_type: Option<String>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
}

/// A file or folder in a drive.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriveItem {
    /// Item id.
    pub id: String,
    /// File or folder name.
    #[serde(default)]
    pub name: Option<String>,
    /// Size in bytes.
    #[serde(default)]
    pub size: Option<u64>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
    /// Present for folders.
    #[serde(default)]
    pub folder: Option<Value>,
    /// Present for files.
    #[serde(default)]
    pub file: Option<Value>,
}

impl DriveItem {
    /// Whether the item is a folder.
    pub fn is_folder(&self) -> bool {
        self.folder.is_some()
    }
}
