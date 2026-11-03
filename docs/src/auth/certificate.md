# Certificate

Authenticates as the **application** itself using a certificate and its RSA private key. No user is
involved.

**Best for:** unattended jobs, daemons, scheduled tasks and back-end services. This is also the
**only** app-only method SharePoint REST accepts (it rejects client secrets).

**App registration:** upload the **public** certificate under *Certificates & secrets* and grant
**application** permissions with admin consent.

## Create a certificate

```sh
openssl req -x509 -newkey rsa:2048 -nodes -days 730 \
  -keyout key.pem -out cert.pem -subj "/CN=oxipoint"
```

Upload `cert.pem` to the app registration. Keep `key.pem` secret.

## From PEM files

```rust
use oxipoint::auth::CertificateCredential;

// PEM certificate + PKCS#8/PKCS#1 private key
let cred = CertificateCredential::from_pem(
    "<tenant-id>", "<client-id>",
    &std::fs::read_to_string("cert.pem")?,
    &std::fs::read_to_string("key.pem")?,
)?;
```

## From a PFX archive

```rust
let cred = CertificateCredential::from_pfx(
    "<tenant-id>", "<client-id>", &std::fs::read("cert.pfx")?, "pfx-password",
)?;
```

## Notes

- Invalid certificate material is reported as `Error::Config` **before any network call**, so a
  bad file fails fast at startup.
- The tenant must be a real tenant ID or domain: `organizations` and `common` are not valid.
- Rotate certificates before expiry; add the new one to the app registration first, deploy, then
  remove the old one.
- For SharePoint, application permissions of `Sites.Selected` plus a per-site grant give
  least-privilege access.
