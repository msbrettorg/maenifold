// T-12.7: RTM NFR-1 — ToolRegistry dispatch-layer (boundary) tests for UpdateAssets.
// These tests exercise the full MCP tool registration and dispatch pipeline via
// ToolRegistry::global().try_invoke("UpdateAssets", ...) rather than calling
// update_assets() directly (which sibling files asset_manager_update_detection.rs
// and asset_manager_errors.rs cover at the direct-fn layer).

#[path = "asset_manager_support/mod.rs"]
mod asset_manager_support;

use asset_manager_support::{
    ConfigResetGuard, PackagedAssetGuard, TEST_MUTEX, isolated_config, unique_relative_path,
    write_file,
};
use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, ToolRegistry};
use serde_json::json;
use std::fs;
use std::path::Path;

// T-12.7: RTM NFR-1 — KEEP. Most complete new-file dry-run test in the suite
// (count + name + section heading + not-copied).
#[test]
fn update_assets_dry_run_detects_files_missing_from_target() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let relative = unique_relative_path("workflows", "t127-added", "json");
    let file_name = Path::new(&relative)
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let _source = PackagedAssetGuard::create(
        &relative,
        r#"{"id":"t127-added","name":"Added boundary workflow"}"#,
    );

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("Added files: 1"),
        "expected one added file in summary: {output}"
    );
    assert!(
        output.contains("### Added Files"),
        "expected Added Files section heading: {output}"
    );
    assert!(
        output.contains(&file_name),
        "expected missing file name in summary: {output}"
    );
    assert!(
        !cfg.assets_path.join(&relative).exists(),
        "dry-run should not copy missing files"
    );
}

// T-12.7: RTM NFR-1 — REWRITE. Exercises the SHA-256 code path via *source-modification*
// (source overwritten after initialize_assets copies v1 to target). This is structurally
// distinct from Test 6, which modifies the *target* after initialization. Both produce
// source != target with equal byte lengths, ensuring SHA-256 comparison (not size) detects
// the change. Test 2 validates the source-modification direction; Test 6 validates the
// target-modification direction.
#[test]
fn update_assets_dry_run_detects_sha_changes_for_existing_files() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    // Both v1 and v2 are the same byte length (33 bytes), so size comparison alone would miss
    // the change. Only SHA-256 hashing detects the difference.
    let relative = unique_relative_path("roles", "t127-updated", "json");
    let file_name = Path::new(&relative)
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let source = PackagedAssetGuard::create(&relative, r#"{"id":"t127-updated","version":1}"#);
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");
    write_file(source.path(), r#"{"id":"t127-updated","version":2}"#);

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("Updated files: 1"),
        "expected one updated file in summary: {output}"
    );
    assert!(
        output.contains("### Updated Files"),
        "expected Updated Files section heading: {output}"
    );
    assert!(
        output.contains(&file_name),
        "expected updated file name in summary: {output}"
    );
    // Dry-run must not modify the target — it should still contain v1 content.
    let target_content = fs::read_to_string(cfg.assets_path.join(&relative)).expect("read target");
    assert!(
        target_content.contains(r#""version":1"#),
        "dry-run must not overwrite target with v2: {target_content}"
    );
}

// T-12.7: RTM NFR-1 — REWRITE. Strict classification: a new file that never existed in the
// target is Added by definition. The previous `||` disjunction masked potential
// misclassification. Content-equality assertion is the load-bearing value of this test.
#[test]
fn update_assets_non_dry_run_copies_added_files_into_runtime_assets() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let relative = unique_relative_path("workflows", "t127-copy", "json");
    let source_content = r#"{"id":"t127-copy","name":"Copied workflow"}"#;
    let _source = PackagedAssetGuard::create(&relative, source_content);

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": false}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    // Strict: a file created after initialize_assets is *Added*, not *Updated*.
    assert!(
        output.contains("Added files: 1"),
        "new file should be classified Added, not Updated: {output}"
    );
    let target_content = fs::read_to_string(cfg.assets_path.join(&relative)).expect("target file");
    assert_eq!(target_content, source_content);
}

// T-12.7: RTM NFR-1 — REWRITE. Added negative guards to distinguish a correct no-change
// response from a malformed response that happens to contain "No changes detected" alongside
// stray count lines.
#[test]
fn update_assets_reports_no_changes_when_source_and_target_match() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("No changes detected"),
        "expected no-change summary: {output}"
    );
    assert!(
        !output.contains("Added files:"),
        "no-change output must not list Added counts: {output}"
    );
    assert!(
        !output.contains("Updated files:"),
        "no-change output must not list Updated counts: {output}"
    );
}

// T-12.7: RTM NFR-1 — KEEP. Unique across the entire suite: tests the learn-mode early-exit
// path at the ToolRegistry dispatch layer. The `learn` parameter branches before any
// filesystem access.
#[test]
fn update_assets_learn_mode_returns_help_instead_of_update_summary() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"learn": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    // The help document always contains "UpdateAssets" in its heading or description.
    assert!(
        output.contains("UpdateAssets"),
        "expected help documentation containing tool name: {output}"
    );
    assert!(
        !output.contains("Asset Update Summary"),
        "learn mode should not render update summary: {output}"
    );
}

// T-12.7: RTM NFR-1 — KEEP. Canonical SHA-256 regression guard. Creates source={"a":1},
// copies to target via initialize_assets, then overwrites *target* with {"b":2} (same 7
// bytes). If the implementation dropped SHA-256 and compared only byte lengths, the update
// would be missed and this test would fail.
#[test]
fn update_assets_uses_sha256_not_file_size_for_equal_length_content() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = unique_relative_path("roles", "t127-same-size", "json");
    let source = PackagedAssetGuard::create(&relative, r#"{"a":1}"#);
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");
    write_file(&cfg.assets_path.join(&relative), r#"{"b":2}"#);

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("Updated files: 1"),
        "same-size content changes must still be reported: {output}"
    );
    assert!(
        output.contains("### Updated Files"),
        "expected Updated Files section heading: {output}"
    );
    assert!(
        output.contains(
            source
                .path()
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .as_ref()
        ),
        "updated same-size file should be listed: {output}"
    );
}
