use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A SharePoint site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    /// Composite id (`host,siteId,webId`).
    pub id: String,
    /// URL segment name.
    pub name: Option<String>,
    /// Display name.
    pub display_name: Option<String>,
    /// Browser URL.
    pub web_url: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Creation time (ISO 8601).
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    pub last_modified_date_time: Option<String>,
    /// Present (as an empty object) on the root site of a site collection.
    pub root: Option<Value>,
    /// Site collection information.
    pub site_collection: Option<SiteCollection>,
}

/// Site collection information of a [`Site`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SiteCollection {
    /// Host name.
    pub hostname: Option<String>,
    /// Data location code.
    pub data_location_code: Option<String>,
    /// Present (as an empty object) when the site is a collection root.
    pub root: Option<Value>,
}

/// A site column.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// Column id.
    pub id: String,
    /// Internal name.
    pub name: Option<String>,
    /// Display name.
    pub display_name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Column group.
    pub column_group: Option<String>,
    /// Whether a value is required.
    pub required: Option<bool>,
    /// Whether the column is hidden.
    pub hidden: Option<bool>,
}

/// A content type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContentType {
    /// Content type id such as `0x0101...`.
    pub id: String,
    /// Name.
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Group.
    pub group: Option<String>,
    /// Whether the content type is hidden.
    pub hidden: Option<bool>,
    /// Whether the content type ships with SharePoint.
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
    #[serde(rename = "grantedToV2")]
    pub granted_to: Option<Value>,
    /// Sharing link, when the permission comes from one.
    pub link: Option<Value>,
    /// Expiration time (ISO 8601).
    pub expiration_date_time: Option<String>,
    /// Whether a sharing link is protected by a password.
    pub has_password: Option<bool>,
}

impl Permission {
    /// The URL of a sharing link, when this permission is one.
    pub fn link_url(&self) -> Option<&str> {
        self.link.as_ref()?.get("webUrl")?.as_str()
    }

    /// The type of a sharing link (`view`, `edit`, ...).
    pub fn link_type(&self) -> Option<&str> {
        self.link.as_ref()?.get("type")?.as_str()
    }

    /// The scope of a sharing link (`anonymous`, `organization`, `users`).
    pub fn link_scope(&self) -> Option<&str> {
        self.link.as_ref()?.get("scope")?.as_str()
    }
}

/// A modern SharePoint page of a site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SitePage {
    /// Page id.
    pub id: String,
    /// OData type (`#microsoft.graph.sitePage` for site pages).
    #[serde(rename = "@odata.type")]
    pub odata_type: Option<String>,
    /// File name, for example `Home.aspx`.
    pub name: Option<String>,
    /// Title.
    pub title: Option<String>,
    /// Browser URL.
    pub web_url: Option<String>,
    /// Page layout (`article`, `home`, ...).
    pub page_layout: Option<String>,
    /// Publishing state.
    pub publishing_state: Option<PublishingState>,
    /// Creation time (ISO 8601).
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    pub last_modified_date_time: Option<String>,
    /// Canvas content; present with `$expand=canvasLayout`.
    pub canvas_layout: Option<CanvasLayout>,
}

/// Publishing state of a [`SitePage`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishingState {
    /// `draft` or `published`.
    pub level: Option<String>,
    /// Version label.
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
    pub vertical_section: Option<Value>,
}

/// A horizontal section of a canvas.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HorizontalSection {
    /// Section id.
    pub id: Option<String>,
    /// Layout (`oneColumn`, `twoColumns`, ...).
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
    pub id: Option<String>,
    /// Width in grid units.
    pub width: Option<u32>,
    /// Web parts in the column.
    #[serde(default)]
    pub webparts: Vec<WebPart>,
}

/// A web part on a page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WebPart {
    /// Web part id.
    pub id: Option<String>,
    /// OData type, for example `#microsoft.graph.textWebPart`.
    #[serde(rename = "@odata.type")]
    pub odata_type: Option<String>,
    /// Inner HTML of a text web part.
    #[serde(rename = "innerHtml")]
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
