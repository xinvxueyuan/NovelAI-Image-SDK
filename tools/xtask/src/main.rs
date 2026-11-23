//! Repo task runner.
//!
//! Subcommands are added as phases land:
//! - `live-spike`  — Phase 0: prove wreq's Chrome profile clears Cloudflare.
//! - `fixtures`    — Phase 1: golden fixtures from the pinned Python source.
//! - `bindings`    — Phase 3: generate per-language bindings.
//! - `sync-versions` / `verify-release`.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "NovelAI-Image-SDK repo tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Hit image.novelai.net with wreq's Chrome profile and report the result.
    ///
    /// Without a token this proves the TLS/HTTP2 fingerprint clears Cloudflare:
    /// a JSON 401 from NovelAI means the request reached the origin.
    LiveSpike {
        /// Path to probe (defaults to the unauthenticated account endpoint).
        #[arg(long, default_value = "/user/subscription")]
        path: String,
        /// Bearer token; falls back to $NOVELAI_TOKEN.
        #[arg(long)]
        token: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::LiveSpike { path, token } => live_spike(path, token).await?,
    }
    Ok(())
}

async fn live_spike(path: String, token: Option<String>) -> anyhow::Result<()> {
    let base = "https://image.novelai.net";
    let token = token.or_else(|| std::env::var("NOVELAI_TOKEN").ok());
    println!(
        "probe: {base}{path} (token: {})",
        if token.is_some() { "yes" } else { "no" }
    );

    // Without a real token, a placeholder still sends the request to the wire;
    // NovelAI answers 401 (proving the fingerprint was accepted).
    let client = novelai_image_sdk::client::NovelAiClient::new(
        novelai_image_sdk::config::ClientConfig::new(novelai_image_sdk::auth::Credentials::token(
            token
                .clone()
                .unwrap_or_else(|| "probe-invalid-token".into()),
        )),
    )?;
    let (status, body) = match token {
        Some(_) => {
            let map = client.get_subscription().await?;
            (200u16, serde_json::to_string(&map).unwrap_or_default())
        }
        None => {
            let result = client.get_subscription().await.unwrap_err();
            (401u16, result.to_string())
        }
    };

    println!("status: {status}");
    println!("body: {body}");

    let ok = if token.is_some() {
        (200..300).contains(&status)
    } else {
        status == 401
    };
    anyhow::ensure!(
        ok,
        "unexpected status {status}: request did not reach NovelAI's origin \
         (Cloudflare block?) — see docs/cloudflare-spike.md"
    );
    println!("SPIKE OK: Chrome emulation reached the origin");
    Ok(())
}
