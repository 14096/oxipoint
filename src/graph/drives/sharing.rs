use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    Result,
    graph::*,
    http::{Body, RawRequest, encode_path},
};

/// What a sharing link allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkType {
    /// Read-only.
    View,
    /// Read and write.
    Edit,
    /// An embeddable read-only link (OneDrive personal).
    Embed,
}

impl LinkType {
    fn as_str(self) -> &'static str {
        match self {
            LinkType::View => "view",
            LinkType::Edit => "edit",
            LinkType::Embed => "embed",
        }
    }
}

/// Who can use a sharing link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkScope {
    /// Anyone with the link.
    Anonymous,
    /// Anyone in the organization.
    Organization,
    /// Only the people the link is shared with.
    Users,
}

impl LinkScope {
    fn as_str(self) -> &'static str {
        match self {
            LinkScope::Anonymous => "anonymous",
            LinkScope::Organization => "organization",
            LinkScope::Users => "users",
        }
    }
}

/// Settings of a sharing link for `DriveItemRequest::create_link`.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkOptions {
    link_type: LinkType,
    scope: Option<LinkScope>,
    expiration: Option<String>,
    password: Option<String>,
    retain_inherited: Option<bool>,
}

impl LinkOptions {
    /// A link of the given type. Without a scope the tenant's default applies.
    pub fn new(link_type: LinkType) -> Self {
        Self {
            link_type,
            scope: None,
            expiration: None,
            password: None,
            retain_inherited: None,
        }
    }

    /// Sets who can use the link.
    pub fn scope(mut self, scope: LinkScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Sets when the link expires (ISO 8601, for example `2030-01-01T00:00:00Z`).
    pub fn expires(mut self, when: impl Into<String>) -> Self {
        self.expiration = Some(when.into());
        self
    }

    /// Protects the link with a password (OneDrive personal only).
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Keeps the permissions the item inherits when the link is created.
    pub fn retain_inherited_permissions(mut self, retain: bool) -> Self {
        self.retain_inherited = Some(retain);
        self
    }

    fn to_json(&self) -> Value {
        let mut v = json!({ "type": self.link_type.as_str() });
        if let Some(s) = self.scope {
            v["scope"] = s.as_str().into();
        }
        if let Some(e) = &self.expiration {
            v["expirationDateTime"] = e.clone().into();
        }
        if let Some(p) = &self.password {
            v["password"] = p.clone().into();
        }
        if let Some(r) = self.retain_inherited {
            v["retainInheritedPermissions"] = r.into();
        }
        v
    }
}

/// An invitation to a drive item for `DriveItemRequest::invite`.
#[derive(Debug, Clone, PartialEq)]
pub struct Invite {
    recipients: Vec<Value>,
    roles: Vec<String>,
    message: Option<String>,
    require_sign_in: bool,
    send_invitation: bool,
    retain_inherited: Option<bool>,
}

impl Invite {
    /// An invitation granting `roles` (`read` or `write`) that requires sign-in and sends an
    /// email.
    pub fn new<I, S>(roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            recipients: Vec::new(),
            roles: roles.into_iter().map(Into::into).collect(),
            message: None,
            require_sign_in: true,
            send_invitation: true,
            retain_inherited: None,
        }
    }

    /// Invites a person by email address.
    pub fn email(mut self, address: impl Into<String>) -> Self {
        self.recipients.push(json!({ "email": address.into() }));
        self
    }

    /// Invites a user or group by object id.
    pub fn object_id(mut self, id: impl Into<String>) -> Self {
        self.recipients.push(json!({ "objectId": id.into() }));
        self
    }

    /// Sets the text of the invitation.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Whether recipients must sign in to use the item (default true).
    pub fn require_sign_in(mut self, yes: bool) -> Self {
        self.require_sign_in = yes;
        self
    }

    /// Whether the service sends the invitation email (default true).
    pub fn send_invitation(mut self, yes: bool) -> Self {
        self.send_invitation = yes;
        self
    }

    /// Keeps the permissions the item inherits.
    pub fn retain_inherited_permissions(mut self, retain: bool) -> Self {
        self.retain_inherited = Some(retain);
        self
    }

    fn to_json(&self) -> Value {
        let mut v = json!({
            "recipients": self.recipients,
            "roles": self.roles,
            "requireSignIn": self.require_sign_in,
            "sendInvitation": self.send_invitation,
        });
        if let Some(m) = &self.message {
            v["message"] = m.clone().into();
        }
        if let Some(r) = self.retain_inherited {
            v["retainInheritedPermissions"] = r.into();
        }
        v
    }
}

#[derive(Deserialize)]
struct Invited {
    #[serde(default)]
    value: Vec<Permission>,
}

collection!(
    /// The permissions of a drive item.
    ItemPermissionsRequest => Permission
);

impl ItemPermissionsRequest {
    /// A permission by id.
    pub fn by_id(&self, id: &str) -> ItemPermissionRequest {
        ItemPermissionRequest::new(self.t.clone(), format!("{}/{}", self.base, encode_path(id)))
    }
}

entity!(
    /// One permission of a drive item. Deleting it revokes the access it granted.
    ItemPermissionRequest => Permission
);

impl ItemPermissionRequest {
    /// Replaces the roles of the permission and returns the result.
    pub async fn update_roles<I, S>(&self, roles: I) -> Result<Permission>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let roles: Vec<String> = roles.into_iter().map(Into::into).collect();
        let mut req = RawRequest::new(Method::PATCH, self.base.as_str());
        req.body = Body::Json(json!({ "roles": roles }));
        self.t.send(&req).await?.json()
    }
}

impl DriveItemRequest {
    /// Creates a sharing link and returns it as a permission; see [`Permission::link_url`].
    pub async fn create_link(&self, options: &LinkOptions) -> Result<Permission> {
        let mut req = RawRequest::new(Method::POST, format!("{}/createLink", self.base));
        req.body = Body::Json(options.to_json());
        self.t.send(&req).await?.json()
    }

    /// Grants access to people and returns the permissions that were created.
    pub async fn invite(&self, invite: &Invite) -> Result<Vec<Permission>> {
        let mut req = RawRequest::new(Method::POST, format!("{}/invite", self.base));
        req.body = Body::Json(invite.to_json());
        Ok(self.t.send(&req).await?.json::<Invited>()?.value)
    }

    /// The permissions on this item.
    pub fn permissions(&self) -> ItemPermissionsRequest {
        ItemPermissionsRequest::new(self.t.clone(), format!("{}/permissions", self.base))
    }

    /// One permission of this item by id.
    pub fn permission(&self, id: &str) -> ItemPermissionRequest {
        self.permissions().by_id(id)
    }
}

/// Encodes a sharing URL as the share id Graph expects (`u!` and unpadded base64url).
pub(crate) fn encode_share_url(url: &str) -> String {
    format!("u!{}", URL_SAFE_NO_PAD.encode(url))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{body_json, method, path},
    };

    use super::*;
    use crate::graph::testutil::client;

    #[tokio::test]
    async fn anonymous_view_link() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/drives/d/items/i/createLink"))
            .and(body_json(json!({"type":"view","scope":"anonymous"})))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id":"p1","roles":["read"],
                "link":{"type":"view","scope":"anonymous","webUrl":"https://x.sharepoint.com/:b:/s/t/abc"}
            })))
            .expect(1)
            .mount(&s)
            .await;
        let p = c
            .drive("d")
            .item("i")
            .create_link(&LinkOptions::new(LinkType::View).scope(LinkScope::Anonymous))
            .await
            .unwrap();
        assert_eq!(p.id, "p1");
        assert_eq!(p.link_url(), Some("https://x.sharepoint.com/:b:/s/t/abc"));
        assert_eq!(
            (p.link_type(), p.link_scope()),
            (Some("view"), Some("anonymous"))
        );
    }

    #[tokio::test]
    async fn link_with_expiration_and_password() {
        let (c, s) = client().await;
        Mock::given(path("/drives/d/items/i/createLink"))
            .and(body_json(json!({
                "type":"edit","scope":"organization","expirationDateTime":"2030-01-01T00:00:00Z",
                "password":"pw","retainInheritedPermissions":false
            })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"p2","hasPassword":true})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let p = c
            .drive("d")
            .item("i")
            .create_link(
                &LinkOptions::new(LinkType::Edit)
                    .scope(LinkScope::Organization)
                    .expires("2030-01-01T00:00:00Z")
                    .password("pw")
                    .retain_inherited_permissions(false),
            )
            .await
            .unwrap();
        assert_eq!(p.has_password, Some(true));
    }

    #[tokio::test]
    async fn a_disabled_scope_is_a_403() {
        let (c, s) = client().await;
        Mock::given(path("/drives/d/items/i/createLink"))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "error":{"code":"accessDenied","message":"Anonymous links are disabled"}
            })))
            .mount(&s)
            .await;
        let e = c
            .drive("d")
            .item("i")
            .create_link(&LinkOptions::new(LinkType::View).scope(LinkScope::Anonymous))
            .await
            .unwrap_err();
        assert_eq!(e.status(), Some(403));
    }

    #[tokio::test]
    async fn invite_by_email_and_without_the_email() {
        let (c, s) = client().await;
        Mock::given(method("POST"))
            .and(path("/drives/d/items/i/invite"))
            .and(body_json(json!({
                "recipients":[{"email":"a@contoso.com"}],"roles":["read"],
                "requireSignIn":true,"sendInvitation":true
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value":[{"id":"p3","roles":["read"],"grantedToV2":{"user":{"displayName":"A"}}}]
            })))
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(path("/drives/d/items/j/invite"))
            .and(body_json(json!({
                "recipients":[{"objectId":"u1"}],"roles":["write"],"message":"hi",
                "requireSignIn":false,"sendInvitation":false
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[]})))
            .expect(1)
            .mount(&s)
            .await;
        let created = c
            .drive("d")
            .item("i")
            .invite(&Invite::new(["read"]).email("a@contoso.com"))
            .await
            .unwrap();
        assert_eq!(created[0].id, "p3");
        c.drive("d")
            .item("j")
            .invite(
                &Invite::new(["write"])
                    .object_id("u1")
                    .message("hi")
                    .require_sign_in(false)
                    .send_invitation(false),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn list_update_and_revoke_permissions() {
        let (c, s) = client().await;
        Mock::given(method("GET"))
            .and(path("/drives/d/items/i/permissions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value":[{"id":"p1","roles":["read"]},{"id":"p2","roles":["write"]}],
                "@odata.nextLink": format!("{}/drives/d/items/i/permissions?page=2", s.uri())
            })))
            .up_to_n_times(1)
            .mount(&s)
            .await;
        Mock::given(method("GET"))
            .and(path("/drives/d/items/i/permissions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"value":[{"id":"p3"}]})))
            .mount(&s)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/drives/d/items/i/permissions/p1"))
            .and(body_json(json!({"roles":["write"]})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"p1","roles":["write"]})),
            )
            .expect(1)
            .mount(&s)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/drives/d/items/i/permissions/p2"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&s)
            .await;
        let item = c.drive("d").item("i");
        assert_eq!(item.permissions().list().await.unwrap().len(), 3);
        let p = item.permission("p1").update_roles(["write"]).await.unwrap();
        assert_eq!(p.roles, ["write"]);
        item.permission("p2").delete().await.unwrap();
    }

    #[test]
    fn share_ids_match_the_documented_encoding() {
        assert_eq!(
            encode_share_url(
                "https://onedrive.live.com/redir?resid=1231244193912!12&authkey=!1243234"
            ),
            "u!aHR0cHM6Ly9vbmVkcml2ZS5saXZlLmNvbS9yZWRpcj9yZXNpZD0xMjMxMjQ0MTkzOTEyITEyJmF1dGhrZXk9ITEyNDMyMzQ"
        );
    }

    #[tokio::test]
    async fn resolves_a_sharing_url_and_reports_an_unresolvable_one() {
        let (c, s) = client().await;
        let url = "https://contoso.sharepoint.com/:x:/s/t/abc";
        let id = encode_share_url(url);
        Mock::given(method("GET"))
            .and(path(format!("/shares/{id}/driveItem")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"id":"i9","name":"book.xlsx"})),
            )
            .expect(1)
            .mount(&s)
            .await;
        let item = c.shared_item(url).get().await.unwrap();
        assert_eq!(item.name.as_deref(), Some("book.xlsx"));

        let e = c
            .shared_item("https://contoso.sharepoint.com/nope")
            .get()
            .await
            .unwrap_err();
        assert_eq!(e.status(), Some(404));
    }
}
