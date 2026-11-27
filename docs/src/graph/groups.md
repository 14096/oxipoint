# Groups

Microsoft 365 groups back most team sites, and Graph also exposes security groups.

```rust
use serde_json::json;

let groups = graph.groups().filter("startswith(displayName,'Project')").list().await?;
for g in &groups {
    println!("{} ({})", g.id, if g.is_microsoft_365() { "Microsoft 365" } else { "other" });
}

let team = graph.group("<group-id>");
let info = team.get().await?;
team.update(&json!({ "description": "Project Falcon" })).await?;
team.delete().await?;

let created = graph
    .groups()
    .create(&json!({
        "displayName": "Falcon",
        "mailNickname": "falcon",
        "mailEnabled": true,
        "securityEnabled": false,
        "groupTypes": ["Unified"],
    }))
    .await?;
```

## Members and owners

```rust
let members = team.members().list().await?;       // users, groups, devices...
let owners = team.owners().list().await?;
let parents = team.member_of().list().await?;

team.members().add("<user-id>").await?;
team.owners().remove("<user-id>").await?;
```

`members()` and `owners()` return [`DirectoryObject`]s; check `odata_type` (for example
`#microsoft.graph.user`) to tell them apart.

## The group's site and library

```rust
let site = team.site();                    // /groups/{id}/sites/root
let lists = site.lists().list().await?;
let files = team.drive().root().children().await?;
```

## Users: manager, reports, memberships and photo

`graph.me()` and `graph.user(id)` share the same relationship helpers:

```rust
let boss = graph.me().manager().await?;
let reports = graph.user("a@contoso.com").direct_reports().await?;
let groups = graph.me().member_of().await?;
let jpeg = graph.me().photo().await?;      // bytes

graph.me().follow_sites(["<site-id>"]).await?;
graph.me().unfollow_sites(["<site-id>"]).await?;
```

## Permissions

Reading groups needs `Group.Read.All` (or `GroupMember.Read.All` for memberships); changing them
needs `Group.ReadWrite.All`. Creating Microsoft 365 groups typically needs an admin.
