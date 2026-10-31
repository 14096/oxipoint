use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified_date_time: Option<String>,
    /// Present (as an empty object) on the root site of a site collection.
    #[serde(default)]
    pub root: Option<Value>,
    /// Site collection information.
    #[serde(default)]
    pub site_collection: Option<SiteCollection>,
}

/// Site collection information of a [`Site`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SiteCollection {
    /// Host name.
    #[serde(default)]
    pub hostname: Option<String>,
    /// Data location code.
    #[serde(default)]
    pub data_location_code: Option<String>,
    /// Present (as an empty object) when the site is a collection root.
    #[serde(default)]
    pub root: Option<Value>,
}

/// A site column.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// Column id.
    pub id: String,
    /// Internal name.
    #[serde(default)]
    pub name: Option<String>,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Column group.
    #[serde(default)]
    pub column_group: Option<String>,
    /// Whether a value is required.
    #[serde(default)]
    pub required: Option<bool>,
    /// Whether the column is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
}

/// A content type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContentType {
    /// Content type id such as `0x0101...`.
    pub id: String,
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Group.
    #[serde(default)]
    pub group: Option<String>,
    /// Whether the content type is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
    /// Whether the content type ships with SharePoint.
    #[serde(default)]
    pub is_built_in: Option<bool>,
}

/// A permission granted on a site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    /// Permission id.
    pub id: String,
    /// Granted roles such as `read` or `write`.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Identities the permission is granted to (application, user, group ...).
    #[serde(default, rename = "grantedToIdentitiesV2")]
    pub granted_to_identities: Vec<Value>,
    /// Single identity the permission is granted to.
    #[serde(default, rename = "grantedToV2")]
    pub granted_to: Option<Value>,
    /// Sharing link, when the permission comes from one.
    #[serde(default)]
    pub link: Option<Value>,
    /// Expiration time (ISO 8601).
    #[serde(default)]
    pub expiration_date_time: Option<String>,
}

/// A modern SharePoint page of a site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SitePage {
    /// Page id.
    pub id: String,
    /// OData type (`#microsoft.graph.sitePage` for site pages).
    #[serde(default, rename = "@odata.type")]
    pub odata_type: Option<String>,
    /// File name, for example `Home.aspx`.
    #[serde(default)]
    pub name: Option<String>,
    /// Title.
    #[serde(default)]
    pub title: Option<String>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
    /// Page layout (`article`, `home`, ...).
    #[serde(default)]
    pub page_layout: Option<String>,
    /// Publishing state.
    #[serde(default)]
    pub publishing_state: Option<PublishingState>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified_date_time: Option<String>,
    /// Canvas content; present with `$expand=canvasLayout`.
    #[serde(default)]
    pub canvas_layout: Option<CanvasLayout>,
}

/// Publishing state of a [`SitePage`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishingState {
    /// `draft` or `published`.
    #[serde(default)]
    pub level: Option<String>,
    /// Version label.
    #[serde(default)]
    pub version_id: Option<String>,
}

/// Canvas content of a page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasLayout {
    /// Horizontal sections.
    #[serde(default)]
    pub horizontal_sections: Vec<HorizontalSection>,
    /// The vertical section, if any.
    #[serde(default)]
    pub vertical_section: Option<Value>,
}

/// A horizontal section of a canvas.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HorizontalSection {
    /// Section id.
    #[serde(default)]
    pub id: Option<String>,
    /// Layout (`oneColumn`, `twoColumns`, ...).
    #[serde(default)]
    pub layout: Option<String>,
    /// Columns of the section.
    #[serde(default)]
    pub columns: Vec<HorizontalSectionColumn>,
}

/// A column of a [`HorizontalSection`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HorizontalSectionColumn {
    /// Column id.
    #[serde(default)]
    pub id: Option<String>,
    /// Width in grid units.
    #[serde(default)]
    pub width: Option<u32>,
    /// Web parts in the column.
    #[serde(default)]
    pub webparts: Vec<WebPart>,
}

/// A web part on a page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WebPart {
    /// Web part id.
    #[serde(default)]
    pub id: Option<String>,
    /// OData type, for example `#microsoft.graph.textWebPart`.
    #[serde(default, rename = "@odata.type")]
    pub odata_type: Option<String>,
    /// Inner HTML of a text web part.
    #[serde(default, rename = "innerHtml")]
    pub inner_html: Option<String>,
    /// Remaining type-specific properties.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Definition of a page for [`PagesRequest::create`](super::PagesRequest::create).
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewSitePage {
    /// OData type, always `#microsoft.graph.sitePage`.
    #[serde(rename = "@odata.type")]
    odata_type: &'static str,
    /// File name, for example `Home.aspx`.
    pub name: String,
    /// Title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Page layout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_layout: Option<String>,
    /// Canvas content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canvas_layout: Option<CanvasLayout>,
}

impl NewSitePage {
    /// A definition with the given file name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            odata_type: "#microsoft.graph.sitePage",
            name: name.into(),
            ..Self::default()
        }
    }

    /// Sets the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the page layout.
    pub fn page_layout(mut self, layout: impl Into<String>) -> Self {
        self.page_layout = Some(layout.into());
        self
    }
}
