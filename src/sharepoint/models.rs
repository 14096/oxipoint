use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

fn flex_opt<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(match Option::<Value>::deserialize(d)? {
        Some(Value::String(s)) => Some(s),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

fn flex_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(flex_opt(d)?.unwrap_or_default())
}

/// Accepts a plain string or the `{ "StringValue": ... }` wrapper used for content type ids.
fn content_type_id<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(match Option::<Value>::deserialize(d)? {
        Some(Value::String(s)) => s,
        Some(Value::Object(o)) => o
            .get("StringValue")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    })
}

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

/// A field (column).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Field {
    /// Field GUID.
    #[serde(default)]
    pub id: String,
    /// Display title.
    #[serde(default)]
    pub title: Option<String>,
    /// Internal name.
    #[serde(default)]
    pub internal_name: Option<String>,
    /// Field type such as `Text`.
    #[serde(default)]
    pub type_as_string: Option<String>,
    /// Group.
    #[serde(default)]
    pub group: Option<String>,
    /// Whether a value is required.
    #[serde(default)]
    pub required: Option<bool>,
    /// Whether the field is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
}

/// A content type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ContentType {
    /// Content type id such as `0x0101`.
    #[serde(default, deserialize_with = "content_type_id")]
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
}

/// A user or other principal.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SpUser {
    /// Id in the site's user information list.
    #[serde(default)]
    pub id: i64,
    /// Login name.
    #[serde(default)]
    pub login_name: Option<String>,
    /// Display name.
    #[serde(default)]
    pub title: Option<String>,
    /// Mail address.
    #[serde(default)]
    pub email: Option<String>,
    /// Principal type bit mask.
    #[serde(default)]
    pub principal_type: Option<i32>,
    /// Whether the user is a site collection administrator.
    #[serde(default)]
    pub is_site_admin: Option<bool>,
}

/// A site group.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SiteGroup {
    /// Group id.
    #[serde(default)]
    pub id: i64,
    /// Title.
    #[serde(default)]
    pub title: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Login name.
    #[serde(default)]
    pub login_name: Option<String>,
    /// Title of the group's owner.
    #[serde(default)]
    pub owner_title: Option<String>,
}

/// A named set of permissions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RoleDefinition {
    /// Role definition id.
    #[serde(default)]
    pub id: i64,
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Whether the role is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
    /// Permissions granted by the role (returned when expanded or selected).
    #[serde(default)]
    pub base_permissions: Option<BasePermissions>,
}

/// A principal with the roles assigned to it. Use `expand("Member,RoleDefinitionBindings")`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RoleAssignment {
    /// Principal id.
    #[serde(default)]
    pub principal_id: i64,
    /// The user or group (when expanded).
    #[serde(default)]
    pub member: Option<SpUser>,
    /// The assigned roles (when expanded).
    #[serde(default)]
    pub role_definition_bindings: Vec<RoleDefinition>,
}

/// Standard SharePoint permission kinds. The discriminant is the permission's bit number plus one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum PermissionKind {
    EmptyMask = 0,
    ViewListItems = 1,
    AddListItems = 2,
    EditListItems = 3,
    DeleteListItems = 4,
    ApproveItems = 5,
    OpenItems = 6,
    ViewVersions = 7,
    DeleteVersions = 8,
    CancelCheckout = 9,
    ManagePersonalViews = 10,
    ManageLists = 12,
    ViewFormPages = 13,
    AnonymousSearchAccessList = 14,
    Open = 17,
    ViewPages = 18,
    AddAndCustomizePages = 19,
    ApplyThemeAndBorder = 20,
    ApplyStyleSheets = 21,
    ViewUsageData = 22,
    CreateSSCSite = 23,
    ManageSubwebs = 24,
    CreateGroups = 25,
    ManagePermissions = 26,
    BrowseDirectories = 27,
    BrowseUserInfo = 28,
    AddDelPrivateWebParts = 29,
    UpdatePersonalWebParts = 30,
    ManageWeb = 31,
    AnonymousSearchAccessWebLists = 32,
    UseClientIntegration = 37,
    UseRemoteAPIs = 38,
    ManageAlerts = 39,
    CreateAlerts = 40,
    EditMyUserInfo = 41,
    EnumeratePermissions = 63,
    FullMask = 65,
}

impl PermissionKind {
    const ALL: &'static [PermissionKind] = {
        use PermissionKind::*;
        &[
            ViewListItems,
            AddListItems,
            EditListItems,
            DeleteListItems,
            ApproveItems,
            OpenItems,
            ViewVersions,
            DeleteVersions,
            CancelCheckout,
            ManagePersonalViews,
            ManageLists,
            ViewFormPages,
            AnonymousSearchAccessList,
            Open,
            ViewPages,
            AddAndCustomizePages,
            ApplyThemeAndBorder,
            ApplyStyleSheets,
            ViewUsageData,
            CreateSSCSite,
            ManageSubwebs,
            CreateGroups,
            ManagePermissions,
            BrowseDirectories,
            BrowseUserInfo,
            AddDelPrivateWebParts,
            UpdatePersonalWebParts,
            ManageWeb,
            AnonymousSearchAccessWebLists,
            UseClientIntegration,
            UseRemoteAPIs,
            ManageAlerts,
            CreateAlerts,
            EditMyUserInfo,
            EnumeratePermissions,
            FullMask,
        ]
    };
}

/// A permission mask as returned by SharePoint (two 32-bit halves, sent as strings).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct BasePermissions {
    /// High 32 bits.
    #[serde(default, deserialize_with = "flex_string")]
    pub high: String,
    /// Low 32 bits.
    #[serde(default, deserialize_with = "flex_string")]
    pub low: String,
}

impl BasePermissions {
    fn mask(&self) -> u64 {
        let high = self.high.trim().parse::<u64>().unwrap_or(0) & 0xFFFF_FFFF;
        let low = self.low.trim().parse::<u64>().unwrap_or(0) & 0xFFFF_FFFF;
        (high << 32) | low
    }

    /// Whether the mask grants `kind`.
    pub fn has(&self, kind: PermissionKind) -> bool {
        let mask = self.mask();
        match kind {
            PermissionKind::EmptyMask => true,
            PermissionKind::FullMask => {
                mask >> 32 == 0x7FFF_FFFF && mask & 0xFFFF_FFFF == 0xFFFF_FFFF
            }
            k => mask >> (k as u8 - 1) & 1 == 1,
        }
    }

    /// The named permissions granted by the mask.
    pub fn permissions(&self) -> Vec<PermissionKind> {
        PermissionKind::ALL
            .iter()
            .copied()
            .filter(|k| self.has(*k))
            .collect()
    }
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    /// Comment id.
    #[serde(default, alias = "Id", deserialize_with = "flex_string")]
    pub id: String,
    /// Text.
    #[serde(default, alias = "Text")]
    pub text: Option<String>,
    /// Creation time (ISO 8601).
    #[serde(default, alias = "CreatedDate")]
    pub created_date: Option<String>,
    /// Author.
    #[serde(default, alias = "Author")]
    pub author: Option<SpUser>,
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

/// Selects which changes `get_changes` returns.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChangeQuery {
    /// Include additions.
    pub add: bool,
    /// Include updates.
    pub update: bool,
    /// Include deletions.
    pub delete_object: bool,
    /// Include item changes.
    pub item: bool,
    /// Resume after this token (from a previous [`Change::change_token`]).
    pub change_token_start: Option<String>,
    /// Stop at this token.
    pub change_token_end: Option<String>,
}

impl ChangeQuery {
    /// Item additions, updates and deletions.
    pub fn items() -> Self {
        Self {
            add: true,
            update: true,
            delete_object: true,
            item: true,
            ..Self::default()
        }
    }

    /// Starts after the given change token.
    pub fn since(mut self, token: impl Into<String>) -> Self {
        self.change_token_start = Some(token.into());
        self
    }

    pub(crate) fn to_body(&self) -> Value {
        let mut q = Map::new();
        q.insert(
            "__metadata".into(),
            serde_json::json!({"type":"SP.ChangeQuery"}),
        );
        q.insert("Add".into(), self.add.into());
        q.insert("Update".into(), self.update.into());
        q.insert("DeleteObject".into(), self.delete_object.into());
        q.insert("Item".into(), self.item.into());
        for (key, token) in [
            ("ChangeTokenStart", &self.change_token_start),
            ("ChangeTokenEnd", &self.change_token_end),
        ] {
            if let Some(t) = token {
                q.insert(
                    key.into(),
                    serde_json::json!({"__metadata":{"type":"SP.ChangeToken"},"StringValue":t}),
                );
            }
        }
        serde_json::json!({ "query": q })
    }
}

/// A change reported by `get_changes`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Change {
    /// Change type (1 add, 2 update, 3 delete, ...).
    #[serde(default)]
    pub change_type: i32,
    /// Token of this change; pass it to [`ChangeQuery::since`] to resume.
    #[serde(default, deserialize_with = "content_type_id")]
    pub change_token: String,
    /// Changed item id, for item changes.
    #[serde(default)]
    pub item_id: Option<i64>,
    /// Id of the list the change belongs to.
    #[serde(default)]
    pub list_id: Option<String>,
    /// Time of the change (ISO 8601).
    #[serde(default)]
    pub time: Option<String>,
}

/// A list item; columns other than `Id` are available in `fields`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ListItem {
    /// Item id (0 when excluded by `$select`).
    #[serde(rename = "Id", alias = "ID", default)]
    pub id: i64,
    /// All other returned columns.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

crate::fields::item_readers!(ListItem, s => Some(&s.fields));

/// A folder.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FolderInfo {
    /// Name.
    pub name: String,
    /// Server-relative URL.
    pub server_relative_url: String,
    /// Number of children.
    #[serde(default)]
    pub item_count: Option<u64>,
}

/// A file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FileInfo {
    /// Name.
    pub name: String,
    /// Server-relative URL.
    pub server_relative_url: String,
    /// Size in bytes (SharePoint returns it as a string).
    #[serde(default)]
    pub length: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn list_models_tolerate_missing_and_extra_fields() {
        let l: ListInfo = serde_json::from_value(json!({
            "Id":"g","BaseTemplate":100,"BaseType":0,"Hidden":false,"Created":"2020-01-01T00:00:00Z",
            "EnableVersioning":true,"ContentTypesEnabled":false,
            "RootFolder":{"ServerRelativeUrl":"/sites/t/Lists/A"},"Other":1
        }))
        .unwrap();
        assert_eq!(
            (l.base_template, l.enable_versioning),
            (Some(100), Some(true))
        );
        assert_eq!(
            l.root_folder.unwrap().server_relative_url.as_deref(),
            Some("/sites/t/Lists/A")
        );
        let l: ListInfo = serde_json::from_value(json!({})).unwrap();
        assert!(l.hidden.is_none() && l.root_folder.is_none());
    }

    #[test]
    fn view_subscription_attachment_version_deserialize() {
        let v: View =
            serde_json::from_value(json!({"Id":"v","Title":"All Items","DefaultView":true,"x":1}))
                .unwrap();
        assert_eq!(
            (v.title.as_deref(), v.default_view),
            (Some("All Items"), Some(true))
        );
        let s: Subscription = serde_json::from_value(json!({"id":"s","notificationUrl":"https://n","expirationDateTime":"2030-01-01T00:00:00Z"})).unwrap();
        assert_eq!(s.notification_url.as_deref(), Some("https://n"));
        let a: Attachment = serde_json::from_value(json!({"FileName":"a.txt"})).unwrap();
        assert_eq!(a.file_name, "a.txt");
        let i: ItemVersion = serde_json::from_value(
            json!({"VersionId":512,"VersionLabel":"1.0","IsCurrentVersion":true}),
        )
        .unwrap();
        assert_eq!(i.version_label.as_deref(), Some("1.0"));
        let c: Change = serde_json::from_value(
            json!({"ChangeType":1,"ChangeToken":{"StringValue":"1;3;x;1;2"},"ItemId":7}),
        )
        .unwrap();
        assert_eq!((c.change_token.as_str(), c.item_id), ("1;3;x;1;2", Some(7)));
        let c: Change = serde_json::from_value(json!({})).unwrap();
        assert!(c.change_token.is_empty());
    }

    #[test]
    fn validated_write_and_comment_models() {
        let r: FieldResult = serde_json::from_value(
            json!({"FieldName":"Due","HasException":true,"ErrorMessage":"bad","ItemId":7,"x":1}),
        )
        .unwrap();
        assert!(!r.is_ok());
        assert_eq!(
            (r.error_message.as_deref(), r.item_id.as_deref()),
            (Some("bad"), Some("7"))
        );
        let r: FieldResult = serde_json::from_value(json!({"FieldName":"Title"})).unwrap();
        assert!(r.is_ok());
        let c: Comment =
            serde_json::from_value(json!({"id":"12","text":"Done","author":{"Id":5,"Title":"A"}}))
                .unwrap();
        assert_eq!(
            (c.id.as_str(), c.text.as_deref(), c.author.unwrap().id),
            ("12", Some("Done"), 5)
        );
        let c: Comment = serde_json::from_value(json!({"Id":3})).unwrap();
        assert_eq!(c.id, "3");
    }

    #[test]
    fn new_list_and_change_query_bodies() {
        let l = NewList::new("Tasks", ListTemplate::GenericList);
        assert_eq!(
            serde_json::to_value(&l).unwrap(),
            json!({"Title":"Tasks","BaseTemplate":100})
        );
        let l = NewList::new("X", ListTemplate::Custom(851)).description("d");
        assert_eq!(
            serde_json::to_value(&l).unwrap(),
            json!({"Title":"X","BaseTemplate":851,"Description":"d"})
        );
        let b = ChangeQuery::items().since("tok").to_body();
        assert_eq!(b["query"]["ChangeTokenStart"]["StringValue"], "tok");
        assert_eq!(b["query"]["Item"], true);
        assert!(b["query"].get("ChangeTokenEnd").is_none());
    }

    #[test]
    fn web_with_extra_and_missing_fields() {
        let w: Web = serde_json::from_value(json!({
            "Id":"g","Language":1033,"Created":"2020-01-01T00:00:00Z","WebTemplate":"STS",
            "MasterUrl":"/_catalogs/masterpage/seattle.master","Unknown":1
        }))
        .unwrap();
        assert_eq!(
            (w.language, w.web_template.as_deref()),
            (Some(1033), Some("STS"))
        );
        let w: Web = serde_json::from_value(json!({})).unwrap();
        assert!(w.id.is_empty() && w.language.is_none());
    }

    #[test]
    fn content_type_id_accepts_wrapper_and_string() {
        let a: ContentType =
            serde_json::from_value(json!({"Id":{"StringValue":"0x0101"},"Name":"Doc"})).unwrap();
        let b: ContentType = serde_json::from_value(json!({"Id":"0x01"})).unwrap();
        assert_eq!((a.id.as_str(), b.id.as_str()), ("0x0101", "0x01"));
    }

    #[test]
    fn collection_models_deserialize() {
        let n: NavigationNode =
            serde_json::from_value(json!({"Id":2003,"Title":"Home","IsExternal":false})).unwrap();
        assert_eq!((n.id, n.is_external), (2003, Some(false)));
        let f: Field =
            serde_json::from_value(json!({"Id":"g","InternalName":"Title","TypeAsString":"Text"}))
                .unwrap();
        assert_eq!(f.type_as_string.as_deref(), Some("Text"));
        let u: SpUser =
            serde_json::from_value(json!({"Id":5,"LoginName":"i:0#.f|m|a@b.com"})).unwrap();
        assert_eq!(u.id, 5);
        let g: SiteGroup = serde_json::from_value(json!({"Id":3,"Title":"Owners"})).unwrap();
        assert_eq!(g.title.as_deref(), Some("Owners"));
        let r: RoleAssignment = serde_json::from_value(json!({
            "PrincipalId":5,"Member":{"Id":5,"Title":"A"},
            "RoleDefinitionBindings":[{"Id":1073741829,"Name":"Full Control","BasePermissions":{"High":"2147483647","Low":"4294967295"}}]
        })).unwrap();
        assert!(
            r.role_definition_bindings[0]
                .base_permissions
                .as_ref()
                .unwrap()
                .has(PermissionKind::FullMask)
        );
        let r: RoleAssignment = serde_json::from_value(json!({"PrincipalId":1})).unwrap();
        assert!(r.role_definition_bindings.is_empty());
        let x: Feature = serde_json::from_value(json!({"DefinitionId":"abc"})).unwrap();
        assert_eq!(x.definition_id, "abc");
        let s: RegionalSettings =
            serde_json::from_value(json!({"LocaleId":1033,"TimeZone":{"Id":4,"Description":"d"}}))
                .unwrap();
        assert_eq!(s.time_zone.unwrap().id, Some(4));
        let b: RecycleBinItem = serde_json::from_value(json!({"Id":"g","Size":"12"})).unwrap();
        let c: RecycleBinItem = serde_json::from_value(json!({"Id":"g","Size":12})).unwrap();
        assert_eq!((b.size, c.size), (Some("12".into()), Some("12".into())));
        let i: SiteInfo = serde_json::from_value(json!({"Id":"g","Url":"u"})).unwrap();
        assert_eq!(i.url.as_deref(), Some("u"));
    }

    #[test]
    fn base_permissions_decode_table() {
        let p = |high: &str, low: &str| BasePermissions {
            high: high.into(),
            low: low.into(),
        };
        let view = p("0", "1");
        assert!(view.has(PermissionKind::ViewListItems) && !view.has(PermissionKind::AddListItems));
        assert_eq!(view.permissions(), vec![PermissionKind::ViewListItems]);
        assert!(p("0", "2").has(PermissionKind::AddListItems));
        assert!(p("0", "65536").has(PermissionKind::Open));
        assert!(p("0", "1073741824").has(PermissionKind::ManageWeb));
        assert!(p("1073741824", "0").has(PermissionKind::EnumeratePermissions));
        assert!(!p("32", "0").has(PermissionKind::UseClientIntegration));
        assert!(p("16", "0").has(PermissionKind::UseClientIntegration));
        let full = p("2147483647", "4294967295");
        assert!(full.has(PermissionKind::FullMask) && full.has(PermissionKind::ManageWeb));
        assert!(!p("0", "0").has(PermissionKind::ViewListItems));
        assert!(p("0", "0").has(PermissionKind::EmptyMask));
        let n: BasePermissions = serde_json::from_value(json!({"High":0,"Low":1})).unwrap();
        assert_eq!(n.low, "1");
    }

    #[test]
    fn new_web_serializes_pascal_case() {
        let v = serde_json::to_value(NewWeb::new("T", "t", "STS#3")).unwrap();
        assert_eq!(
            v,
            json!({"Title":"T","Url":"t","WebTemplate":"STS#3","UseSamePermissionsAsParentSite":true})
        );
    }
}
