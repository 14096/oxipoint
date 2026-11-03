# Sharing, search and batching

## Sharing links

Create a link that gives access to an item:

```rust
use oxipoint::graph::{LinkOptions, LinkScope, LinkType};

let link = report
    .create_link(&LinkOptions::new(LinkType::View).scope(LinkScope::Organization))
    .await?;
println!("{:?}", link.link_url());
```

| `LinkType` | Grants               |
| ---------- | -------------------- |
| `View`     | read-only            |
| `Edit`     | read and write       |
| `Embed`    | an embeddable read-only link (OneDrive personal) |

| `LinkScope`    | Who can use the link             |
| -------------- | -------------------------------- |
| `Anonymous`    | anyone with the link             |
| `Organization` | anyone in your organization      |
| `Users`        | only the specific people invited |

Options: `.expires("2030-01-01T00:00:00Z")`, `.password("...")` and
`.retain_inherited_permissions(bool)`.

## Invitations

Share with specific people and optionally send them an email:

```rust
use oxipoint::graph::Invite;

let perms = report
    .invite(
        &Invite::new(["read"])
            .email("a@contoso.com")
            .message("Here is the report")
            .require_sign_in(true)
            .send_invitation(true),
    )
    .await?;
```

## Inspect and change permissions

```rust
let all = report.permissions().list().await?;
report.permission("<permission-id>").update_roles(["write"]).await?;
```

## Open a sharing URL

Resolve a link someone sent you into a drive item:

```rust
let item = graph.shared_item("https://contoso.sharepoint.com/:x:/s/team/abc").get().await?;
```

## Search

Microsoft Search queries drive items, list items and sites:

```rust
let hits = graph
    .search()
    .entity_types(["driveItem"])
    .query("budget")
    .all()                     // follows paging
    .await?;

for hit in hits {
    println!("{:?}: {:?}", hit.rank, hit.summary);
}
```

One page at a time, with a window:

```rust
let page = graph.search().entity_types(["listItem"]).query("contract").from(0).size(25).send().await?;
println!("{:?} total, more: {}", page.total, page.more_results_available);
```

`hit.resource` holds the raw JSON; `hit.resource_as::<T>()` deserializes it into your own type.

## Batching

A `$batch` sends **up to 20 calls in one request** and returns one result per call. A failed
call does **not** fail the batch.

```rust
use oxipoint::graph::GraphBatchRequest;
use reqwest::Method;

let mut batch = graph.batch();
batch.add(GraphBatchRequest::new(Method::GET, "/me"))?;
batch.add(GraphBatchRequest::new(Method::GET, "/me/drive"))?;

for result in batch.send().await? {
    if result.is_success() {
        let value: serde_json::Value = result.json()?;
        println!("{value}");
    } else {
        eprintln!("failed: {:?}", result.error());
    }
}
```

### Dependencies

`add` returns the request's id. Use `depends_on` to order calls:

```rust
let mut batch = graph.batch();
let create = batch.add(GraphBatchRequest::new(Method::POST, "/me/drive/root/children")
    .json(&serde_json::json!({ "name": "New", "folder": {} }))?)?;
batch.add(GraphBatchRequest::new(Method::GET, "/me/drive/root/children").depends_on(create))?;
```

### Throttling inside batches

A throttled call (429) is reported with `result.retry_after()` and **not retried**. Collect those
and resend them in a new batch after the indicated delay.
