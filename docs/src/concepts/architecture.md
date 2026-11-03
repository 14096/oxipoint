# Architecture

## The pieces

```text
 credential ──► client ──► handle ──► handle ──► terminal call ──► HTTP
 (auth::*)     (Graph /    (sites())  (list("T"))  (.list(), .get(),
               SharePoint)                          .create(..))
```

### Credentials

A credential knows how to obtain an access token for a given scope. Every sign-in method
implements the `auth::TokenCredential` trait, so any credential works with either client. You
can also implement the trait yourself ([custom credentials](../auth/tokens.md)).

### Clients

- `GraphClient` talks to `https://graph.microsoft.com/v1.0` (configurable).
- `SharePointClient` is bound to **one site URL** and talks to its `/_api` endpoint.

Each client wraps your credential in a cache and owns a shared HTTP transport. Clients are cheap
to use from many tasks: concurrent requests share one sign-in.

### Handles

Handles are small, lightweight values that describe *where* you are in the API. They do not
perform I/O. You get them by chaining:

```rust
let tasks = graph
    .sites()
    .by_path("contoso.sharepoint.com", "/sites/team")   // site handle
    .list("Tasks");                                      // list handle
let items = tasks.items();                               // collection handle
let one = tasks.item("7");                               // entity handle
```

Because they are cheap and borrow nothing from the network, you can store them in variables and
reuse them, as the examples do with `tasks`.

### Terminal calls

The last call in a chain actually sends the request. These are `async` and return
`oxipoint::Result<T>`:

| Terminal call                       | Used on        | Meaning                                  |
| ----------------------------------- | -------------- | ---------------------------------------- |
| `get()`                             | entities       | Read one thing                           |
| `list()`                            | collections    | Read every page and collect the results  |
| `pages()`                           | collections    | Read page by page ([paging](queries-and-paging.md)) |
| `create(&body)`                     | collections    | Create, returns the new entity           |
| `update(&body)`                     | entities       | Change properties                        |
| `delete()`                          | entities       | Remove                                   |

Everything before the terminal call is plain builder code, so you can build the chain across
several statements and branch on conditions.

## One transport, one place for behavior

All handles share one `Transport`, which:

1. attaches the bearer token (and refreshes it when it is about to expire),
2. retries throttled responses,
3. maps non-2xx answers to `Error::Http`,
4. refuses to send your token to a foreign host.

Because the [generic request builder](../guides/generic-requests.md) uses the same transport,
endpoints without a typed API get the same behavior.

## Concurrency

Clients are `Send + Sync`-friendly handles over an `Arc`. Share a client by reference, or wrap it in
an `Arc` to move it into spawned tasks:

```rust
use std::sync::Arc;

let sp = Arc::new(SharePointClient::new(site_url, cred)?);
let mut tasks = Vec::new();
for id in 1..=5 {
    let sp = sp.clone();
    tasks.push(tokio::spawn(async move {
        sp.list_by_title("Tasks").item(id).get().await
    }));
}
for t in tasks {
    let item = t.await.unwrap()?;
    println!("{}", item.id);
}
```

Be mindful of throttling when fanning out: many parallel calls will trigger `429` responses. The
client retries them, but you should also bound your concurrency (for example with a
`tokio::sync::Semaphore`) or use [batching](../graph/sharing-search-batch.md).
