# Typed column values

SharePoint and Graph represent the same column differently. A lookup is `ProjectId: 3` in one and
`ProjectLookupId: "3"` in the other. `oxipoint::fields::Fields` lets you describe values once and
serialize them for either back end.

## Writing

```rust
use oxipoint::fields::Fields;

let f = Fields::new()
    .text("Title", "Write docs")
    .number("Estimate", 3.5)
    .bool("Done", false)
    .datetime("Due", "2026-12-31T00:00:00Z")
    .choice("Priority", "High")
    .multi_choice("Tags", vec!["a".into(), "b".into()])
    .lookup("Project", 3)                  // by item id of the target list
    .person("AssignedTo", 12)              // by user id
    .url("Link", "https://example.com", Some("Example"));

site.list("Tasks").items().create(&f.for_graph()).await?;
sp.list_by_title("Tasks").items().create(&f.for_sharepoint()).await?;
```

Adding the same column twice **replaces** the earlier value.

## Builders

| Builder                           | Column type                          |
| --------------------------------- | ------------------------------------ |
| `text(col, s)`                    | Single line / multi line of text     |
| `number(col, f64)`                | Number, currency                     |
| `bool(col, b)`                    | Yes/No                               |
| `datetime(col, iso8601)`          | Date and time                        |
| `choice(col, s)`                  | Choice                               |
| `multi_choice(col, vec)`          | Multiple-choice                      |
| `lookup(col, id)`                 | Lookup                               |
| `multi_lookup(col, ids)`          | Multi-value lookup                   |
| `person(col, id)`                 | Person or group                      |
| `multi_person(col, ids)`          | Multiple people                      |
| `taxonomy(col, label, term_guid)` | Managed metadata (single term)       |
| `url(col, address, description)`  | Hyperlink / picture                  |
| `push(col, FieldValue)`           | Anything above, built by hand        |

The column argument is always the column's **internal name**.

## Limits, reported before sending

Managed metadata has a SharePoint form for a **single term only** and no Graph form. Unsupported
combinations return an error **before** any request is made, so you find out early:

```rust
let f = Fields::new().taxonomy("Dept", "Finance", "<term-guid>");
f.to_value(oxipoint::fields::Target::Graph)?;   // Err: managed metadata is not supported on Graph
```

## Reading

Items from both back ends have the same readers. Each returns `None` for a missing column or one of
another kind:

```rust
let item = sp.list_by_title("Tasks").item(7).get().await?;

let title = item.text("Title");                 // Option<String>
let est = item.number("Estimate");              // Option<f64>
let done = item.bool("Done");                   // Option<bool>
let due = item.datetime("Due");                 // parsed ISO 8601 (oxipoint::fields::DateTime)
let project = item.lookup("Project");           // Some(Lookup { id: 3, .. })
let owner = item.person("AssignedTo");
let link = item.url("Link");
let term = item.taxonomy("Dept");
```

`DateTime::parse` and `unix_seconds()` help with date arithmetic.

## Why use it?

- One definition, two back ends.
- Lookups and people get the right wire format on each API.
- Mistakes (like Graph managed metadata) fail early and locally.
