//! Output persistence (feature `output`).
//!
//! Port of `novelai_image_mcp.nai.output`: writes PNG bytes with a timestamped,
//! non-colliding name, and resolves previously written names to paths without
//! allowing path traversal.

#![cfg(feature = "output")]

use std::path::{Path, PathBuf};

use rand::Rng;
use time::OffsetDateTime;

use crate::error::Error;

/// The default output directory, matching the MCP server.
pub const DEFAULT_OUTPUT_DIR: &str = "outputs";

/// Write image bytes to `<output_dir>/<name>-<UTC timestamp>-<6 hex>.png`.
///
/// Returns the written path. The directory (and parents) are created if needed.
pub fn save_image(data: &[u8], name: &str, output_dir: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir)
        .map_err(|e| Error::config(format!("failed to create output directory: {e}")))?;

    let now = OffsetDateTime::now_utc();
    let timestamp = format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    );
    let suffix: String = {
        use std::fmt::Write as _;
        let mut rng = rand::rng();
        (0..6).fold(String::with_capacity(6), |mut out, _| {
            let _ = write!(out, "{:x}", rng.random_range(0..16));
            out
        })
    };

    // Keep the base name filesystem-safe and non-empty.
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = if stem.is_empty() {
        "image".to_owned()
    } else {
        stem
    };

    let path = output_dir.join(format!("{stem}-{timestamp}-{suffix}.png"));
    std::fs::write(&path, data)
        .map_err(|e| Error::config(format!("failed to write {}: {e}", path.display())))?;
    Ok(path)
}

/// Resolve a previously returned filename inside `output_dir`.
///
/// Rejects absolute paths, path separators, `..`, and anything that is not a
/// plain `*.png` name, so a caller-supplied name can never escape the output
/// directory.
pub fn resolve_output_path(output_dir: impl AsRef<Path>, name: &str) -> Result<PathBuf, Error> {
    // Accept `.PNG`/`.Png` too: the content is PNG bytes either way.
    let extension_is_png = Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"));
    let valid = !name.is_empty()
        && extension_is_png
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !valid {
        return Err(Error::validation(format!(
            "invalid output name: {name:?} (plain *.png names only)"
        )));
    }
    Ok(output_dir.as_ref().join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "novelai-sdk-output-{tag}-{}-{}",
            std::process::id(),
            rand::rng().random::<u32>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_distinct_timestamped_files() {
        let dir = temp_dir("save");
        let mut paths = std::collections::HashSet::new();
        for _ in 0..20 {
            let path = save_image(b"png-bytes", "image", &dir).unwrap();
            assert_eq!(std::fs::read(&path).unwrap(), b"png-bytes");
            assert!(
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("image-")
            );
            paths.insert(path);
        }
        assert_eq!(paths.len(), 20, "names must not collide");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn creates_nested_directories() {
        let dir = temp_dir("nested").join("a").join("b");
        let path = save_image(b"x", "img", &dir).unwrap();
        assert!(path.exists());
        std::fs::remove_dir_all(dir.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn rejects_traversal_and_odd_names() {
        let dir = temp_dir("safe");
        for name in [
            "../etc/passwd",
            "/abs/path.png",
            "sub/dir.png",
            "noext",
            ".hidden.png",
            "",
        ] {
            assert!(
                resolve_output_path(&dir, name).is_err(),
                "accepted {name:?}"
            );
        }
        assert_eq!(
            resolve_output_path(&dir, "image-20260101T000000Z-abcdef.png").unwrap(),
            dir.join("image-20260101T000000Z-abcdef.png")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
