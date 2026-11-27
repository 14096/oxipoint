use serde::{Deserialize, Serialize};

/// A Microsoft 365 group, security group or Teams-backed group.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    /// Object id.
    pub id: String,
    /// Display name.
    pub display_name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Mail address.
    pub mail: Option<String>,
    /// Mail alias.
    pub mail_nickname: Option<String>,
    /// Whether the group can receive mail.
    pub mail_enabled: Option<bool>,
    /// Whether the group can be used to assign permissions.
    pub security_enabled: Option<bool>,
    /// Group kinds; contains `Unified` for a Microsoft 365 group.
    #[serde(default)]
    pub group_types: Vec<String>,
    /// `Public`, `Private` or `HiddenMembership`.
    pub visibility: Option<String>,
    /// Creation time (ISO 8601).
    pub created_date_time: Option<String>,
}

impl Group {
    /// Whether this is a Microsoft 365 (`Unified`) group.
    pub fn is_microsoft_365(&self) -> bool {
        self.group_types.iter().any(|t| t == "Unified")
    }
}

/// A directory object such as a user, group or device, as returned by membership and
/// relationship queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryObject {
    /// Object id.
    pub id: String,
    /// Concrete type, for example `#microsoft.graph.user`.
    #[serde(rename = "@odata.type")]
    pub odata_type: Option<String>,
    /// Display name.
    pub display_name: Option<String>,
    /// Mail address (users and groups).
    pub mail: Option<String>,
    /// Sign-in name (users only).
    pub user_principal_name: Option<String>,
}
