# Search, batching and recycle bin

## Search

```rust
let rows = sp.search("contract").select(["Title", "Path"]).all().await?;
println!("{:?}", rows[0].get("Title"));
```

Options:

| Method                  | Purpose                                  |
| ----------------------- | ---------------------------------------- |
| `select([..])`          | Managed properties to return             |
| `row_limit(n)`          | Page size                                |
| `start_row(n)`          | Offset                                   |
| `sort_by(prop, asc)`    | Sort                                     |
| `refiners("..")`        | Request facets                           |
| `trim_duplicates(bool)` | Collapse near-duplicate results          |
| `template(..)`          | Query template                           |

Use `send()` for one page, `all()` to follow paging:

```rust
let page = sp.search("budget").row_limit(10).sort_by("LastModifiedTime", false).send().await?;
```

Rows expose `get(name)` and `properties()`.

## Batching

Send many calls in one round trip. **Changesets** group writes that succeed or fail together:

```rust
use oxipoint::sharepoint::SharePointBatchRequest;
use reqwest::Method;

let mut batch = sp.batch();
batch.add(SharePointBatchRequest::new(Method::GET, "/web/title"))?;
batch.add_changeset([
    SharePointBatchRequest::new(Method::POST, "/web/lists/getbytitle('Tasks')/items")
        .json(&serde_json::json!({ "Title": "A" }))?,
    SharePointBatchRequest::new(Method::POST, "/web/lists/getbytitle('Tasks')/items")
        .json(&serde_json::json!({ "Title": "B" }))?,
])?;

for r in batch.send().await? {
    println!("{} {}", r.is_success(), r.text());
}
```

A throttled call in a batch is reported by `retry_after()` and not retried. Resend it in a new
batch. The multipart batch format is tested against recorded response shapes, so try it on your
tenant before depending on it.

## Recycle bin

SharePoint has two stages: the **first stage** (the site's recycle bin) and the **second stage**
(the site collection bin, where deleted items go from stage one).

```rust
let bin = sp.web().recycle_bin().list().await?;                           // stage 1
let second = sp.web().site_recycle_bin().second_stage().list().await?;   // stage 2
```

Act on one item:

```rust
let item = sp.web().recycle_bin().item("<guid>");
item.restore().await?;
item.move_to_second_stage().await?;
item.delete().await?;                          // permanent
```

Or on many at once:

```rust
sp.web().site_recycle_bin().restore_many(&["<guid-1>", "<guid-2>"]).await?;
sp.web().site_recycle_bin().move_many_to_second_stage(&["<guid-1>"]).await?;
sp.web().site_recycle_bin().delete_many(&["<guid-1>"]).await?;
sp.web().recycle_bin().empty().await?;
```

Recycle bin items carry the item type and state, the author and who deleted them.
