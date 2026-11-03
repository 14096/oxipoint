# Device code

Prints a short code that the user enters at `https://microsoft.com/devicelogin` on **any** device,
such as a phone. The program meanwhile polls until sign-in completes.

**Best for:** SSH sessions, containers, CI shells and servers without a browser.

**App registration:** *Allow public client flows* = Yes.

```rust
use oxipoint::auth::DeviceCodeCredential;

let cred = DeviceCodeCredential::new("contoso.onmicrosoft.com", "<client-id>", |info| {
    println!("{}", info.message);
});
```

## The callback

The callback receives `DeviceCodeInfo` with `user_code`, `verification_uri`, `message` and more.
Show it however you like: print it, send it to a chat channel, or render it in a UI.

```rust
let cred = DeviceCodeCredential::new(tenant, client_id, |info| {
    eprintln!("Go to {} and enter {}", info.verification_uri, info.user_code);
});
```

## Notes

- The callback is invoked once per sign-in, not per request, because tokens are cached and
  refreshed.
- Like the browser flow, the refresh token is reused for other scopes, so a program using both
  Graph and SharePoint through one credential instance prompts once.
