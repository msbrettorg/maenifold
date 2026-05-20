#![cfg_attr(not(feature = "model-download"), allow(dead_code))]

use super::ort_runtime_catalog::OrtRuntimeCatalogEntry;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tar::Archive;

/// In-process mutex serializing concurrent `download_runtime` calls.
/// Sufficient because the maenifold daemon is a single process per
/// repo-root (see AGENTS.md localhost trust model + maenifold.json).
static DOWNLOAD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn download_lock() -> &'static Mutex<()> {
    DOWNLOAD_LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(feature = "model-download")]
pub async fn download_runtime(
    entry: &OrtRuntimeCatalogEntry,
    assets_path: &Path,
) -> Result<PathBuf, String> {
    let response = reqwest::get(&entry.download_url)
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let archive_bytes = response.bytes().await.map_err(|error| error.to_string())?;
    download_runtime_inner(
        archive_bytes.as_ref(),
        &entry.sha256,
        &entry.dylib_filename,
        assets_path,
    )
}

/// Inner implementation that takes pre-fetched archive bytes.
/// Serializes concurrent callers via `DOWNLOAD_LOCK` and rechecks
/// the refuse-if-exists guard inside the critical section.
pub(crate) fn download_runtime_inner(
    archive_bytes: &[u8],
    expected_sha256: &str,
    dylib_filename: &str,
    assets_path: &Path,
) -> Result<PathBuf, String> {
    let lib_dir = assets_path.join("lib");
    let final_dylib = lib_dir.join(dylib_filename);

    // Acquire in-process lock — single-process assumption (see DOWNLOAD_LOCK doc).
    let _guard = download_lock()
        .lock()
        .map_err(|e| format!("download lock poisoned: {e}"))?;

    // Refuse-if-exists checked INSIDE the critical section to prevent TOCTOU.
    if final_dylib.is_file() {
        return Err(format!(
            "ORT runtime already installed at {}",
            final_dylib.display()
        ));
    }

    stage_and_extract_checked(archive_bytes, expected_sha256, &lib_dir)?;
    Ok(final_dylib)
}

#[cfg(not(feature = "model-download"))]
pub async fn download_runtime(
    _entry: &OrtRuntimeCatalogEntry,
    _assets_path: &Path,
) -> Result<PathBuf, String> {
    Err(super::embedding_download::MODEL_DOWNLOAD_FEATURE_ERROR.to_string())
}

pub(crate) fn extract_runtime_dylibs(
    archive_bytes: &[u8],
    target_dir: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(target_dir).map_err(|error| error.to_string())?;
    let cursor = std::io::Cursor::new(archive_bytes);
    let mut archive = Archive::new(GzDecoder::new(cursor));
    let mut canonical_link_target: Option<String> = None;
    let mut versioned_dylibs = Vec::new();

    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path().map_err(|error| error.to_string())?;
        let Some(file_name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        if !is_runtime_dylib(&file_name) {
            continue;
        }

        if entry.header().entry_type().is_file() {
            let out_path = target_dir.join(&file_name);
            let mut output =
                std::fs::File::create(&out_path).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
            if file_name != "libonnxruntime.dylib" {
                versioned_dylibs.push(file_name);
            }
            continue;
        }

        if entry.header().entry_type().is_symlink() || entry.header().entry_type().is_hard_link() {
            canonical_link_target = entry
                .link_name()
                .map_err(|error| error.to_string())?
                .and_then(|target| {
                    target
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_string)
                });
        }
    }

    ensure_canonical_dylib(
        target_dir,
        canonical_link_target.as_deref(),
        &versioned_dylibs,
    )
}

/// Atomic staging: verify sha256 in-memory, extract to a per-invocation
/// staging directory, then rename files into the final target_dir. The
/// final directory is never written to directly — existing files are safe
/// if extraction fails partway through.
pub(crate) fn stage_and_extract_checked(
    archive_bytes: &[u8],
    expected_sha256: &str,
    target_dir: &Path,
) -> Result<(), String> {
    // Verify SHA-256 BEFORE any disk write (contract: verify before disk).
    let actual = sha256_bytes(archive_bytes);
    if actual != expected_sha256.to_ascii_lowercase() {
        return Err(format!(
            "SHA-256 verification failed for ort-runtime.tar.gz: \
             expected {expected_sha256}, got {actual}"
        ));
    }

    std::fs::create_dir_all(target_dir).map_err(|error| error.to_string())?;

    let staging = tempfile::tempdir_in(target_dir).map_err(|error| {
        format!("Failed to create staging directory: {error}")
    })?;

    extract_runtime_dylibs(archive_bytes, staging.path())?;

    // Atomic finalize: rename each staged file into target_dir.
    let entries = std::fs::read_dir(staging.path())
        .map_err(|error| error.to_string())?;
    for dir_entry in entries {
        let dir_entry = dir_entry.map_err(|error| error.to_string())?;
        let file_name = dir_entry.file_name();
        let final_path = target_dir.join(&file_name);
        std::fs::rename(dir_entry.path(), &final_path).map_err(|error| {
            format!(
                "Atomic rename failed for {}: {error}",
                file_name.to_string_lossy()
            )
        })?;
    }

    Ok(())
}

fn ensure_canonical_dylib(
    target_dir: &Path,
    canonical_link_target: Option<&str>,
    versioned_dylibs: &[String],
) -> Result<(), String> {
    let canonical = target_dir.join("libonnxruntime.dylib");
    if canonical.is_file() {
        return Ok(());
    }

    let source = canonical_link_target
        .map(|name| target_dir.join(name))
        .filter(|path| path.is_file())
        .or_else(|| {
            versioned_dylibs
                .iter()
                .map(|name| target_dir.join(name))
                .find(|path| path.is_file())
        })
        .ok_or_else(|| {
            "Archive did not contain a loadable libonnxruntime.dylib".to_string()
        })?;

    std::fs::copy(source, canonical)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn is_runtime_dylib(file_name: &str) -> bool {
    file_name == "libonnxruntime.dylib"
        || (file_name.starts_with("libonnxruntime.") && file_name.ends_with(".dylib"))
}

fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
