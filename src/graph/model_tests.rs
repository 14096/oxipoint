use crate::graph::*;
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
    let c: Column = serde_json::from_value(json!({"id":"1","name":"n","text":{},"x":1})).unwrap();
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
    let a: ItemAnalytics =
        serde_json::from_value(json!({"allTime":{"access":{"actionCount":2}},"lastSevenDays":{}}))
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
