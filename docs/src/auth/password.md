# Username and password

Signs in as a user with no prompt (the OAuth *resource owner password* flow).

> **Legacy.** It fails for accounts that require MFA, use federated sign-in, or fall under
> conditional access, and Microsoft discourages it. Use it only for service accounts in test
> tenants and automated test suites.

**App registration:** *Allow public client flows* = Yes; the user must not require MFA.

```rust
use oxipoint::auth::PasswordCredential;

let cred = PasswordCredential::new(
    "contoso.onmicrosoft.com",
    "<client-id>",
    "user@contoso.com",
    std::env::var("APP_PASSWORD").unwrap(), // never hardcode passwords
);
```

## Notes

- This is the only credential that is `Clone`, which makes it convenient for one credential shared
  by a Graph client and a SharePoint client.
- The password is stored as a `Secret` and printed as `[REDACTED]`.
- Load the password from an environment variable, a secret manager or a vault, never from source.
