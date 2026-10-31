use serde::{Deserialize, Serialize};

/// A directory user.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Object id.
    pub id: String,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Primary mail address.
    #[serde(default)]
    pub mail: Option<String>,
    /// Sign-in name.
    #[serde(default)]
    pub user_principal_name: Option<String>,
    /// Given name.
    #[serde(default)]
    pub given_name: Option<String>,
    /// Surname.
    #[serde(default)]
    pub surname: Option<String>,
    /// Job title.
    #[serde(default)]
    pub job_title: Option<String>,
}
