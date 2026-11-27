use serde::{Deserialize, Serialize};

/// A directory user.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Object id.
    pub id: String,
    /// Display name.
    pub display_name: Option<String>,
    /// Primary mail address.
    pub mail: Option<String>,
    /// Sign-in name.
    pub user_principal_name: Option<String>,
    /// Given name.
    pub given_name: Option<String>,
    /// Surname.
    pub surname: Option<String>,
    /// Job title.
    pub job_title: Option<String>,
}
