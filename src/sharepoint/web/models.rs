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
    /// Language (LCID).
    #[serde(default)]
    pub language: Option<u32>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created: Option<String>,
    /// Last time an item changed (ISO 8601).
    #[serde(default)]
    pub last_item_modified_date: Option<String>,
    /// Web template name.
    #[serde(default)]
    pub web_template: Option<String>,
    /// Master page URL.
    #[serde(default)]
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
    #[serde(default)]
    pub title: Option<String>,
    /// URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Whether the URL points outside the site.
    #[serde(default)]
    pub is_external: Option<bool>,
    /// Whether the node is shown.
    #[serde(default)]
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
    #[serde(default)]
    pub display_name: Option<String>,
}

/// A time zone.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TimeZone {
    /// Time zone id.
    #[serde(default)]
    pub id: Option<i32>,
    /// Description such as `(UTC+01:00) Amsterdam, Berlin`.
    #[serde(default)]
    pub description: Option<String>,
}

/// Regional settings of a web. Use `expand("TimeZone")` to include the time zone.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RegionalSettings {
    /// Locale id (LCID).
    #[serde(default, rename = "LocaleId")]
    pub locale_id: Option<u32>,
    /// Time zone (when expanded).
    #[serde(default)]
    pub time_zone: Option<TimeZone>,
    /// First day of the week (0 is Sunday).
    #[serde(default)]
    pub first_day_of_week: Option<u8>,
    /// Time format (0 is 12 hour, 1 is 24 hour).
    #[serde(default)]
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
    #[serde(default)]
    pub title: Option<String>,
    /// File or folder name.
    #[serde(default)]
    pub leaf_name: Option<String>,
    /// Folder the item was deleted from.
    #[serde(default)]
    pub dir_name: Option<String>,
    /// Size in bytes.
    #[serde(default, deserialize_with = "flex_opt")]
    pub size: Option<String>,
    /// Deletion time (ISO 8601).
    #[serde(default)]
    pub deleted_date: Option<String>,
}

/// A site collection. Use `expand("Usage")` to include usage numbers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SiteInfo {
    /// Site collection GUID.
    #[serde(default)]
    pub id: String,
    /// Absolute URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Server-relative URL.
    #[serde(default)]
    pub server_relative_url: Option<String>,
    /// Usage information (when expanded).
    #[serde(default)]
    pub usage: Option<Value>,
}
