use serde::{Deserialize, Serialize};

use crate::sharepoint::inner::flex_string;

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
