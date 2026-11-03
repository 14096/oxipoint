# Overview and choosing a flow

Every sign-in method implements `auth::TokenCredential`, so any of them works with both
`GraphClient` and `SharePointClient`.

## Pick by who the program acts as

| Flow                       | Acts as            | Type                    | Feature            |
| -------------------------- | ------------------ | ----------------------- | ------------------ |
| Interactive browser (PKCE) | the signed-in user | `BrowserCredential`     | `auth-browser`     |
| Device code                | the signed-in user | `DeviceCodeCredential`  | `auth-device-code` |
| Username/password          | a user, no prompt  | `PasswordCredential`    | `auth-password`    |
| Certificate                | the application    | `CertificateCredential` | `auth-certificate` |

## Decision guide

```text
Is a person present to sign in?
├── yes ── Is there a browser on this machine?
│          ├── yes ── BrowserCredential
│          └── no  ── DeviceCodeCredential   (SSH sessions, containers, servers)
└── no  ── Unattended job or service?
           ├── yes ── CertificateCredential  (the only app-only method SharePoint REST accepts)
           └── test tenant service account only ── PasswordCredential (legacy)
```

- **Tools used by people**: browser on a desktop, device code anywhere else.
- **Unattended jobs and services**: certificate.
- **Username/password** fails for accounts with MFA, federated sign-in or conditional access, and
  Microsoft discourages it. Use it only for service accounts in test tenants.

## Delegated vs application permissions

- **Delegated** (browser, device code, password): the app can do what the *signed-in user* can do,
  limited by the permissions consented on the app.
- **Application** (certificate): the app acts as itself. It needs *application* permissions, which
  are always admin-consented, and it sees everything those permissions cover.

## Parameters

- **tenant**: the tenant ID (GUID) **or** a verified domain such as `contoso.onmicrosoft.com`.
  Delegated flows also accept `organizations` (work or school accounts from any tenant) and
  `common`. Certificate auth needs a real tenant ID or domain.
- **client_id**: the Application (client) ID of your app registration.

## Using a credential

```rust
let graph = GraphClient::new(cred);                                // Graph
let sp = SharePointClient::new("https://contoso.sharepoint.com/sites/team", cred)?; // SharePoint
```

The client asks the credential for tokens as needed: `https://graph.microsoft.com/.default` for
Graph and `<site origin>/.default` for SharePoint.

## National clouds

Every credential accepts `.with_authority(url)` to target a sovereign cloud, for example
`https://login.microsoftonline.us`. Also point the Graph client at the matching endpoint with
`GraphClient::builder(cred).base_url(..)`.
