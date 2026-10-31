use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A directory user.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Object id.
    pub id: String,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Primary mail address.
    #[serde(default)]
    pub mail: Option<String>,
    /// Sign-in name.
    #[serde(default)]
    pub user_principal_name: Option<String>,
    /// Given name.
    #[serde(default)]
    pub given_name: Option<String>,
    /// Surname.
    #[serde(default)]
    pub surname: Option<String>,
    /// Job title.
    #[serde(default)]
    pub job_title: Option<String>,
}

/// A SharePoint site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    /// Composite id (`host,siteId,webId`).
    pub id: String,
    /// URL segment name.
    #[serde(default)]
    pub name: Option<String>,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
}

/// A SharePoint list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct List {
    /// List id.
    pub id: String,
    /// Internal name.
    #[serde(default)]
    pub name: Option<String>,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
}

/// An item of a list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    /// Item id.
    pub id: String,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
    /// Column values (present when `$expand=fields` is used or after create).
    #[serde(default)]
    pub fields: Option<Map<String, Value>>,
}

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
