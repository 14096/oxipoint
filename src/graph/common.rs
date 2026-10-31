use serde::{Deserialize, Serialize};

/// Number of actions and distinct actors in a period.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityCounts {
    /// Number of actions.
    #[serde(default)]
    pub action_count: Option<u64>,
    /// Number of distinct actors.
    #[serde(default)]
    pub actor_count: Option<u64>,
}

/// Activity statistics of a site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Analytics {
    /// Start of the period (ISO 8601).
    #[serde(default)]
    pub start_date_time: Option<String>,
    /// End of the period (ISO 8601).
    #[serde(default)]
    pub end_date_time: Option<String>,
    /// Views and similar access.
    #[serde(default)]
    pub access: Option<ActivityCounts>,
    /// Creations.
    #[serde(default)]
    pub create: Option<ActivityCounts>,
    /// Deletions.
    #[serde(default)]
    pub delete: Option<ActivityCounts>,
    /// Edits.
    #[serde(default)]
    pub edit: Option<ActivityCounts>,
    /// Moves.
    #[serde(default, rename = "move")]
    pub moved: Option<ActivityCounts>,
}

/// A user, application or device as shown in item metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    /// Object id, when known.
    #[serde(default)]
    pub id: Option<String>,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Mail address (users).
    #[serde(default)]
    pub email: Option<String>,
}

/// The identities behind an action.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IdentitySet {
    /// A user.
    #[serde(default)]
    pub user: Option<Identity>,
    /// An application.
    #[serde(default)]
    pub application: Option<Identity>,
    /// A device.
    #[serde(default)]
    pub device: Option<Identity>,
    /// A group.
    #[serde(default)]
    pub group: Option<Identity>,
}

/// Where an item lives.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParentReference {
    /// Id of the parent (the list or folder).
    #[serde(default)]
    pub id: Option<String>,
    /// Path of the parent.
    #[serde(default)]
    pub path: Option<String>,
    /// Id of the site.
    #[serde(default)]
    pub site_id: Option<String>,
}
