//! Repo task runner.
//!
//! - `live-spike`     — prove wreq's Chrome profile clears Cloudflare.
//! - `bindings`       — build the FFI crate and generate per-language bindings.
//! - `fixtures`       — print how to regenerate golden fixtures.

use std::path::{Path, PathBuf};
use std::process::Command as Proc;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use regex::Regex;

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

    /// Write the workspace version into every manifest that carries one.
    ///
    /// `Cargo.toml`'s `workspace.package.version` is the single source of
    /// truth; the Python, npm, Kotlin and Swift manifests are generated from
    /// it. `--check` verifies instead of writing, which is what the release
    /// workflow runs on the tagged commit.
    SyncVersions {
        /// Fail when a manifest disagrees, without writing anything.
        #[arg(long)]
        check: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::LiveSpike { path, token } => live_spike(path, token).await?,
        Command::Bindings { language, release } => bindings(language, release)?,
        Command::Fixtures => fixtures()?,
        Command::Check => check()?,
        Command::SyncVersions { check } => sync_versions(check)?,
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
        if lang == "kotlin" {
            package_kotlin_library(&root, profile)?;
        }
        if lang == "swift" {
            // SwiftPM's `systemLibrary` target wants a directory containing the
            // header and a file literally named `module.modulemap`; UniFFI
            // writes `<Module>.modulemap` beside the Swift file.
            let module_dir = out_dir.join("NovelAIImageSDKFFI");
            std::fs::create_dir_all(&module_dir)?;
            std::fs::copy(
                out_dir.join("NovelAIImageSDKFFI.h"),
                module_dir.join("NovelAIImageSDKFFI.h"),
            )?;
            std::fs::copy(
                out_dir.join("NovelAIImageSDKFFI.modulemap"),
                module_dir.join("module.modulemap"),
            )?;
        }
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

/// The workspace version, read from the root manifest.
fn workspace_version(root: &Path) -> anyhow::Result<String> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))?;
    let in_workspace_package = manifest
        .split("[workspace.package]")
        .nth(1)
        .context("no [workspace.package] section in Cargo.toml")?;
    for line in in_workspace_package.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("version = ") {
            return Ok(rest.trim().trim_matches('"').to_owned());
        }
        if line.starts_with('[') {
            break;
        }
    }
    anyhow::bail!("no version in [workspace.package]")
}

/// Replace a version string in a manifest, reporting whether it changed.
/// Replace the version captured by `pattern`'s `{version}` slot.
///
/// `pattern` must contain exactly one `{version}`; it is compiled as a regex
/// where that slot matches any version-shaped string. A plain `str::replace`
/// cannot work here: the pattern is a template, not the literal text, so
/// nothing would ever match.
fn patch_version(path: &Path, pattern: &str, version: &str) -> anyhow::Result<Option<String>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;

    let mut regex = String::new();
    for (index, part) in pattern.split("{version}").enumerate() {
        if index > 0 {
            // Version-shaped: digits, dots, optionally a pre-release suffix.
            regex.push_str(r"([0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?)");
        }
        regex.push_str(&regex_escape(part));
    }

    let re = Regex::new(&regex).with_context(|| format!("invalid pattern {pattern}"))?;
    let Some(captures) = re.captures(&text) else {
        return Ok(None);
    };
    let Some(current) = captures.get(1) else {
        return Ok(None);
    };
    if current.as_str() == version {
        return Ok(None);
    }
    // Replace only the captured version; the surrounding text (a JSON key, a
    // TOML assignment) must survive untouched.
    let updated = format!(
        "{}{}{}",
        &text[..current.start()],
        version,
        &text[current.end()..]
    );
    Ok(Some(updated))
}

/// Escape the literal parts of a regex template.
fn regex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if r"\\.^$|?*+()[]{}".contains(character) {
            out.push('\\');
        }
        out.push(character);
    }
    out
}

fn sync_versions(check_only: bool) -> anyhow::Result<()> {
    let root = repo_root()?;
    let version = workspace_version(&root)?;
    println!("workspace version: {version}");

    // (path, the exact line that carries the version, human name)
    let targets: &[(&str, &str, &str)] = &[
        (
            "bindings/python/pyproject.toml",
            "version = \"{version}\"",
            "python",
        ),
        (
            "bindings/node/package.json",
            "\"version\": \"{version}\"",
            "npm",
        ),
        (
            "bindings/node/package.json",
            "\"@xinvxueyuan/novelai-image-sdk-{platform}\": \"{version}\"",
            "npm-platform-deps",
        ),
        (
            "bindings/kotlin/build.gradle.kts",
            "version = \"{version}\"",
            "kotlin",
        ),
        (
            "bindings/swift/Package.swift",
            "releases/download/v{version}/",
            "swift",
        ),
    ];

    let mut stale = Vec::new();
    for (relative, pattern, name) in targets {
        let path = root.join(relative);
        if !path.exists() {
            println!("  {name}: {} does not exist yet, skipped", path.display());
            continue;
        }
        let Some(updated) = patch_version(&path, pattern, &version)? else {
            println!("  {name}: up to date");
            continue;
        };
        stale.push(*name);
        if check_only {
            println!("  {name}: stale");
            continue;
        }
        std::fs::write(&path, updated)?;
        println!("  {name}: updated");
    }

    if check_only && !stale.is_empty() {
        anyhow::bail!(
            "version mismatch in: {} — run `cargo xtask sync-versions`",
            stale.join(", ")
        );
    }
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

/// The resource directory name for the host running the build.
fn host_platform_tag() -> &'static str {
    if cfg!(windows) {
        "win-x64"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "mac-arm64"
        } else {
            "mac-x64"
        }
    } else if cfg!(target_arch = "aarch64") {
        "linux-arm64"
    } else {
        "linux-x64"
    }
}

/// Copy the host's cdylib into the Kotlin package's resources.
///
/// A JAR cannot load a shared library from inside itself, so NativeLoader
/// extracts `resources/<os>-<arch>/` at runtime. The release workflow collects
/// the other platforms' libraries separately.
fn package_kotlin_library(root: &Path, profile: &str) -> anyhow::Result<()> {
    let target_dir = root.join("target").join(profile);
    let candidates = [
        "novelai_image_sdk_ffi.dll",
        "libnovelai_image_sdk_ffi.dylib",
        "libnovelai_image_sdk_ffi.so",
    ];
    let Some(source) = candidates
        .iter()
        .map(|name| target_dir.join(name))
        .find(|path| path.exists())
    else {
        println!(
            "note: no native library built for this host;              the JAR will carry no native resources"
        );
        return Ok(());
    };

    let destination = root
        .join("bindings")
        .join("kotlin")
        .join("src/main/resources")
        .join(host_platform_tag());
    std::fs::create_dir_all(&destination)?;
    let target = destination.join(source.file_name().context("library has a name")?);
    std::fs::copy(&source, &target)?;
    println!("packaged {} into {}", source.display(), target.display());
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
