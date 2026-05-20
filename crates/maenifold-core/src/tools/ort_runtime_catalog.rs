#![cfg_attr(not(feature = "model-download"), allow(dead_code))]

use serde::{Deserialize, Serialize};
use std::path::Path;

const BUNDLED_ORT_RUNTIME_CATALOG_JSON: &str =
    include_str!("../../../../assets/catalogs/ort-runtime.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OrtRuntimeCatalogEntry {
    pub id: String,
    pub version: String,
    pub target_os: String,
    pub target_arch: String,
    pub dylib_filename: String,
    pub download_url: String,
    pub sha256: String,
}

pub(crate) fn load_ort_runtime_catalog(
    assets_path: &Path,
    trust_external: bool,
) -> Vec<OrtRuntimeCatalogEntry> {
    let external = assets_path.join("catalogs").join("ort-runtime.json");
    if external.is_file() {
        if !trust_external {
            tracing::warn!(
                "Ignoring external ORT runtime catalog override because explicit trust was not provided; using bundled catalog"
            );
        } else {
            tracing::warn!(
                "Using explicitly trusted external ORT runtime catalog override from configured assets directory"
            );
            match std::fs::read_to_string(&external) {
                Ok(text) => match load_ort_runtime_catalog_from_str(&text) {
                    Ok(entries) => return entries,
                    Err(error) => tracing::warn!(
                        "Failed to parse external ORT runtime catalog: {error}; falling back to bundled catalog"
                    ),
                },
                Err(error) => tracing::warn!(
                    "Failed to read external ORT runtime catalog: {error}; falling back to bundled catalog"
                ),
            }
        }
    }

    match load_ort_runtime_catalog_from_str(BUNDLED_ORT_RUNTIME_CATALOG_JSON) {
        Ok(entries) => entries,
        Err(error) => {
            tracing::error!("Failed to parse bundled ORT runtime catalog JSON: {error}");
            Vec::new()
        }
    }
}

pub(crate) fn load_ort_runtime_catalog_from_str(
    text: &str,
) -> Result<Vec<OrtRuntimeCatalogEntry>, String> {
    let entries: Vec<OrtRuntimeCatalogEntry> =
        serde_json::from_str(text).map_err(|error| error.to_string())?;
    for entry in &entries {
        validate_catalog_entry(entry)?;
    }
    Ok(entries)
}

pub(crate) fn pick_ort_runtime_entry<'a>(
    entries: &'a [OrtRuntimeCatalogEntry],
    target_os: Option<&str>,
    target_arch: Option<&str>,
) -> Result<&'a OrtRuntimeCatalogEntry, String> {
    let target_os = target_os.unwrap_or(std::env::consts::OS);
    let target_arch = target_arch.unwrap_or(std::env::consts::ARCH);
    entries
        .iter()
        .find(|entry| entry.target_os == target_os && entry.target_arch == target_arch)
        .ok_or_else(|| {
            format!("No ONNX Runtime download is cataloged for {target_os}/{target_arch}")
        })
}

pub(crate) fn ort_runtime_dylib_filename(target_os: &str) -> &'static str {
    match target_os {
        "windows" => "onnxruntime.dll",
        "linux" | "android" => "libonnxruntime.so",
        "macos" | "ios" => "libonnxruntime.dylib",
        _ => "libonnxruntime.so",
    }
}

fn validate_catalog_entry(entry: &OrtRuntimeCatalogEntry) -> Result<(), String> {
    if entry.id.trim().is_empty()
        || entry.version.trim().is_empty()
        || entry.target_os.trim().is_empty()
        || entry.target_arch.trim().is_empty()
        || entry.dylib_filename.trim().is_empty()
        || entry.download_url.trim().is_empty()
        || entry.sha256.trim().is_empty()
    {
        return Err("ORT runtime catalog entry is missing required metadata".to_string());
    }
    if entry.sha256.len() != 64 || !entry.sha256.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!(
            "ORT runtime catalog entry {} has invalid SHA-256 metadata",
            entry.id
        ));
    }
    let expected_dylib = ort_runtime_dylib_filename(&entry.target_os);
    if entry.dylib_filename != expected_dylib {
        return Err(format!(
            "ORT runtime catalog entry {} must use dylib filename {} for target {}",
            entry.id, expected_dylib, entry.target_os
        ));
    }
    Ok(())
}
