use serde::{Deserialize, Serialize};

/// A modern SharePoint page.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SitePage {
    /// Item id of the page in the Site Pages library.
    #[serde(default)]
    pub id: u32,
    /// File name, for example `Home.aspx`.
    #[serde(default)]
    pub file_name: Option<String>,
    /// Title.
    #[serde(default)]
    pub title: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Server-relative URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Layout (`Article`, `Home`, ...).
    #[serde(default)]
    pub page_layout_type: Option<String>,
    /// Canvas content as JSON text; present when selected (`CanvasContent1`).
    #[serde(default, rename = "CanvasContent1")]
    pub canvas_content: Option<String>,
    /// Whether the page is a promoted news post (`Article` pages only).
    #[serde(default)]
    pub promotion_kind: Option<String>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub first_published: Option<String>,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified: Option<String>,
}

/// Definition of a page for [`PagesRequest::create`](super::PagesRequest::create).
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct NewSitePage {
    #[serde(rename = "__metadata")]
    metadata: Metadata,
    /// File name, for example `News.aspx`.
    pub name: String,
    /// Title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Layout (`Article`, `Home`, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_layout_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Metadata {
    #[serde(rename = "type")]
    kind: &'static str,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            kind: "SP.Publishing.SitePage",
        }
    }
}

impl NewSitePage {
    /// A definition with the given file name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Sets the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the layout.
    pub fn page_layout_type(mut self, layout: impl Into<String>) -> Self {
        self.page_layout_type = Some(layout.into());
        self
    }
}
