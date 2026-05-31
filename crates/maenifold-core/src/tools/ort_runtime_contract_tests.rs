use super::ToolContext;
use super::ort_runtime_catalog::{
    OrtRuntimeCatalogEntry, load_ort_runtime_catalog_from_str, pick_ort_runtime_entry,
};
#[cfg(target_os = "macos")]
use super::ort_runtime_catalog::ort_runtime_dylib_filename;
use crate::config::Config;
use crate::mcp::schema::EmbeddingModelsParams;
use crate::tools::embedding_models::embedding_models;
use crate::util::json::OutputFormat;
use serde_json::json;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::path::PathBuf;

struct ResetConfigOverrides;

impl Drop for ResetConfigOverrides {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

#[test]
fn ort_runtime_catalog_contract_accepts_valid_entry() {
    let entries = load_ort_runtime_catalog_from_str(
        &serde_json::to_string(&json!([{
        "id": "onnxruntime",
        "version": "1.24.0",
        "targetOs": "macos",
        "targetArch": "aarch64",
        "dylibFilename": "libonnxruntime.dylib",
        "downloadUrl": "https://github.com/microsoft/onnxruntime/releases/download/v1.24.0/onnxruntime-osx-arm64-1.24.0.tgz",
        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }]))
        .expect("serialize fixture"),
    )
    .expect("valid catalog");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "onnxruntime");
    assert_eq!(entries[0].target_os, "macos");
    assert_eq!(entries[0].target_arch, "aarch64");
    assert_eq!(entries[0].sha256.len(), 64);
}

#[test]
fn ort_runtime_catalog_contract_rejects_missing_sha256() {
    let error = load_ort_runtime_catalog_from_str(
        &serde_json::to_string(&json!([{
            "id": "onnxruntime",
            "version": "1.24.0",
            "targetOs": "macos",
            "targetArch": "aarch64",
            "dylibFilename": "libonnxruntime.dylib",
            "downloadUrl": "https://example.invalid/onnxruntime-osx-arm64-1.24.0.tgz"
        }]))
        .expect("serialize fixture"),
    )
    .expect_err("missing sha256 should fail");

    assert!(error.to_string().contains("sha256"));
}

#[test]
fn ort_runtime_platform_picker_selects_matching_entry() {
    let entries = fixture_entries();

    let picked =
        pick_ort_runtime_entry(&entries, Some("macos"), Some("aarch64")).expect("host match");

    assert_eq!(picked.version, "1.24.0");
    assert_eq!(picked.dylib_filename, "libonnxruntime.dylib");
    assert!(picked.download_url.contains("osx-arm64"));
    assert_eq!(picked.sha256.len(), 64);
}

#[test]
fn ort_runtime_platform_picker_errors_when_platform_missing() {
    let entries = fixture_entries();
    let error = pick_ort_runtime_entry(&entries, Some("linux"), Some("aarch64"))
        .expect_err("missing platform");

    assert_eq!(
        error,
        "No ONNX Runtime download is cataloged for linux/aarch64"
    );
}

/// ORT runtime catalog is only consumed on macOS (where the daemon downloads
/// the runtime on-demand). Windows ships ORT as bundled DLLs in `assets/lib/`
/// via a different loading path; the catalog has no Windows entry by design.
/// Linux/aarch64 etc. ship via the same on-demand mechanism so the entry-must-
/// exist invariant only applies on macOS.
#[cfg(target_os = "macos")]
#[test]
fn ort_runtime_catalog_asset_must_exist_for_host_platform() {
    let catalog_path = repo_root()
        .join("assets")
        .join("catalogs")
        .join("ort-runtime.json");
    let text = std::fs::read_to_string(&catalog_path).unwrap_or_else(|error| {
        panic!(
            "missing bundled ORT runtime catalog at {}: {error}",
            catalog_path.display()
        )
    });
    let entries = load_ort_runtime_catalog_from_str(&text).expect("bundled catalog parses");
    let host_file = ort_runtime_dylib_filename(std::env::consts::OS);
    let entry = pick_ort_runtime_entry(&entries, None, None).expect("bundled catalog host entry");

    assert_eq!(entry.dylib_filename, host_file);
}

#[test]
fn embedding_models_params_contract_accepts_download_runtime_shape() {
    let _params: EmbeddingModelsParams = serde_json::from_value(json!({
        "action": "download-runtime",
        "targetOs": "macos",
        "targetArch": "aarch64",
        "trustExternalCatalog": false
    }))
    .expect("download-runtime params should deserialize");
}

#[test]
#[ignore = "blue-team contract: enable once download-runtime action and schema land"]
fn embedding_models_download_runtime_action_contract() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let _reset = ResetConfigOverrides;
    Config::override_root(temp.path());
    Config::set_database_path(temp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    std::fs::write(config.maenifold_root.join("maenifold.json"), "{}").expect("config fixture");

    let context = ToolContext::new(config, OutputFormat::Json);
    let response = embedding_models(
        &context,
        &json!({
            "action": "download-runtime",
            "targetOs": "macos",
            "targetArch": "aarch64",
            "trustExternalCatalog": false
        }),
    )
    .expect("download-runtime response");
    let payload: serde_json::Value = serde_json::from_str(&response).expect("json response");

    assert_eq!(payload["downloaded"], true);
    assert_eq!(payload["targetPath"], "assets/lib/libonnxruntime.dylib");
    assert_eq!(payload["runtimeId"], "onnxruntime");
}

fn fixture_entries() -> Vec<OrtRuntimeCatalogEntry> {
    load_ort_runtime_catalog_from_str(
        &serde_json::to_string(&json!([
            {
                "id": "onnxruntime",
                "version": "1.24.0",
                "targetOs": "macos",
                "targetArch": "aarch64",
                "dylibFilename": "libonnxruntime.dylib",
                "downloadUrl": "https://example.invalid/onnxruntime-osx-arm64-1.24.0.tgz",
                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            },
            {
                "id": "onnxruntime",
                "version": "1.24.0",
                "targetOs": "macos",
                "targetArch": "x86_64",
                "dylibFilename": "libonnxruntime.dylib",
                "downloadUrl": "https://example.invalid/onnxruntime-osx-x64-1.24.0.tgz",
                "sha256": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
            }
        ]))
        .expect("serialize fixture"),
    )
    .expect("fixture catalog")
}

#[cfg(target_os = "macos")]
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .to_path_buf()
}
