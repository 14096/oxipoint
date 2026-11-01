use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::sharepoint::{SpUser, inner::flex_opt};

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
    /// Template number (see [`ListTemplate`]).
    #[serde(default)]
    pub base_template: Option<i32>,
    /// Base type (0 list, 1 library, ...).
    #[serde(default)]
    pub base_type: Option<i32>,
    /// Whether the list is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created: Option<String>,
    /// Last time an item changed (ISO 8601).
    #[serde(default)]
    pub last_item_modified_date: Option<String>,
    /// Root folder (returned with `expand("RootFolder")`).
    #[serde(default)]
    pub root_folder: Option<RootFolder>,
    /// Whether versioning is enabled.
    #[serde(default)]
    pub enable_versioning: Option<bool>,
    /// Whether content types are enabled.
    #[serde(default)]
    pub content_types_enabled: Option<bool>,
}

/// The root folder of a list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RootFolder {
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Server-relative URL.
    #[serde(default)]
    pub server_relative_url: Option<String>,
}

/// Well-known list templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListTemplate {
    /// Custom list (100).
    GenericList,
    /// Document library (101).
    DocumentLibrary,
    /// Survey (102).
    Survey,
    /// Links (103).
    Links,
    /// Announcements (104).
    Announcements,
    /// Contacts (105).
    Contacts,
    /// Calendar (106).
    Events,
    /// Tasks (107).
    Tasks,
    /// Discussion board (108).
    DiscussionBoard,
    /// Picture library (109).
    PictureLibrary,
    /// Any other template number.
    Custom(i32),
}

impl ListTemplate {
    /// The template number.
    pub fn value(self) -> i32 {
        match self {
            Self::GenericList => 100,
            Self::DocumentLibrary => 101,
            Self::Survey => 102,
            Self::Links => 103,
            Self::Announcements => 104,
            Self::Contacts => 105,
            Self::Events => 106,
            Self::Tasks => 107,
            Self::DiscussionBoard => 108,
            Self::PictureLibrary => 109,
            Self::Custom(n) => n,
        }
    }
}

/// Input for creating a list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct NewList {
    /// Title.
    pub title: String,
    /// Template number.
    pub base_template: i32,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl NewList {
    /// A list with a title and template.
    pub fn new(title: impl Into<String>, template: ListTemplate) -> Self {
        Self {
            title: title.into(),
            base_template: template.value(),
            description: None,
        }
    }

    /// Sets the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// A list view.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct View {
    /// View GUID.
    #[serde(default)]
    pub id: String,
    /// Title.
    #[serde(default)]
    pub title: Option<String>,
    /// Server-relative URL.
    #[serde(default)]
    pub server_relative_url: Option<String>,
    /// Whether this is the default view.
    #[serde(default)]
    pub default_view: Option<bool>,
    /// Whether the view is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
    /// Rows per page.
    #[serde(default)]
    pub row_limit: Option<u32>,
}

/// A webhook subscription on a list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    /// Subscription id.
    #[serde(default)]
    pub id: String,
    /// Resource being watched.
    #[serde(default)]
    pub resource: Option<String>,
    /// Endpoint that receives notifications.
    #[serde(default)]
    pub notification_url: Option<String>,
    /// Expiry (ISO 8601).
    #[serde(default)]
    pub expiration_date_time: Option<String>,
    /// Opaque value echoed in notifications.
    #[serde(default)]
    pub client_state: Option<String>,
}

/// Input for adding a webhook subscription.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSubscription {
    /// Endpoint that receives notifications.
    pub notification_url: String,
    /// Expiry (ISO 8601).
    pub expiration_date_time: String,
    /// Opaque value echoed in notifications.
    pub client_state: Option<String>,
}

impl NewSubscription {
    /// A subscription with a notification URL and expiry.
    pub fn new(notification_url: impl Into<String>, expiration: impl Into<String>) -> Self {
        Self {
            notification_url: notification_url.into(),
            expiration_date_time: expiration.into(),
            client_state: None,
        }
    }
}

/// The result of one column of a validated write.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FieldResult {
    /// Internal name of the column.
    #[serde(default)]
    pub field_name: String,
    /// Value after the write, as formatted text.
    #[serde(default)]
    pub field_value: Option<String>,
    /// Whether the column was rejected.
    #[serde(default)]
    pub has_exception: bool,
    /// Why the column was rejected.
    #[serde(default)]
    pub error_message: Option<String>,
    /// Id of the created or updated item, when returned.
    #[serde(default, deserialize_with = "flex_opt")]
    pub item_id: Option<String>,
}

impl FieldResult {
    /// Whether the column was accepted.
    pub fn is_ok(&self) -> bool {
        !self.has_exception
    }
}

/// Options for validated item writes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValidateOptions {
    /// Treat the write as a new document update (`bNewDocumentUpdate`).
    pub new_document_update: bool,
    /// Check-in comment.
    pub check_in_comment: Option<String>,
}

/// A comment on a list item.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    /// Comment id.
    pub id: String,
    /// Text.
    pub text: Option<String>,
    /// Creation time (ISO 8601).
    pub created_date: Option<String>,
    /// Author.
    pub author: Option<SpUser>,
}

impl<'de> Deserialize<'de> for Comment {
    /// Accepts camelCase and PascalCase names, even when a response has both.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let mut m = Map::deserialize(d)?;
        let mut take = |lower: &str, upper: &str| {
            let a = m.remove(lower);
            let b = m.remove(upper);
            a.filter(|v| !v.is_null()).or(b).filter(|v| !v.is_null())
        };
        let id = match take("id", "Id") {
            Some(Value::String(s)) => s,
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        let text = take("text", "Text").and_then(|v| v.as_str().map(str::to_owned));
        let created_date =
            take("createdDate", "CreatedDate").and_then(|v| v.as_str().map(str::to_owned));
        let author = match take("author", "Author") {
            Some(v) => Some(serde_json::from_value(v).map_err(serde::de::Error::custom)?),
            None => None,
        };
        Ok(Self {
            id,
            text,
            created_date,
            author,
        })
    }
}

/// A file attached to a list item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Attachment {
    /// File name.
    #[serde(default)]
    pub file_name: String,
    /// Server-relative URL.
    #[serde(default)]
    pub server_relative_url: Option<String>,
}

/// One version of a list item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ItemVersion {
    /// Version id.
    #[serde(default)]
    pub version_id: Option<i64>,
    /// Version label such as `2.0`.
    #[serde(default)]
    pub version_label: Option<String>,
    /// Whether this is the current version.
    #[serde(default)]
    pub is_current_version: Option<bool>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created: Option<String>,
}

/// A list item; columns other than `Id` are available in `fields`.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct ListItem {
    /// Item id (0 when excluded by `$select`).
    #[serde(rename = "Id")]
    pub id: i64,
    /// All other returned columns.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

impl<'de> Deserialize<'de> for ListItem {
    /// Accepts `Id` and `ID`, which SharePoint may return together.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let mut fields = Map::deserialize(d)?;
        let upper = fields.remove("ID");
        let id = fields.remove("Id").or(upper);
        let id = match id {
            None | Some(Value::Null) => 0,
            Some(v) => v
                .as_i64()
                .ok_or_else(|| serde::de::Error::custom("item id is not an integer"))?,
        };
        Ok(Self { id, fields })
    }
}

crate::fields::item_readers!(ListItem, s => Some(&s.fields));
