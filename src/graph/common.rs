use serde::{Deserialize, Serialize};

/// Number of actions and distinct actors in a period.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityCounts {
    /// Number of actions.
    pub action_count: Option<u64>,
    /// Number of distinct actors.
    pub actor_count: Option<u64>,
}

/// Activity statistics of a site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Analytics {
    /// Start of the period (ISO 8601).
    pub start_date_time: Option<String>,
    /// End of the period (ISO 8601).
    pub end_date_time: Option<String>,
    /// Views and similar access.
    pub access: Option<ActivityCounts>,
    /// Creations.
    pub create: Option<ActivityCounts>,
    /// Deletions.
    pub delete: Option<ActivityCounts>,
    /// Edits.
    pub edit: Option<ActivityCounts>,
    /// Moves.
    #[serde(rename = "move")]
    pub moved: Option<ActivityCounts>,
}

/// A user, application or device as shown in item metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    /// Object id, when known.
    pub id: Option<String>,
    /// Display name.
    pub display_name: Option<String>,
    /// Mail address (users).
    pub email: Option<String>,
}

/// The identities behind an action.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IdentitySet {
    /// A user.
    pub user: Option<Identity>,
    /// An application.
    pub application: Option<Identity>,
    /// A device.
    pub device: Option<Identity>,
    /// A group.
    pub group: Option<Identity>,
}

/// Where an item lives.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParentReference {
    /// Id of the parent (the list or folder).
    pub id: Option<String>,
    /// Path of the parent.
    pub path: Option<String>,
    /// Id of the site.
    pub site_id: Option<String>,
    /// Id of the drive (drive items).
    pub drive_id: Option<String>,
}
