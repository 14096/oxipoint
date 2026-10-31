//! Demo CLI: signs in interactively and queries a SharePoint site.
//!
//! ```text
//! rsp <site-url> --client-id <id> [--tenant <tenant>] [--device-code]
//! ```
//! Values can also come from the environment or a `.env` file in the working directory
//! (`RSP_SITE_URL`, `RSP_CLIENT_ID`, `RSP_TENANT`, `RSP_DEVICE_CODE`). See `.env.example`.

use std::{collections::HashMap, env, fs, process::ExitCode};

use rsp::{
    Result,
    auth::{BrowserCredential, DeviceCodeCredential, TokenCredential},
    sharepoint::SharePointClient,
};

struct Args {
    site_url: String,
    client_id: String,
    tenant: String,
    device_code: bool,
}

/// Reads `KEY=VALUE` lines from `./.env` (comments and `export` prefixes allowed).
/// A missing file is not an error. Real environment variables take precedence.
fn load_dotenv() -> HashMap<String, String> {
    let Ok(text) = fs::read_to_string(".env") else {
        return HashMap::new();
    };
    text.lines()
        .filter_map(|line| {
            let line = line.trim().trim_start_matches("export ").trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            let v = v.trim();
            let v = v
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| v.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(v);
            Some((k.trim().to_owned(), v.to_owned()))
        })
        .collect()
}

fn parse_args() -> std::result::Result<Args, String> {
    let dotenv = load_dotenv();
    let get = |key: &str| {
        env::var(key)
            .ok()
            .or_else(|| dotenv.get(key).cloned())
            .filter(|v| !v.is_empty())
    };
    let mut site_url = get("RSP_SITE_URL");
    let mut client_id = get("RSP_CLIENT_ID");
    let mut tenant = get("RSP_TENANT");
    let mut device_code = get("RSP_DEVICE_CODE")
        .is_some_and(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"));
    let mut it = env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--client-id" => client_id = Some(it.next().ok_or("--client-id needs a value")?),
            "--tenant" => tenant = Some(it.next().ok_or("--tenant needs a value")?),
            "--device-code" => device_code = true,
            "-h" | "--help" => return Err(String::new()),
            _ if a.starts_with('-') => return Err(format!("unknown option {a}")),
            _ => site_url = Some(a),
        }
    }
    Ok(Args {
        site_url: site_url.ok_or("missing <site-url> (or RSP_SITE_URL in the environment/.env)")?,
        client_id: client_id
            .ok_or("missing --client-id (or RSP_CLIENT_ID in the environment/.env)")?,
        tenant: tenant.unwrap_or_else(|| "organizations".into()),
        device_code,
    })
}

async fn run(args: Args) -> Result<()> {
    let credential: Box<dyn TokenCredential> = if args.device_code {
        Box::new(DeviceCodeCredential::new(
            &args.tenant,
            &args.client_id,
            |info| {
                println!("{}", info.message);
            },
        ))
    } else {
        println!("Opening the browser to sign in...");
        Box::new(BrowserCredential::new(&args.tenant, &args.client_id))
    };

    let sp = SharePointClient::new(&args.site_url, BoxedCredential(credential))?;

    let web = sp.web().select(["Title", "Url", "Id"]).get().await?;
    println!("Site:  {}", web.title.as_deref().unwrap_or("(untitled)"));
    println!("URL:   {}", web.url.as_deref().unwrap_or(&args.site_url));

    let lists = sp
        .lists()
        .filter("Hidden eq false")
        .select(["Id", "Title", "ItemCount"])
        .top(10)
        .list()
        .await?;
    println!("Lists ({}):", lists.len());
    for l in lists {
        println!(
            "  - {} ({} items)",
            l.title.unwrap_or_default(),
            l.item_count.unwrap_or(0)
        );
    }
    Ok(())
}

/// Lets a `Box<dyn TokenCredential>` be passed where `impl TokenCredential` is expected.
#[derive(Debug)]
struct BoxedCredential(Box<dyn TokenCredential>);

#[async_trait::async_trait]
impl TokenCredential for BoxedCredential {
    async fn get_token(&self, scope: &str) -> Result<rsp::auth::AccessToken> {
        self.0.get_token(scope).await
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("error: {msg}\n");
            }
            eprintln!("usage: rsp <site-url> --client-id <id> [--tenant <tenant>] [--device-code]");
            return if msg.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            };
        }
    };
    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
