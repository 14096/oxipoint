use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::graph::{IdentitySet, ParentReference};

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
    /// Where the item lives.
    #[serde(default)]
    pub parent_reference: Option<ParentReference>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified_date_time: Option<String>,
    /// Who created the item.
    #[serde(default)]
    pub created_by: Option<IdentitySet>,
    /// Who modified the item last.
    #[serde(default)]
    pub last_modified_by: Option<IdentitySet>,
    /// Entity tag of the item.
    #[serde(default)]
    pub e_tag: Option<String>,
    /// Entity tag of the item's content.
    #[serde(default)]
    pub c_tag: Option<String>,
    /// Present in delta results for deleted items.
    #[serde(default)]
    pub deleted: Option<Value>,
}

impl DriveItem {
    /// MIME type of a file, when reported.
    pub fn mime_type(&self) -> Option<&str> {
        self.file.as_ref()?.get("mimeType")?.as_str()
    }

    /// Whether a delta result marks the item as deleted.
    pub fn is_deleted(&self) -> bool {
        self.deleted.is_some()
    }

    /// Whether the item is a folder.
    pub fn is_folder(&self) -> bool {
        self.folder.is_some()
    }
}

/// What to do when a folder or file with the same name exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConflictBehavior {
    /// Fail with an HTTP error (the service default).
    Fail,
    /// Keep both by renaming the new item.
    Rename,
    /// Replace the existing item.
    Replace,
}

impl ConflictBehavior {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ConflictBehavior::Fail => "fail",
            ConflictBehavior::Rename => "rename",
            ConflictBehavior::Replace => "replace",
        }
    }
}

/// Changes to apply to a drive item with one `PATCH`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DriveItemUpdate {
    name: Option<String>,
    description: Option<String>,
    parent_id: Option<String>,
}

impl DriveItemUpdate {
    /// An update that changes nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Renames the item.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Moves the item under the folder with this id (in the same drive).
    pub fn move_to(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub(crate) fn to_json(&self) -> Value {
        let mut m = serde_json::Map::new();
        if let Some(n) = &self.name {
            m.insert("name".into(), n.clone().into());
        }
        if let Some(d) = &self.description {
            m.insert("description".into(), d.clone().into());
        }
        if let Some(p) = &self.parent_id {
            m.insert("parentReference".into(), serde_json::json!({ "id": p }));
        }
        Value::Object(m)
    }
}

/// A version of a drive item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriveItemVersion {
    /// Version id such as `2.0`.
    pub id: String,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified_date_time: Option<String>,
    /// Size in bytes.
    #[serde(default)]
    pub size: Option<u64>,
    /// Who made the version.
    #[serde(default)]
    pub last_modified_by: Option<IdentitySet>,
}

/// The result of a drive delta query.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DriveDeltaPage {
    /// Changed items across all pages. Deleted items have [`DriveItem::is_deleted`] set.
    pub items: Vec<DriveItem>,
    /// Token to pass to the next `delta` call to receive only newer changes.
    pub next_token: Option<String>,
}
