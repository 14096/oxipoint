# List items

```rust
let tasks = sp.list_by_title("Tasks");
```

## Reading

```rust
let open = tasks
    .items()
    .filter("Status eq 'Open'")
    .select(["Id", "Title", "Status"])
    .order_by("Id desc")
    .top(50)
    .list()
    .await?;

for item in open {
    println!("{}: {:?}", item.id, item.fields.get("Title"));
}
```

`ListItem::id` is a number; everything else is in `fields`, keyed by internal name.

One item:

```rust
let item = tasks.item(7).get().await?;
```

## Create, update, delete

```rust
use serde_json::json;

let created = tasks.items().create(&json!({ "Title": "New task" })).await?;
tasks.item(created.id).update(&json!({ "Title": "Renamed" })).await?;
tasks.item(created.id).recycle().await?;      // recoverable
tasks.item(created.id).delete().await?;       // permanent
```

## Optimistic concurrency

```rust
// Fails with an HTTP error if the item changed since you read it
tasks.item(7).update_with_etag(&json!({ "Title": "x" }), "\"3\"").await?;
```

## CAML queries

For complex queries, or when you need fine control over folders and ordering:

```rust
let hits = tasks
    .items()
    .caml("<View><Query><Where><Eq><FieldRef Name='Status'/><Value Type='Text'>Open</Value></Eq></Where></Query></View>")
    .await?
    .collect_all()
    .await?;
```

## Counting

```rust
let n = tasks.items().filter("Status eq 'Open'").count().await?;
```

## Typed values and validated writes

Use [`Fields`](../guides/fields.md) for lookups, people, multi-choice and so on.

SharePoint can also **validate each column** and report per-column errors instead of failing the
whole request:

```rust
use oxipoint::{fields::Fields, sharepoint::ValidateOptions};

let results = tasks
    .item(7)
    .validate_update(&Fields::new().text("Title", "x"), &ValidateOptions::default())
    .await?;

let rejected: Vec<_> = results.iter().filter(|r| !r.is_ok()).collect();
if !rejected.is_empty() {
    eprintln!("{} column(s) rejected", rejected.len());
}
```

`validate_create` does the same for new items. Read values as the user sees them:

```rust
let shown = tasks.item(7).field_values_as_text().await?;      // also _as_html and _for_edit
```

## Attachments

```rust
tasks.item(7).attachments().add("a.txt", "hello").await?;
tasks.item(7).attachments().by_name("a.txt").delete().await?;
```

## Versions

```rust
let versions = tasks.item(7).versions().await?;
tasks.item(7).restore_version("2.0").await?;       // by version label
```

## Likes and comments

```rust
tasks.item(7).like().await?;
tasks.item(7).unlike().await?;
let who = tasks.item(7).liked_by().await?;

let c = tasks.item(7).comments().add("Done").await?;
tasks.item(7).comments().delete(&c.id).await?;
```

## Related entities

```rust
let ct = tasks.item(7).content_type().await?;
let list = tasks.item(7).parent_list().await?;
let file = tasks.item(7).file().await?;        // for document libraries
```

## Item permissions

```rust
tasks.item(7).permissions().break_inheritance(false, false).await?;
```
