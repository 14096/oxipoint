use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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

/// A SharePoint site.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    /// Composite id (`host,siteId,webId`).
    pub id: String,
    /// URL segment name.
    #[serde(default)]
    pub name: Option<String>,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified_date_time: Option<String>,
    /// Present (as an empty object) on the root site of a site collection.
    #[serde(default)]
    pub root: Option<Value>,
    /// Site collection information.
    #[serde(default)]
    pub site_collection: Option<SiteCollection>,
}

/// Site collection information of a [`Site`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SiteCollection {
    /// Host name.
    #[serde(default)]
    pub hostname: Option<String>,
    /// Data location code.
    #[serde(default)]
    pub data_location_code: Option<String>,
    /// Present (as an empty object) when the site is a collection root.
    #[serde(default)]
    pub root: Option<Value>,
}

/// A site column.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// Column id.
    pub id: String,
    /// Internal name.
    #[serde(default)]
    pub name: Option<String>,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Column group.
    #[serde(default)]
    pub column_group: Option<String>,
    /// Whether a value is required.
    #[serde(default)]
    pub required: Option<bool>,
    /// Whether the column is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
}

/// A content type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContentType {
    /// Content type id such as `0x0101...`.
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
    /// Whether the content type ships with SharePoint.
    #[serde(default)]
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
    #[serde(default, rename = "grantedToV2")]
    pub granted_to: Option<Value>,
    /// Sharing link, when the permission comes from one.
    #[serde(default)]
    pub link: Option<Value>,
    /// Expiration time (ISO 8601).
    #[serde(default)]
    pub expiration_date_time: Option<String>,
}

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

/// A SharePoint list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct List {
    /// List id.
    pub id: String,
    /// Internal name.
    #[serde(default)]
    pub name: Option<String>,
    /// Display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified_date_time: Option<String>,
    /// Template and flags of the list.
    #[serde(default)]
    pub list: Option<ListFacet>,
}

/// Template and flags of a [`List`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListFacet {
    /// Template such as `genericList` or `documentLibrary`.
    #[serde(default)]
    pub template: Option<String>,
    /// Whether the list is hidden.
    #[serde(default)]
    pub hidden: Option<bool>,
    /// Whether content types are enabled.
    #[serde(default)]
    pub content_types_enabled: Option<bool>,
}

/// One version of a list item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemVersion {
    /// Version id such as `2.0`.
    pub id: String,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified_date_time: Option<String>,
    /// Column values of that version.
    #[serde(default)]
    pub fields: Option<Map<String, Value>>,
}

/// A change-notification subscription.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    /// Subscription id.
    pub id: String,
    /// Watched resource such as `sites/{id}/lists/{id}`.
    #[serde(default)]
    pub resource: Option<String>,
    /// Change types, such as `updated`.
    #[serde(default)]
    pub change_type: Option<String>,
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
    #[serde(default)]
    pub web_url: Option<String>,
    /// Column values (present when `$expand=fields` is used or after create).
    #[serde(default)]
    pub fields: Option<Map<String, Value>>,
    /// Creation time (ISO 8601).
    #[serde(default)]
    pub created_date_time: Option<String>,
    /// Last modification time (ISO 8601).
    #[serde(default)]
    pub last_modified_date_time: Option<String>,
    /// Who created the item.
    #[serde(default)]
    pub created_by: Option<IdentitySet>,
    /// Who last modified the item.
    #[serde(default)]
    pub last_modified_by: Option<IdentitySet>,
    /// The item's content type.
    #[serde(default)]
    pub content_type: Option<ContentType>,
    /// The list the item lives in.
    #[serde(default)]
    pub parent_reference: Option<ParentReference>,
    /// ETag, usable with `update_if_match`.
    #[serde(default)]
    pub e_tag: Option<String>,
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

/// Activity statistics of a list item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemAnalytics {
    /// Statistics since creation.
    #[serde(default)]
    pub all_time: Option<Analytics>,
    /// Statistics of the last seven days.
    #[serde(default)]
    pub last_seven_days: Option<Analytics>,
}

/// One activity on a list item.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemActivity {
    /// Activity id.
    #[serde(default)]
    pub id: Option<String>,
    /// What happened (`comment`, `edit`, `share`, ...), as returned.
    #[serde(default)]
    pub action: Option<Value>,
    /// Who did it.
    #[serde(default)]
    pub actor: Option<IdentitySet>,
    /// When it happened, as returned.
    #[serde(default)]
    pub times: Option<Value>,
}

crate::fields::item_readers!(ListItem, s => s.fields.as_ref());

/// A document library or drive.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// Drive id.
    pub id: String,
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Kind such as `documentLibrary`.
    #[serde(default)]
    pub drive_type: Option<String>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
}

/// A file or folder in a drive.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriveItem {
    /// Item id.
    pub id: String,
    /// File or folder name.
    #[serde(default)]
    pub name: Option<String>,
    /// Size in bytes.
    #[serde(default)]
    pub size: Option<u64>,
    /// Browser URL.
    #[serde(default)]
    pub web_url: Option<String>,
    /// Present for folders.
    #[serde(default)]
    pub folder: Option<Value>,
    /// Present for files.
    #[serde(default)]
    pub file: Option<Value>,
}

impl DriveItem {
    /// Whether the item is a folder.
    pub fn is_folder(&self) -> bool {
        self.folder.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn site_with_extra_and_missing_properties() {
        let s: Site = serde_json::from_value(json!({
            "id":"h,s,w","description":"d","createdDateTime":"2020-01-01T00:00:00Z",
            "lastModifiedDateTime":"2020-02-01T00:00:00Z","root":{},
            "siteCollection":{"hostname":"x.sharepoint.com","unknown":1},"unknown":true
        }))
        .unwrap();
        assert_eq!(s.description.as_deref(), Some("d"));
        assert_eq!(
            s.site_collection.unwrap().hostname.as_deref(),
            Some("x.sharepoint.com")
        );
        assert!(s.root.is_some());
        let s: Site = serde_json::from_value(json!({"id":"a"})).unwrap();
        assert!(s.site_collection.is_none() && s.created_date_time.is_none());
    }

    #[test]
    fn new_models_deserialize() {
        let c: Column =
            serde_json::from_value(json!({"id":"1","name":"n","text":{},"x":1})).unwrap();
        assert_eq!(c.name.as_deref(), Some("n"));
        let t: ContentType = serde_json::from_value(json!({"id":"0x01","isBuiltIn":true})).unwrap();
        assert_eq!(t.is_built_in, Some(true));
        let p: Permission = serde_json::from_value(json!({
            "id":"p","roles":["read"],"grantedToIdentitiesV2":[{"user":{"displayName":"A"}}],"extra":1
        }))
        .unwrap();
        assert_eq!(
            (p.roles, p.granted_to_identities.len()),
            (vec!["read".to_owned()], 1)
        );
        let p: Permission = serde_json::from_value(json!({"id":"p"})).unwrap();
        assert!(p.roles.is_empty());
        let a: Analytics = serde_json::from_value(
            json!({"access":{"actionCount":3,"actorCount":2},"move":{"actionCount":1}}),
        )
        .unwrap();
        assert_eq!(a.access.unwrap().action_count, Some(3));
        assert_eq!(a.moved.unwrap().action_count, Some(1));
    }

    #[test]
    fn list_item_metadata_deserializes() {
        let i: ListItem = serde_json::from_value(json!({
            "id":"7","eTag":"\"abc,2\"","createdDateTime":"2020-01-01T00:00:00Z",
            "lastModifiedDateTime":"2020-02-01T00:00:00Z",
            "createdBy":{"user":{"displayName":"Ann","email":"a@b.c","id":"u1"}},
            "lastModifiedBy":{"application":{"displayName":"App"},"extra":1},
            "contentType":{"id":"0x0100","name":"Item"},
            "parentReference":{"id":"L","siteId":"s1"},"@odata.context":"x"
        }))
        .unwrap();
        assert_eq!(i.e_tag.as_deref(), Some("\"abc,2\""));
        assert_eq!(
            i.created_by.unwrap().user.unwrap().email.as_deref(),
            Some("a@b.c")
        );
        assert_eq!(
            i.last_modified_by
                .unwrap()
                .application
                .unwrap()
                .display_name
                .as_deref(),
            Some("App")
        );
        assert_eq!(i.content_type.unwrap().name.as_deref(), Some("Item"));
        assert_eq!(i.parent_reference.unwrap().site_id.as_deref(), Some("s1"));
        let i: ListItem = serde_json::from_value(json!({"id":"1"})).unwrap();
        assert!(i.created_by.is_none() && i.e_tag.is_none());
        let a: ItemAnalytics = serde_json::from_value(
            json!({"allTime":{"access":{"actionCount":2}},"lastSevenDays":{}}),
        )
        .unwrap();
        assert_eq!(a.all_time.unwrap().access.unwrap().action_count, Some(2));
        let v: ItemActivity = serde_json::from_value(
            json!({"id":"a","action":{"edit":{}},"actor":{"user":{"displayName":"A"}}}),
        )
        .unwrap();
        assert!(v.actor.unwrap().user.is_some());
    }

    #[test]
    fn list_models_deserialize() {
        let l: List = serde_json::from_value(json!({
            "id":"l","description":"d","createdDateTime":"2020-01-01T00:00:00Z",
            "list":{"template":"genericList","hidden":false,"contentTypesEnabled":true},"extra":1
        }))
        .unwrap();
        let f = l.list.unwrap();
        assert_eq!(
            (l.description.as_deref(), f.template.as_deref(), f.hidden),
            (Some("d"), Some("genericList"), Some(false))
        );
        let l: List = serde_json::from_value(json!({"id":"l"})).unwrap();
        assert!(l.list.is_none() && l.created_date_time.is_none());
        let v: ItemVersion =
            serde_json::from_value(json!({"id":"2.0","fields":{"Title":"x"}})).unwrap();
        assert_eq!(v.fields.unwrap()["Title"], "x");
        let s: Subscription = serde_json::from_value(
            json!({"id":"s","resource":"sites/a/lists/b","changeType":"updated","x":1}),
        )
        .unwrap();
        assert_eq!(s.change_type.as_deref(), Some("updated"));
        let s: Subscription = serde_json::from_value(json!({"id":"s"})).unwrap();
        assert!(s.resource.is_none());
        assert_eq!(DeltaPage::default().next_token, None);
    }
}
