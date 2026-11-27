use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::graph::{Analytics, ContentType, IdentitySet, ParentReference};

/// A SharePoint list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct List {
    /// List id.
    pub id: String,
    /// Internal name.
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
    /// Template and flags of the list.
    pub list: Option<ListFacet>,
}

/// Template and flags of a [`List`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListFacet {
    /// Template such as `genericList` or `documentLibrary`.
    pub template: Option<String>,
    /// Whether the list is hidden.
    pub hidden: Option<bool>,
    /// Whether content types are enabled.
    pub content_types_enabled: Option<bool>,
}

/// One version of a list item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemVersion {
    /// Version id such as `2.0`.
    pub id: String,
    /// Last modification time (ISO 8601).
    pub last_modified_date_time: Option<String>,
    /// Column values of that version.
    pub fields: Option<Map<String, Value>>,
}

/// A change-notification subscription.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    /// Subscription id.
    pub id: String,
    /// Watched resource such as `sites/{id}/lists/{id}`.
    pub resource: Option<String>,
    /// Change types, such as `updated`.
    pub change_type: Option<String>,
    /// Endpoint that receives notifications.
    pub notification_url: Option<String>,
    /// Expiry (ISO 8601).
    pub expiration_date_time: Option<String>,
    /// Opaque value echoed in notifications.
    pub client_state: Option<String>,
}

/// Input for creating a list subscription.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSubscription {
    /// Endpoint that receives notifications (must be HTTPS).
    pub notification_url: String,
    /// Expiry (ISO 8601).
    pub expiration_date_time: String,
    /// Change types; `updated` unless set.
    pub change_type: String,
    /// Opaque value echoed in notifications.
    pub client_state: Option<String>,
}

impl NewSubscription {
    /// A subscription for `updated` changes.
    pub fn new(notification_url: impl Into<String>, expiration: impl Into<String>) -> Self {
        Self {
            notification_url: notification_url.into(),
            expiration_date_time: expiration.into(),
            change_type: "updated".into(),
            client_state: None,
        }
    }
}

/// The result of a delta query.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeltaPage {
    /// Changed items across all pages. Deleted items carry only their id.
    pub items: Vec<ListItem>,
    /// Token to pass to the next `delta` call to receive only newer changes.
    pub next_token: Option<String>,
}

/// An item of a list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    /// Item id.
    pub id: String,
    /// Browser URL.
    pub web_url: Option<String>,
    /// Column values (present when `$expand=fields` is used or after create).
    pub fields: Option<Map<String, Value>>,
    /// Creation time (ISO 8601).
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    pub last_modified_date_time: Option<String>,
    /// Who created the item.
    pub created_by: Option<IdentitySet>,
    /// Who last modified the item.
    pub last_modified_by: Option<IdentitySet>,
    /// The item's content type.
    pub content_type: Option<ContentType>,
    /// The list the item lives in.
    pub parent_reference: Option<ParentReference>,
    /// ETag, usable with `update_if_match`.
    pub e_tag: Option<String>,
}

/// Activity statistics of a list item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemAnalytics {
    /// Statistics since creation.
    pub all_time: Option<Analytics>,
    /// Statistics of the last seven days.
    pub last_seven_days: Option<Analytics>,
}

/// One activity on a list item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemActivity {
    /// Activity id.
    pub id: Option<String>,
    /// What happened (`comment`, `edit`, `share`, ...), as returned.
    pub action: Option<Value>,
    /// Who did it.
    pub actor: Option<IdentitySet>,
    /// When it happened, as returned.
    pub times: Option<Value>,
}

crate::fields::item_readers!(ListItem, s => s.fields.as_ref());
