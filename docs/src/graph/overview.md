# Overview

`GraphClient` is the entry point for [Microsoft Graph](https://learn.microsoft.com/graph/). It
builds requests by chaining handles; the last call (`get`, `list`, `create`, ...) sends them.
Collections follow `@odata.nextLink` automatically.

```rust
use oxipoint::graph::GraphClient;

let graph = GraphClient::new(cred);
```

## Map of the API

```text
graph
├── me()                      ─ the signed-in user          → drive()
├── users() / user(id)        ─ directory users
├── sites()                   ─ root(), by_id(), by_path(), search()
│   └── SiteRequest           ─ lists, list(name), pages(), drive(), drives(),
│                               columns(), content_types(), permissions(), analytics(),
│                               sites(), site_by_path()
│       └── ListRequest       ─ items(), item(id), columns(), content_types(), subscriptions()
├── drive(id)                 ─ root(), item(id), path("a/b.txt"), delta()
├── shared_item(url)          ─ open a sharing URL
├── followed_sites()
├── search()                  ─ Microsoft Search
├── batch()                   ─ $batch
└── request(method, path)     ─ generic builder for anything else
```

## Client settings

```rust
let graph = GraphClient::builder(cred)
    .base_url("https://graph.microsoft.com/beta") // default: v1.0
    .max_retries(5)
    .build();
```

- Use `beta` to try preview APIs. They can change without notice; do not depend on them in
  production.
- Use `base_url` to target a national cloud or a mock server in tests.

## Limits of Graph v1.0

Good to know before you plan a feature:

- Graph exposes **neither list views nor list permissions**. Use the
  [SharePoint client](../sharepoint/lists.md) for those.
- Graph has **no way to update or delete a site**, so the Graph client does not offer it. Use
  `sp.web().update(..)` / `sp.web().delete()` instead.
- Web features such as navigation, features, regional settings and the recycle bin are
  SharePoint-only.

See [Choosing between Graph and SharePoint](../guides/choosing-an-api.md).
