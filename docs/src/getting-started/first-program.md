# Your first program

This walkthrough signs in with the **device code** flow, which needs no browser on the machine
running the code, then reads data from both back ends.

## 1. Sign in and call Graph

```rust
use oxipoint::{auth::DeviceCodeCredential, graph::GraphClient};

#[tokio::main]
async fn main() -> oxipoint::Result<()> {
    let tenant = std::env::var("OXIPOINT_TENANT").expect("OXIPOINT_TENANT");
    let client_id = std::env::var("OXIPOINT_CLIENT_ID").expect("OXIPOINT_CLIENT_ID");

    let cred = DeviceCodeCredential::new(&tenant, &client_id, |info| {
        // Show this to the user: it contains the URL and the code to type.
        println!("{}", info.message);
    });
    let graph = GraphClient::new(cred);

    let me = graph.me().select(["displayName", "mail"]).get().await?;
    println!("Signed in as {}", me.display_name.unwrap_or_default());
    Ok(())
}
```

On first run the program prints something like _"To sign in, open https://microsoft.com/devicelogin
and enter the code ABC123"_. Once you complete it, the call proceeds.

## 2. Read a SharePoint site with Graph

```rust
let site = graph.sites().by_path("contoso.sharepoint.com", "/sites/team");
for list in site.lists().list().await? {
    println!("{:?}", list.display_name);
}
```

## 3. Do the same with SharePoint REST

```rust
use oxipoint::sharepoint::SharePointClient;

let sp = SharePointClient::new("https://contoso.sharepoint.com/sites/team", cred)?;
let web = sp.web().get().await?;
println!("{:?}", web.title);
let lists = sp.lists().select(["Id", "Title", "ItemCount"]).list().await?;
```

> A credential is **not** `Clone` (except `PasswordCredential`). To use both clients in one
> program, create two credential instances, or pick one API. See
> [tokens and security](../auth/tokens.md).

## 4. Create item

```rust
use serde_json::json;

let item = sp.list_by_title("Tasks")
    .items()
    .create(&json!({ "Title": "My first item" }))
    .await?;
println!("Created item {}", item.id);
```
