# Register an Entra app

Every call is made on behalf of an **app registration** in Microsoft Entra ID (formerly Azure AD).
You need two values from it:

- **Application (client) ID**: passed to every credential as `client_id`.
- **Directory (tenant) ID**: passed as `tenant`. A verified domain such as
  `contoso.onmicrosoft.com` works too. The short name `contoso` does **not**.

## Step by step

1. Open the Entra admin center, then **App registrations**, then **New registration**.
2. Pick a name. For *Supported account types*, "single tenant" is the safest default.
3. Copy the **Application (client) ID** and **Directory (tenant) ID** from the overview page.
4. Under **API permissions**, add what your program needs (see below) and **grant admin consent**
   where the permission requires it.
5. Under **Authentication**, configure the platform for the sign-in method you chose.

## Choosing permissions

oxipoint requests the `/.default` scope, which means *"give me every permission already consented
on the app registration"*. Consequently a `403` almost always means a permission or its consent is
missing, not that the code is wrong.

| API            | Delegated (a signed-in user)                         | Application (no user, certificate flow)       |
| -------------- | ---------------------------------------------------- | --------------------------------------------- |
| Microsoft Graph | `User.Read`, `Sites.ReadWrite.All`, `Files.ReadWrite.All` | `Sites.ReadWrite.All`, `Files.ReadWrite.All`, `Sites.Selected` |
| SharePoint      | `AllSites.Read`, `AllSites.FullControl`              | `Sites.Selected`, `Sites.ReadWrite.All`       |

Prefer **`Sites.Selected`** for production services: it limits the app to the specific sites you
grant, instead of the whole tenant.

## Authentication settings per flow

| Flow              | What to configure under **Authentication**                                                      |
| ----------------- | ----------------------------------------------------------------------------------------------- |
| Browser           | Add platform **Mobile and desktop** with redirect URI `http://localhost`. Enable public client. |
| Device code       | **Allow public client flows** = Yes.                                                            |
| Username/password | **Allow public client flows** = Yes; the user must not require MFA.                             |
| Certificate       | Upload the public certificate under **Certificates & secrets**. Use application permissions.    |

> Client secrets are not supported for SharePoint REST app-only access: SharePoint rejects them.
> Use a certificate.

## Keep the values out of source code

The demo CLI reads `OXIPOINT_TENANT` and `OXIPOINT_CLIENT_ID` from the environment or a `.env`
file. Do the same in your programs, and never commit a `.env` (this repo's `.gitignore` already
excludes it).
