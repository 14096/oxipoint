# Delta sync and subscriptions

## Delta: incremental sync

Instead of re-reading a whole list or drive, ask only for what changed since your last call.

```rust
// First sync: returns everything and a token
let first = tasks.items().delta(None).await?;
for item in &first.items {
    println!("{}", item.id);
}
let token = first.next_token.clone();          // persist this (file, database, ...)

// Later: only changes since the token
let changes = tasks.items().delta(token.as_deref()).await?;
let token = changes.next_token.clone();        // replace the stored token
```

The same pattern works on drives:

```rust
let sync = drive.delta(None).await?;
let later = drive.delta(sync.next_token.as_deref()).await?;
```

Guidelines:

- Always **store the newest token**; an old token replays old changes.
- Deleted items arrive flagged as deleted in the payload: handle them to keep your copy in sync.
- If a token expires the service answers with an error; start over with `delta(None)`.

## Webhook subscriptions

Subscriptions make Graph call **your URL** when a list changes, so you do not need to poll.

```rust
use oxipoint::graph::NewSubscription;

let sub = tasks
    .subscriptions()
    .create(&NewSubscription::new("https://example.com/hook", "2030-01-01T00:00:00Z"))
    .await?;
println!("{} expires {:?}", sub.id, sub.expiration_date_time);
```

Requirements and behavior:

- The notification URL must be **public HTTPS** and answer Graph's validation request within a few
  seconds.
- A notification only says *something changed*. Combine it with **delta** to find out what.
- Subscriptions expire. Renew before `expiration_date_time`.

```rust
let existing = tasks.subscriptions().list().await?;
```

A common architecture: webhook arrives, enqueue a job, the job calls `delta(token)` and processes
the changes.
