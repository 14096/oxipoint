use crate::sharepoint::*;
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
    let s: Subscription = serde_json::from_value(
        json!({"id":"s","notificationUrl":"https://n","expirationDateTime":"2030-01-01T00:00:00Z"}),
    )
    .unwrap();
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
    let u: SpUser = serde_json::from_value(json!({"Id":5,"LoginName":"i:0#.f|m|a@b.com"})).unwrap();
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
