#![cfg_attr(not(feature = "model-download"), allow(dead_code))]

use super::ort_runtime_catalog::OrtRuntimeCatalogEntry;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tar::Archive;

/// In-process mutex serializing concurrent `download_runtime` calls.
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

/// Inner: takes pre-fetched archive bytes, serializes via DOWNLOAD_LOCK.
pub(crate) fn download_runtime_inner(
    archive_bytes: &[u8],
    expected_sha256: &str,
    dylib_filename: &str,
    assets_path: &Path,
) -> Result<PathBuf, String> {
    let lib_dir = assets_path.join("lib");
    let final_dylib = lib_dir.join(dylib_filename);

    // Acquire in-process lock (single-process assumption).
    let _guard = download_lock()
        .lock()
        .map_err(|e| format!("download lock poisoned: {e}"))?;
    // Refuse-if-exists INSIDE critical section to prevent TOCTOU.
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
        let path_str = path.to_string_lossy().to_string();
        let Some(file_name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        // Fast-path basename filter (keeps only *.dylib matching ort naming).
        // NOTE: This is a pre-filter only. The full-path predicate below is
        // the authoritative gate that prevents dSYM collisions.
        if !is_runtime_dylib(&file_name) {
            continue;
        }
        // Authoritative full-path gate: reject any entry inside a .dSYM
        // bundle and require the pattern `*/lib/<filename>.dylib`.
        if !is_valid_dylib_archive_path(&path_str) {
            continue;
        }

        if entry.header().entry_type().is_file() {
            let out_path = target_dir.join(&file_name);
            let mut output = std::fs::File::create(&out_path).map_err(|error| error.to_string())?;
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

/// Atomic staging: verify sha256, extract to staging dir, validate Mach-O
/// magic + filetype, then rename into target_dir. Staging auto-cleans on fail.
pub(crate) fn stage_and_extract_checked(
    archive_bytes: &[u8],
    expected_sha256: &str,
    target_dir: &Path,
) -> Result<(), String> {
    // Verify SHA-256 BEFORE any disk write.
    let actual = sha256_bytes(archive_bytes);
    if actual != expected_sha256.to_ascii_lowercase() {
        return Err(format!(
            "SHA-256 verification failed: expected {expected_sha256}, got {actual}"
        ));
    }

    std::fs::create_dir_all(target_dir).map_err(|error| error.to_string())?;

    let staging = tempfile::tempdir_in(target_dir)
        .map_err(|e| format!("Failed to create staging directory: {e}"))?;

    extract_runtime_dylibs(archive_bytes, staging.path())?;
    // Post-extraction: validate magic + filetype BEFORE rename into final dir.
    validate_staged_dylib_magic(staging.path())?;

    // Atomic finalize: rename staged files into target_dir.
    for dir_entry in std::fs::read_dir(staging.path()).map_err(|e| e.to_string())? {
        let dir_entry = dir_entry.map_err(|e| e.to_string())?;
        let file_name = dir_entry.file_name();
        let final_path = target_dir.join(&file_name);
        std::fs::rename(dir_entry.path(), &final_path).map_err(|e| {
            format!(
                "Atomic rename failed for {}: {e}",
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
        .ok_or_else(|| "Archive did not contain a loadable libonnxruntime.dylib".to_string())?;

    std::fs::copy(source, canonical)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn is_runtime_dylib(file_name: &str) -> bool {
    file_name == "libonnxruntime.dylib"
        || (file_name.starts_with("libonnxruntime.") && file_name.ends_with(".dylib"))
}

/// Authoritative full-path gate. Rejects entries inside `.dSYM` bundles and
/// requires the archive path to match `<prefix>/lib/<basename>.dylib` — i.e.
/// the file must be a direct child of a `lib/` directory with no deeper nesting.
fn is_valid_dylib_archive_path(archive_path: &str) -> bool {
    // Reject any path containing a .dSYM segment (debug symbol bundles).
    // Case-insensitive: also catches `.dsym/` (rare but defensive).
    let lower = archive_path.to_ascii_lowercase();
    if lower.contains(".dsym/") || lower.contains(".dsym\\") {
        return false;
    }
    // Require pattern: <anything>/lib/<filename>.dylib
    let Some(lib_idx) = archive_path.rfind("/lib/") else {
        return false;
    };
    let after_lib = &archive_path[lib_idx + 5..]; // skip "/lib/"
    // Must be a direct child — no further path separators.
    !after_lib.contains('/')
}

/// Validates staged .dylib files: magic bytes + Mach-O filetype (MH_DYLIB/MH_BUNDLE).
/// Fat binaries pass on magic alone (documented: path filter is primary dSYM gate).
#[cfg(target_os = "macos")]
fn validate_staged_dylib_magic(staging_dir: &Path) -> Result<(), String> {
    use std::io::Read;
    for dir_entry in std::fs::read_dir(staging_dir).map_err(|e| e.to_string())? {
        let dir_entry = dir_entry.map_err(|e| e.to_string())?;
        if !dir_entry.path().is_file() {
            continue;
        }
        let mut buf = [0u8; 16];
        let mut file = std::fs::File::open(dir_entry.path()).map_err(|e| e.to_string())?;
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        let name = dir_entry.file_name();
        if n < 4 {
            return Err(format!(
                "Staged file {} too small ({n} bytes)",
                name.to_string_lossy()
            ));
        }
        if !is_loadable_library_magic(&[buf[0], buf[1], buf[2], buf[3]]) {
            return Err(format!(
                "expected Mach-O magic, got 0x{:02x}{:02x}{:02x}{:02x} in staged file {}",
                buf[0],
                buf[1],
                buf[2],
                buf[3],
                name.to_string_lossy()
            ));
        }
        validate_macho_filetype(&buf, n, &name)?;
    }
    Ok(())
}

/// Recognized Mach-O / fat-binary magic numbers (6 constants preserved).
#[cfg(target_os = "macos")]
fn is_loadable_library_magic(header: &[u8; 4]) -> bool {
    let magic = u32::from_ne_bytes(*header);
    matches!(
        magic,
        0xfeedfacf | 0xcffaedfe | 0xfeedface | 0xcefaedfe | 0xcafebabe | 0xbebafeca
    )
}

/// Layer 2 filetype check: byte 12 = filetype. MH_DYLIB(0x6)/MH_BUNDLE(0x8) pass.
/// Swapped magic → read filetype as BE. Fat binaries skip (accepted on magic).
#[cfg(target_os = "macos")]
fn validate_macho_filetype(
    buf: &[u8; 16],
    bytes_read: usize,
    filename: &std::ffi::OsStr,
) -> Result<(), String> {
    let magic = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if magic == 0xcafebabe || magic == 0xbebafeca {
        return Ok(());
    }
    if bytes_read < 16 {
        return Err(format!(
            "{} has Mach-O magic but only {bytes_read} bytes (need 16)",
            filename.to_string_lossy()
        ));
    }
    let ft = [buf[12], buf[13], buf[14], buf[15]];
    let filetype = if magic == 0xcffaedfe || magic == 0xcefaedfe {
        u32::from_be_bytes(ft)
    } else {
        u32::from_le_bytes(ft)
    };
    if filetype != 0x6 && filetype != 0x8 {
        return Err(format!(
            "Mach-O filetype 0x{filetype:x} in {} is not MH_DYLIB/MH_BUNDLE",
            filename.to_string_lossy()
        ));
    }
    Ok(())
}

/// Non-macOS: no dSYM filetype concern; stub accepts all staged files.
#[cfg(not(target_os = "macos"))]
fn validate_staged_dylib_magic(_staging_dir: &Path) -> Result<(), String> {
    Ok(())
}

fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
