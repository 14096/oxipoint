# Users and me

## The signed-in user

```rust
let me = graph.me().get().await?;
println!("{} <{}>", me.display_name.unwrap_or_default(), me.mail.unwrap_or_default());
```

`me()` needs a **delegated** credential. With the certificate flow there is no signed-in user, so
use `user(id)` instead.

Request only what you need:

```rust
let me = graph.me().select(["displayName", "mail", "jobTitle"]).get().await?;
```

## Looking up one user

```rust
let user = graph.user("a.smith@contoso.com").get().await?;   // id or user principal name
```

## Listing and filtering users

```rust
let a_users = graph
    .users()
    .filter("startswith(displayName,'A')")
    .select(["id", "displayName", "mail"])
    .order_by("displayName")
    .top(25)
    .list()
    .await?;

for u in a_users {
    println!("{}: {:?}", u.id, u.display_name);
}
```

Large tenants have many users. Page through them instead of collecting everything:

```rust
let mut pager = graph.users().select(["id", "displayName"]).top(100).pages();
while let Some(page) = pager.next_page().await? {
    println!("{} users", page.len());
}
```

## Permissions

Reading other users needs `User.ReadBasic.All` or `User.Read.All`. Reading yourself needs only
`User.Read`.

## OneDrive

`graph.me().drive()` is the signed-in user's OneDrive. See [Drives and files](drives.md).
