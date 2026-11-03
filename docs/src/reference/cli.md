# Demo CLI

The repository also builds an `oxipoint` binary. It is a **demo**: it signs in with a device code,
creates a custom list named `TEST` with three columns shown in the default view, and adds ten random
items.

```text
oxipoint <site-url> --client-id <id> [--tenant <tenant>]
```

| Option / variable                         | Meaning                                              |
| ----------------------------------------- | ---------------------------------------------------- |
| `<site-url>` / `OXIPOINT_SITE_URL`        | The SharePoint site, e.g. `https://contoso.sharepoint.com/sites/team` |
| `--client-id` / `OXIPOINT_CLIENT_ID`      | Application (client) ID                              |
| `--tenant` / `OXIPOINT_TENANT`            | Tenant ID or domain (default `organizations`)        |

Values may come from the environment or from a `.env` file in the working directory
(`KEY=VALUE` lines; comments and `export` prefixes are allowed). Real environment variables win.
See `.env.example`.

```sh
cp .env.example .env     # fill in the values
cargo run
```
