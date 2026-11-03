# Users, groups and permissions

## Users

```rust
let me = sp.web().current_user().await?;
println!("{} ({:?})", me.id, me.email);

// Make sure a user exists in this site's user list and get their local id
let user = sp.web().ensure_user("i:0#.f|membership|a@contoso.com").await?;

let site_users = sp.web().users().list().await?;
let one = sp.web().users().by_id(user.id).get().await?;
```

SharePoint has **its own numeric user ids** per site collection. You need them for person columns
and role assignments. `ensure_user` is the standard way to get one from a login name.

## Groups

```rust
let groups = sp.web().groups().list().await?;
let members = sp.web().groups().by_id(5).users().list().await?;
```

## What can the current user do?

```rust
use oxipoint::sharepoint::PermissionKind;

let can_edit = sp.web().permissions().effective().await?.has(PermissionKind::EditListItems);
if !can_edit {
    eprintln!("read-only access");
}
```

Use this to show or hide actions in your app *before* the user hits a `403`.

## Role definitions and assignments

```rust
let roles = sp.web().role_definitions().list().await?;
let assignments = sp.web().permissions().role_assignments().await?;

// Give user `user.id` the role definition 1073741827 (Contribute)
sp.web().permissions().add_role_assignment(user.id, 1073741827).await?;
sp.web().permissions().remove_role_assignment(user.id, 1073741827).await?;
```

## Breaking inheritance

By default a list inherits permissions from its web, and an item from its list. To give something
unique permissions:

```rust
// copy the current assignments, keep the parent's scope links
sp.web().permissions().break_inheritance(true, false).await?;

// ...and to undo it
sp.web().permissions().reset_inheritance().await?;
```

The same works for lists and items:

```rust
sp.list_by_title("Tasks").permissions().break_inheritance(true, false).await?;
sp.list_by_title("Tasks").item(7).permissions().break_inheritance(false, false).await?;
```

> **Destructive.** Breaking inheritance and then resetting it discards the unique permissions
> you had set. Decide whether to copy the existing assignments (first argument) deliberately. Mind
> the 50,000 unique-scope-per-list limit: per-item permissions do not scale.
