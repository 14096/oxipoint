use std::sync::Arc;

use super::{get_req, post_action};
use crate::{
    Result,
    http::{Body, Pager, QueryOptions, query::query_methods},
    sharepoint::{BasePermissions, Inner, RoleAssignment, RoleDefinition, SiteGroup, SpUser},
};

collection!(
    /// A collection of users.
    UsersRequest => SpUser
);

impl UsersRequest {
    /// A user by id.
    pub fn by_id(&self, id: i64) -> UserRequest {
        UserRequest::new(self.inner.clone(), format!("{}/getbyid({id})", self.base))
    }
}

entity!(
    /// One user.
    UserRequest => SpUser
);

collection!(
    /// The site groups.
    SiteGroupsRequest => SiteGroup
);

impl SiteGroupsRequest {
    /// A group by id.
    pub fn by_id(&self, id: i64) -> SiteGroupRequest {
        SiteGroupRequest::new(self.inner.clone(), format!("{}({id})", self.base))
    }
}

entity!(
    /// One site group.
    SiteGroupRequest => SiteGroup
);

impl SiteGroupRequest {
    /// The members of the group.
    pub fn users(&self) -> UsersRequest {
        UsersRequest::new(self.inner.clone(), format!("{}/users", self.base))
    }
}

collection!(
    /// The role definitions of the web.
    RoleDefinitionsRequest => RoleDefinition
);

/// Role assignments and inheritance of a web.
///
/// Changes here are immediate and affect who can access the web.
#[derive(Debug, Clone)]
pub struct PermissionsRequest {
    inner: Arc<Inner>,
    base: String,
    query: QueryOptions,
}

impl PermissionsRequest {
    pub(crate) fn new(inner: Arc<Inner>, base: String) -> Self {
        Self {
            inner,
            base,
            query: QueryOptions::new(),
        }
    }

    /// The effective permissions of the signed-in user.
    pub async fn effective(&self) -> Result<BasePermissions> {
        self.inner
            .t
            .send(&get_req(
                &format!("{}/effectivebasepermissions", self.base),
                &self.query,
            ))
            .await?
            .json()
    }

    /// Lists the role assignments. Use `expand("Member,RoleDefinitionBindings")` for details.
    pub async fn role_assignments(&self) -> Result<Vec<RoleAssignment>> {
        Pager::new(
            self.inner.t.clone(),
            get_req(&format!("{}/roleassignments", self.base), &self.query),
        )
        .collect_all()
        .await
    }

    /// Grants a role definition to a principal (user or group id).
    pub async fn add_role_assignment(
        &self,
        principal_id: i64,
        role_definition_id: i64,
    ) -> Result<()> {
        post_action(
            &self.inner,
            format!(
                "{}/roleassignments/addroleassignment(principalid={principal_id},roledefid={role_definition_id})",
                self.base
            ),
            Body::None,
        )
        .await
    }

    /// Removes a role definition from a principal.
    pub async fn remove_role_assignment(
        &self,
        principal_id: i64,
        role_definition_id: i64,
    ) -> Result<()> {
        post_action(
            &self.inner,
            format!(
                "{}/roleassignments/removeroleassignment(principalid={principal_id},roledefid={role_definition_id})",
                self.base
            ),
            Body::None,
        )
        .await
    }

    /// Stops inheriting permissions from the parent.
    pub async fn break_inheritance(
        &self,
        copy_role_assignments: bool,
        clear_subscopes: bool,
    ) -> Result<()> {
        post_action(
            &self.inner,
            format!(
                "{}/breakroleinheritance(copyRoleAssignments={copy_role_assignments},clearSubscopes={clear_subscopes})",
                self.base
            ),
            Body::None,
        )
        .await
    }

    /// Resumes inheriting permissions from the parent.
    pub async fn reset_inheritance(&self) -> Result<()> {
        post_action(
            &self.inner,
            format!("{}/resetroleinheritance", self.base),
            Body::None,
        )
        .await
    }
}
query_methods!(PermissionsRequest);
