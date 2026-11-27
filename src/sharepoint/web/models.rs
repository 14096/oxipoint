use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::sharepoint::inner::flex_opt;

/// A SharePoint web (site).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Web {
    /// Web GUID (empty when excluded by `$select`).
    #[serde(default)]
    pub id: String,
    /// Title.
    pub title: Option<String>,
    /// Absolute URL.
    pub url: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Server-relative URL.
    pub server_relative_url: Option<String>,
    /// Language (LCID).
    pub language: Option<u32>,
    /// Creation time (ISO 8601).
    pub created: Option<String>,
    /// Last time an item changed (ISO 8601).
    pub last_item_modified_date: Option<String>,
    /// Web template name.
    pub web_template: Option<String>,
    /// Master page URL.
    pub master_url: Option<String>,
}

/// Input for creating a subweb.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct NewWeb {
    /// Title.
    pub title: String,
    /// URL segment of the new web.
    pub url: String,
    /// Web template such as `STS#3`.
    pub web_template: String,
    /// Language (LCID); the parent's language when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<u32>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Inherit permissions from the parent web.
    pub use_same_permissions_as_parent_site: bool,
}

impl NewWeb {
    /// Creates a definition that inherits permissions from the parent.
    pub fn new(
        title: impl Into<String>,
        url: impl Into<String>,
        web_template: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
            web_template: web_template.into(),
            language: None,
            description: None,
            use_same_permissions_as_parent_site: true,
        }
    }
}

/// A navigation node.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct NavigationNode {
    /// Node id.
    #[serde(default)]
    pub id: i64,
    /// Title.
    pub title: Option<String>,
    /// URL.
    pub url: Option<String>,
    /// Whether the URL points outside the site.
    pub is_external: Option<bool>,
    /// Whether the node is shown.
    pub is_visible: Option<bool>,
}

/// A web or site collection feature.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Feature {
    /// Feature definition GUID.
    #[serde(default)]
    pub definition_id: String,
    /// Display name (when selected).
    pub display_name: Option<String>,
}

/// A time zone.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TimeZone {
    /// Time zone id.
    pub id: Option<i32>,
    /// Description such as `(UTC+01:00) Amsterdam, Berlin`.
    pub description: Option<String>,
}

/// Regional settings of a web. Use `expand("TimeZone")` to include the time zone.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RegionalSettings {
    /// Locale id (LCID).
    #[serde(rename = "LocaleId")]
    pub locale_id: Option<u32>,
    /// Time zone (when expanded).
    pub time_zone: Option<TimeZone>,
    /// First day of the week (0 is Sunday).
    pub first_day_of_week: Option<u8>,
    /// Time format (0 is 12 hour, 1 is 24 hour).
    pub time_format: Option<u8>,
}

/// An item in the recycle bin.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RecycleBinItem {
    /// Item GUID.
    #[serde(default)]
    pub id: String,
    /// Title.
    pub title: Option<String>,
    /// File or folder name.
    pub leaf_name: Option<String>,
    /// Folder the item was deleted from.
    pub dir_name: Option<String>,
    /// Size in bytes.
    #[serde(default, deserialize_with = "flex_opt")]
    pub size: Option<String>,
    /// Deletion time (ISO 8601).
    pub deleted_date: Option<String>,
    /// Kind of item (1 file, 2 file version, 3 list item, 5 list, 6 folder, 9 attachment).
    pub item_type: Option<i32>,
    /// Stage the item is in (1 first stage, 2 second stage).
    pub item_state: Option<i32>,
    /// Name of the author.
    pub author_name: Option<String>,
    /// Name of the user who deleted the item.
    pub deleted_by_name: Option<String>,
}

/// A site collection. Use `expand("Usage")` to include usage numbers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SiteInfo {
    /// Site collection GUID.
    #[serde(default)]
    pub id: String,
    /// Absolute URL.
    pub url: Option<String>,
    /// Server-relative URL.
    pub server_relative_url: Option<String>,
    /// Usage information (when expanded).
    pub usage: Option<Value>,
}
