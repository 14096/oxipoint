use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::sharepoint::{SpUser, inner::flex_opt};

/// A folder.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FolderInfo {
    /// Name (empty when excluded by `$select`).
    #[serde(default)]
    pub name: String,
    /// Server-relative URL (empty when excluded by `$select`).
    #[serde(default)]
    pub server_relative_url: String,
    /// Number of children.
    #[serde(default)]
    pub item_count: Option<u64>,
}

/// A file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FileInfo {
    /// Name (empty when excluded by `$select`).
    #[serde(default)]
    pub name: String,
    /// Server-relative URL (empty when excluded by `$select`).
    #[serde(default)]
    pub server_relative_url: String,
    /// Size in bytes (SharePoint returns it as a string).
    #[serde(default, deserialize_with = "flex_opt")]
    pub length: Option<String>,
    /// Column values of the file's list item (with `$expand=ListItemAllFields`).
    #[serde(default, rename = "ListItemAllFields")]
    pub list_item: Option<Map<String, Value>>,
    /// Who created the file (with `$expand=Author`).
    #[serde(default)]
    pub author: Option<SpUser>,
    /// Who modified the file last (with `$expand=ModifiedBy`).
    #[serde(default)]
    pub modified_by: Option<SpUser>,
    /// Who has the file checked out (with `$expand=CheckedOutByUser`).
    #[serde(default)]
    pub checked_out_by_user: Option<SpUser>,
    /// Comment of the last check-in.
    #[serde(default)]
    pub check_in_comment: Option<String>,
}

/// How a file is checked in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CheckInKind {
    /// A minor version.
    Minor,
    /// A major version.
    Major,
    /// Overwrite the current version.
    Overwrite,
}

impl CheckInKind {
    pub(crate) fn code(self) -> u8 {
        match self {
            CheckInKind::Minor => 0,
            CheckInKind::Major => 1,
            CheckInKind::Overwrite => 2,
        }
    }
}

/// A version of a file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FileVersion {
    /// Version id, used to read or download the version.
    #[serde(default, rename = "ID")]
    pub id: i64,
    /// Label such as `2.0`.
    #[serde(default)]
    pub version_label: Option<String>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created: Option<String>,
    /// Whether this is the current version.
    #[serde(default)]
    pub is_current_version: Option<bool>,
    /// Check-in comment.
    #[serde(default)]
    pub check_in_comment: Option<String>,
    /// Size in bytes.
    #[serde(default)]
    pub size: Option<i64>,
    /// URL of the version relative to the site.
    #[serde(default)]
    pub url: Option<String>,
}

/// A file or folder found by a recursive listing.
#[derive(Debug, Clone, PartialEq)]
pub enum FolderEntry {
    /// A folder.
    Folder(FolderInfo),
    /// A file.
    File(Box<FileInfo>),
}

impl FolderEntry {
    /// Server-relative URL of the entry.
    pub fn server_relative_url(&self) -> &str {
        match self {
            FolderEntry::Folder(f) => &f.server_relative_url,
            FolderEntry::File(f) => &f.server_relative_url,
        }
    }
}
