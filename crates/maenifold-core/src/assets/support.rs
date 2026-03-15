use super::types::AssetKind;
use crate::error::{MaenifoldError, Result};
use crate::util::paths::{join_validated_path_buf, validate_relative_path};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Display;
use std::fs;
use std::io::ErrorKind;
use std::io::Read;
use std::path::{Path, PathBuf};

// T-10.3: RTM FR-11 / RTM-071 — MCP resource-change notifications only track JSON
// asset files under the 4 published asset catalogs.
pub fn is_watchable_asset_json_path(assets_root: &Path, path: &Path) -> bool {
    // T-14.5: RTM FR-12 - watcher notifications must ignore symlinked JSON files that resolve outside assets_root.
    join_validated_path_buf(assets_root, path.strip_prefix(assets_root).unwrap_or(path)).is_ok()
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        && watched_asset_kind_for_path(assets_root, path).is_some()
        && !is_hidden_asset_relative_path(assets_root, path)
}

/// Resolve the packaged assets directory.
///
/// 1. Check next to the running executable (release installs where assets ZIP
///    is extracted alongside the binary).
/// 2. Walk up from the executable looking for a Cargo workspace root (dev mode).
///
/// Returns `None` when no assets directory can be located.
// T-ASSET-PATH: RTM FR-3, FR-20 — runtime resolution replaces compile-time
// `CARGO_MANIFEST_DIR` so the binary works on any machine.
pub(crate) fn resolve_packaged_assets_root() -> Option<PathBuf> {
    // 1. exe-adjacent (release/installed binary)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join("assets");
            if asset_root_has_packaged_assets(&candidate) {
                return Some(candidate);
            }
        }
    }

    // 2. dev mode: walk up from exe looking for Cargo.toml with assets/ sibling
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        while let Some(d) = dir {
            let candidate = d.join("assets");
            if d.join("Cargo.toml").exists() && asset_root_has_packaged_assets(&candidate) {
                return Some(candidate);
            }
            dir = d.parent();
        }
    }

    None
}

fn asset_root_has_packaged_assets(root: &Path) -> bool {
    if !root.is_dir() {
        return false;
    }

    ["workflows", "roles", "colors", "perspectives"]
        .into_iter()
        .map(|dir| root.join(dir))
        .any(|dir| {
            dir.is_dir()
                && fs::read_dir(dir)
                    .map(|mut entries| entries.next().is_some())
                    .unwrap_or(false)
        })
}

pub(crate) fn sha256_digest(path: &Path) -> Result<[u8; 32]> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(hasher.finalize().into())
}

pub(crate) fn normalize_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn target_metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn asset_is_multi_agent(value: &Value) -> bool {
    value
        .get("multiAgent")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn watched_asset_kind_for_path(assets_root: &Path, path: &Path) -> Option<AssetKind> {
    let relative = path.strip_prefix(assets_root).ok()?;
    let first = relative.components().next()?.as_os_str().to_str()?;
    AssetKind::all()
        .into_iter()
        .find(|kind| kind.dir_name() == first)
}

pub(crate) fn is_hidden_asset_relative_path(assets_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(assets_root) else {
        return true;
    };

    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}

pub(crate) fn asset_files_in_root(root: &Path, kind: AssetKind) -> Result<Vec<PathBuf>> {
    let dir = root.join(kind.dir_name());
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            // T-14.5: RTM FR-12 - MCP asset reads/listing must skip symlinked files that canonically escape assets root.
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && is_watchable_asset_json_path(root, &path)
            {
                files.push(path);
            }
        }
    }
    // T-0.2: RTM NFR-3 - C# metadata ordering follows lexicographic file path order.
    files.sort();
    Ok(files)
}

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

pub(crate) fn validate_adopt_identifier(identifier: &str) -> Result<&str> {
    let trimmed = identifier.trim();
    if trimmed.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "Identifier must not be empty or whitespace-only.".to_string(),
        ));
    }

    // T-1.5: RTM FR-1 - reject traversal and non-slug identifiers before any asset path join.
    let invalid_identifier = || {
        MaenifoldError::InvalidArgument(format!(
            "Invalid identifier '{identifier}'. Identifiers may only contain ASCII letters, numbers, hyphens, and underscores."
        ))
    };

    validate_relative_path(trimmed).map_err(|_| invalid_identifier())?;

    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err(invalid_identifier());
    }

    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(invalid_identifier());
    }

    Ok(trimmed)
}

pub(crate) fn emit_asset_degraded_diagnostic(method: &str, path: &Path, error: &impl Display) {
    let display_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    emit_stderr_line(&format!(
        "[DEGRADED][AssetResources][{method}] Skipping malformed asset '{display_name}': {error}"
    ));
}

#[cfg(unix)]
fn emit_stderr_line(message: &str) {
    unsafe extern "C" {
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    }

    let mut line = String::with_capacity(message.len() + 1);
    line.push_str(message);
    line.push('\n');
    // SAFETY: fd=2 is the process stderr and the buffer is valid for the call duration.
    unsafe {
        let _ = write(2, line.as_ptr(), line.len());
    }
}

#[cfg(not(unix))]
fn emit_stderr_line(message: &str) {
    eprintln!("{message}");
}
