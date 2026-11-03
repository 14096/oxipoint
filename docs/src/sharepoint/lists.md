# Lists, fields, views and content types

## Reading lists

```rust
let lists = sp.lists().select(["Id", "Title", "ItemCount"]).list().await?;
for l in lists {
    println!("{:?}: {:?} items", l.title, l.item_count);
}

let tasks = sp.list_by_title("Tasks").get().await?;     // by title
let same = sp.list_by_id("<list-guid>").get().await?;   // by id
```

## Creating a list

```rust
use oxipoint::sharepoint::{ListTemplate, NewList};

let books = sp
    .lists()
    .create(&NewList::new("Books", ListTemplate::GenericList).description("Reading list"))
    .await?;
```

Templates: `GenericList`, `DocumentLibrary`, `Survey`, `Links`, `Announcements`, `Contacts`,
`Events`, `Tasks`, `DiscussionBoard`, `PictureLibrary`, or `Custom(i32)` for any other base template.

## Updating and removing

```rust
use serde_json::json;

sp.list_by_title("Books").update(&json!({ "Description": "Updated" })).await?;

sp.list_by_title("Books").recycle().await?;   // move to the recycle bin (restorable)
sp.list_by_title("Books").delete().await?;    // remove for good
```

Prefer `recycle()` in application code so mistakes are recoverable.

## Fields (columns)

```rust
let fields = sp.list_by_title("Tasks").fields();

let all = fields.list().await?;
let status = fields.by_internal_name("Status").get().await?;
let by_title = fields.by_title("Due date").get().await?;

// Create: FieldTypeKind 2=Text, 4=DateTime, 8=Boolean, 9=Number
fields.create(&json!({ "Title": "Due", "FieldTypeKind": 4 })).await?;
fields.by_internal_name("Due").update(&json!({ "Description": "Deadline" })).await?;
fields.by_internal_name("Due").delete().await?;
```

> The **internal name** is fixed at creation (spaces become `_x0020_`). Renaming a column later
> changes only its display title. Always address columns by internal name in code.

## Views

```rust
let views = sp.list_by_title("Tasks").views();

let all = views.list().await?;
let fields_in_view = views.by_title("All Items").fields().await?;
```

Views can also be created, updated and deleted:

```rust
views.create(&json!({ "Title": "Open tasks", "Query": "<Where>...</Where>" })).await?;
```

## Content types

```rust
let types = sp.list_by_title("Tasks").content_types();

sp.list_by_title("Tasks").content_types().add_from_web("0x0100...").await?;  // attach a site content type
let list_types = types.list().await?;
```

## Change tracking

`get_changes` returns what happened to a list since a token, similar to Graph's delta:

```rust
use oxipoint::sharepoint::ChangeQuery;

let list = sp.list_by_title("Tasks");
let first = list.get_changes(&ChangeQuery::items()).await?;
let token = first.last().map(|c| c.change_token.clone());

// Later
if let Some(t) = token {
    let newer = list.get_changes(&ChangeQuery::items().since(t)).await?;
    for c in newer {
        println!("change {} on item {:?}", c.change_type, c.item_id);
    }
}
```

## Webhooks

```rust
use oxipoint::sharepoint::NewSubscription;

let list = sp.list_by_title("Tasks");
let sub = list
    .subscriptions()
    .add(&NewSubscription::new("https://example.com/hook", "2030-01-01T00:00:00Z"))
    .await?;

list.subscriptions().list().await?;
list.subscriptions().delete(&sub.id).await?;
```

As with Graph, a webhook only says *something changed*; follow up with `get_changes`.
