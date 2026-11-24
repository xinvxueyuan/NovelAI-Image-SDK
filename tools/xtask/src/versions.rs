//! Version synchronisation between the workspace and the binding manifests.

use std::path::Path;

use anyhow::Context;
use regex::Regex;

use crate::repo_root;

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
        regex.push_str(&template_body(part));
    }

    let re = Regex::new(&regex).with_context(|| format!("invalid pattern {pattern}"))?;
    // Replace every captured version: an npm manifest has one entry per
    // platform package, all carrying the same number. Only the captured
    // group changes, so surrounding JSON keys and TOML assignments survive.
    let mut updated = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut changed = false;
    for captures in re.captures_iter(&text) {
        let Some(current) = captures.get(1) else {
            continue;
        };
        updated.push_str(&text[cursor..current.start()]);
        if current.as_str() == version {
            updated.push_str(current.as_str());
        } else {
            updated.push_str(version);
            changed = true;
        }
        cursor = current.end();
    }
    if !changed {
        return Ok(None);
    }
    updated.push_str(&text[cursor..]);
    Ok(Some(updated))
}

/// Expand the non-version placeholders of a template into regex fragments.
///
/// `{platform}` matches any platform token and is never rewritten; only the
/// version slot is captured.
fn template_body(part: &str) -> String {
    let mut out = String::new();
    for (index, segment) in part.split("{platform}").enumerate() {
        if index > 0 {
            out.push_str("[A-Za-z0-9._-]+");
        }
        out.push_str(&regex_escape(segment));
    }
    out
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

pub fn sync_versions(check_only: bool) -> anyhow::Result<()> {
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
            // One entry per platform package; `patch_version` rewrites every
            // match, so one target pattern covers them all.
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
