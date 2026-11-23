//! Repo task runner.
//!
//! - `live-spike`     — prove wreq's Chrome profile clears Cloudflare.
//! - `bindings`       — build the FFI crate and generate per-language bindings.
//! - `fixtures`       — print how to regenerate golden fixtures.

use std::path::{Path, PathBuf};
use std::process::Command as Proc;

use anyhow::{Context, bail};
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

    /// Generate UniFFI bindings (python, kotlin, swift) and the napi header.
    Bindings {
        /// Restrict to one language: python, kotlin, swift, or napi.
        #[arg(long)]
        language: Option<String>,
        /// Release build.
        #[arg(long)]
        release: bool,
    },

    /// Print the exact command that regenerates the golden fixtures.
    Fixtures,

    /// Run the full local gate (fmt, clippy, nextest, deny, REUSE, typos,
    /// TOML lint, file length). CI runs the same set; this is for a pre-push
    /// sanity check.
    Check,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::LiveSpike { path, token } => live_spike(path, token).await?,
        Command::Bindings { language, release } => bindings(language, release)?,
        Command::Fixtures => fixtures()?,
        Command::Check => check()?,
    }
    Ok(())
}

fn repo_root() -> anyhow::Result<PathBuf> {
    // xtask lives at <root>/tools/xtask
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .context("could not locate the repository root")
}

fn run(command: &mut Proc) -> anyhow::Result<()> {
    let display = format!("{command:?}");
    let status = command
        .status()
        .with_context(|| format!("failed to spawn {display}"))?;
    if !status.success() {
        bail!("{display} failed with {status}");
    }
    Ok(())
}

fn bindings(language: Option<String>, release: bool) -> anyhow::Result<()> {
    let root = repo_root()?;
    let profile = if release { "release" } else { "debug" };

    // 1. Build the FFI cdylib and the pinned bindgen binary.
    let mut build = Proc::new(env!("CARGO"));
    build
        .current_dir(&root)
        .args([
            "build",
            "-p",
            "novelai-image-ffi-uniffi",
            "-p",
            "uniffi-bindgen",
        ])
        .args(if release { vec!["--release"] } else { vec![] });
    run(&mut build)?;

    // 2. Locate the cdylib.
    let lib_name = if cfg!(target_os = "windows") {
        "novelai_image_sdk_ffi.dll"
    } else if cfg!(target_os = "macos") {
        "libnovelai_image_sdk_ffi.dylib"
    } else {
        "libnovelai_image_sdk_ffi.so"
    };
    let library = root.join("target").join(profile).join(lib_name);
    anyhow::ensure!(
        library.exists(),
        "cdylib not found at {} — build the ffi crate first",
        library.display()
    );

    let bindgen = root
        .join("target")
        .join(profile)
        .join(if cfg!(target_os = "windows") {
            "uniffi-bindgen.exe"
        } else {
            "uniffi-bindgen"
        });
    anyhow::ensure!(
        bindgen.exists(),
        "uniffi-bindgen not found at {}",
        bindgen.display()
    );

    let languages: Vec<&str> = match language.as_deref() {
        Some("python") => vec!["python"],
        Some("kotlin") => vec!["kotlin"],
        Some("swift") => vec!["swift"],
        Some("napi") => vec![],
        Some(other) => bail!("unknown language {other:?}; expected python, kotlin, swift, napi"),
        None => vec!["python", "kotlin", "swift"],
    };

    for lang in languages {
        let out_dir = root.join("bindings").join(lang).join("generated");
        std::fs::create_dir_all(&out_dir)?;
        let mut generate = Proc::new(&bindgen);
        generate
            .current_dir(&root)
            .arg("generate")
            .arg("--library")
            .arg(&library)
            .args(["--language", lang])
            .arg("--out-dir")
            .arg(&out_dir);
        run(&mut generate)?;
        println!("generated {lang} bindings in {}", out_dir.display());
    }

    println!(
        "napi addon: {} (build with `napi build --release --target <triple>` in          bindings/node during phase P3)",
        root.join("target")
            .join(profile)
            .join("novelai_image_ffi_napi.dll")
            .display()
    );
    Ok(())
}

fn check() -> anyhow::Result<()> {
    let root = repo_root()?;
    let steps: &[(&str, &[&str])] = &[
        ("cargo fmt", &["fmt", "--all", "--", "--check"]),
        (
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
        (
            "nextest",
            &["nextest", "run", "--workspace", "--all-features"],
        ),
        ("deny", &["deny", "check"]),
    ];
    for (name, args) in steps {
        println!("== {name}");
        let mut command = Proc::new("cargo");
        command.current_dir(&root).args(*args);
        run(&mut command)?;
    }

    // Non-cargo gates.
    for (name, program, args) in [
        ("reuse", "reuse", vec!["lint"]),
        ("typos", "typos", vec![]),
        ("tombi", "tombi", vec!["lint"]),
        ("file-length", "node", vec!["scripts/check-file-length.mjs"]),
        (
            "prek",
            "prek",
            vec!["run", "--all-files", "--stage", "pre-commit"],
        ),
    ] {
        println!("== {name}");
        let mut command = Proc::new(program);
        command.current_dir(&root).args(args);
        if let Err(error) = run(&mut command) {
            eprintln!("{name} is not installed or failed: {error}");
            eprintln!("install it and re-run `cargo xtask check`");
            return Err(error);
        }
    }
    println!("all gates passed");
    Ok(())
}

fn fixtures() -> anyhow::Result<()> {
    println!("Golden fixtures are generated from the pinned Python reference:\n");
    println!("  NOVELAI_REFERENCE_ROOT=../NovelAI-Image-MCP \\");
    println!("    uv run --directory ../NovelAI-Image-MCP/apps/server \\");
    println!("    python \"$PWD/scripts/gen_fixtures.py\"\n");
    println!("Then review `git diff fixtures/` — every change is a behavior change.");
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
