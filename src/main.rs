//! Demo CLI: signs in with a device code, creates a custom list named TEST with three columns
//! shown in the default view, and adds 10 random items.
//!
//! ```text
//! rsp <site-url> --client-id <id> [--tenant <tenant>]
//! ```
//! Values can also come from the environment or a `.env` file in the working directory
//! (`RSP_SITE_URL`, `RSP_CLIENT_ID`, `RSP_TENANT`). See `.env.example`.

use std::{collections::HashMap, env, fs, process::ExitCode};

use reqwest::Method;
use rsp::{
    Result,
    auth::DeviceCodeCredential,
    sharepoint::{ListTemplate, NewList, SharePointClient},
};
use serde_json::json;

const LIST: &str = "TEST";

struct Args {
    site_url: String,
    client_id: String,
    tenant: String,
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
    let mut it = env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--client-id" => client_id = Some(it.next().ok_or("--client-id needs a value")?),
            "--tenant" => tenant = Some(it.next().ok_or("--tenant needs a value")?),
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
    })
}

async fn run(args: Args) -> Result<()> {
    let credential = DeviceCodeCredential::new(&args.tenant, &args.client_id, |info| {
        println!("{}", info.message);
    });
    let sp = SharePointClient::new(&args.site_url, credential)?;

    let web = sp.web().select(["Title", "Url", "Id"]).get().await?;
    println!("Site:  {}", web.title.as_deref().unwrap_or("(untitled)"));

    let list = sp
        .lists()
        .create(&NewList::new(LIST, ListTemplate::GenericList).description("Demo list"))
        .await?;
    println!("Created list {}", list.title.unwrap_or_default());

    let list = sp.list_by_title(LIST);
    let columns = [("Notes", 2), ("Quantity", 9), ("Active", 8)];
    for (name, kind) in columns {
        list.fields()
            .create(&json!({ "Title": name, "FieldTypeKind": kind }))
            .await?;
        sp.request(
            Method::POST,
            format!(
                "/web/lists/getbytitle('{LIST}')/defaultview/viewfields/addviewfield('{name}')"
            ),
        )
        .send()
        .await?;
        println!("Added column {name} to the default view");
    }

    for i in 1..=10 {
        let item = list
            .items()
            .create(&json!({
                "Title": format!("Item {i}"),
                "Notes": format!("note-{:08x}", rand::random::<u32>()),
                "Quantity": rand::random::<u32>() % 1000,
                "Active": rand::random::<bool>(),
            }))
            .await?;
        println!("Created item {i} (id {})", item.id);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("error: {msg}\n");
            }
            eprintln!("usage: rsp <site-url> --client-id <id> [--tenant <tenant>]");
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
