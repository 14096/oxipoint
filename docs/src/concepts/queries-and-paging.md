# Queries and paging

## Query options

Collection handles accept the OData options both APIs understand:

| Method           | OData      | Purpose                                  |
| ---------------- | ---------- | ---------------------------------------- |
| `select([..])`   | `$select`  | Return only these properties             |
| `filter("..")`   | `$filter`  | Server-side filtering                    |
| `expand("..")`   | `$expand`  | Include related entities (e.g. `fields`) |
| `order_by("..")` | `$orderby` | Sort                                     |
| `top(n)`         | `$top`     | Page size / maximum count                |
| `search("..")`   | `$search`  | Free-text search where supported         |

```rust
let open = sp.list_by_title("Tasks")
    .items()
    .filter("Status eq 'Open'")
    .select(["Id", "Title", "DueDate"])
    .order_by("DueDate desc")
    .top(50)
    .list()
    .await?;
```

## Building filters safely

The `filter!` macro and the `Filter` builder quote and escape values for you:

```rust
use oxipoint::filter;

let name = "O'Brien";
let open = sp.list_by_title("Tasks")
    .items()
    .filter(filter!(Status == "Open" && (Priority >= 3 || Urgent == true) && Author == (name)))
    .list()
    .await?;
// Status eq 'Open' and (Priority ge 3 or Urgent eq true) and Author eq 'O''Brien'
```

| Part        | Syntax                                                                                      |
| ----------- | ------------------------------------------------------------------------------------------- |
| Column      | a bare identifier (`Status`) or a string literal (`"Author/Title"`)                         |
| Comparison  | `==` `!=` `>` `>=` `<` `<=` (become `eq` `ne` `gt` `ge` `lt` `le`)                          |
| Value       | one literal (`"text"`, `3`, `2.5`, `true`) or a parenthesized expression (`(name)`, `(-1)`) |
| Combination | `&&` (and), `\|\|` (or), parentheses; `&&` binds tighter than `\|\|`, as in Rust            |

Strings are always quoted and escaped. Numbers and booleans are written as is.

The same filter without the macro, using the builder:

```rust
use oxipoint::filter::Filter;

let f = Filter::eq("Status", "Open")
    .and(Filter::ge("Priority", 3).or(Filter::eq("Urgent", true)))
    .and(Filter::eq("Author", name));
let starts = Filter::starts_with("Title", "Report");
```

A `Filter` converts into `String`, so it works wherever `filter(..)` is accepted, on both Graph
and SharePoint handles. Dates and functions other than `startswith` are not modelled; pass those
as a raw string (for example `datetime'2026-01-01T00:00:00Z'` on SharePoint).

Entity handles accept `select` and `expand` where they make sense (for example
`pages.by_id(..).expand("canvasLayout")`).

## Paging

Service collections are returned in pages and linked by `@odata.nextLink` (Graph) or `odata.nextLink`
(SharePoint). oxipoint follows them for you.

### Collect everything

```rust
let all = graph.users().list().await?;     // every page, concatenated
```

Convenient, but it holds the whole result in memory and makes as many requests as there are pages.

### Page by page

Use `pages()` (or `pager()` on the generic builder) when a collection is large or you want to
stop early:

```rust
let mut pager = sp.list_by_title("Big").items().top(500).pages();
while let Some(batch) = pager.next_page().await? {
    for item in batch {
        process(item);
    }
}
```

`collect_all()` on a `Pager` finishes the job when you change your mind:

```rust
let mut pager = graph.users().pages();
let first = pager.next_page().await?;          // look at page one
let rest = pager.collect_all().await?;         // then take the remainder
```

### Continuation links are safe

A next-link pointing at a different host than the client's own fails with `Error::Config`
instead of sending your token there.

### CAML on SharePoint

`items().caml(xml)` runs a CAML query and returns a pager too:

```rust
let hits = list
    .items()
    .caml("<View><Query><Where>...</Where></Query></View>")
    .await?
    .collect_all()
    .await?;
```

### Counting without fetching

```rust
let n = sp.list_by_title("Tasks").items().filter("Status eq 'Open'").count().await?;
```
