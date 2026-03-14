#[path = "asset_manager_support/mod.rs"]
mod asset_manager_support;

use asset_manager_support::{
    ConfigResetGuard, PackagedAssetGuard, initialize_assets, invoke_update_assets, isolated_config,
    test_mutex, unique_relative_path,
};
use std::fs;
use std::path::Path;

// T-12.7: RTM NFR-1 - split update-detection coverage from legacy asset_manager_tests.rs.

// T-12.7: RTM NFR-1 — REWRITE (absorbed T1). Complete dry-run new-file scenario at the direct-fn
// layer: section heading, file name, count line, and "not copied" guard.
#[test]
fn update_assets_dry_run_emits_added_files_section_for_new_source_file() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    initialize_assets(&cfg);

    let relative = unique_relative_path("roles", "asset-manager-added", "json");
    let file_name = Path::new(&relative)
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let _source = PackagedAssetGuard::create(&relative, r#"{"id":"brand-new"}"#);

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("### Added Files"),
        "expected added files section: {output}"
    );
    assert!(
        output.contains(&file_name),
        "expected added file in summary: {output}"
    );
    assert!(
        output.contains("Added files: 1"),
        "expected count line for one added file: {output}"
    );
    assert!(
        !cfg.assets_path.join(&relative).exists(),
        "dry-run should not copy files into runtime assets"
    );
}

// T-12.7: RTM NFR-1 — REWRITE. Strict classification: a new file that never existed in the target
// is definitionally Added, not Updated. The previous OR disjunction masked a potential
// classification bug.
#[test]
fn update_assets_apply_copies_new_files_to_target() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    initialize_assets(&cfg);

    let relative = unique_relative_path("workflows", "asset-manager-apply-new", "json");
    let expected = r#"{"id":"apply-source","name":"apply source content"}"#;
    let _source = PackagedAssetGuard::create(&relative, expected);

    let output = invoke_update_assets(&cfg, false).expect("apply update assets");

    assert!(
        output.contains("Added files: 1"),
        "new file must be classified as Added, not Updated: {output}"
    );
    assert_eq!(
        fs::read_to_string(cfg.assets_path.join(&relative)).expect("read copied file"),
        expected,
        "applied file content must match source content"
    );
}

// T-12.7: RTM NFR-1 — REWRITE. Intentionally exercises the size-diff branch (different byte
// lengths). The SHA-256-only branch (same size, different content) is tested in
// update_assets_boundary::update_assets_uses_sha256_not_file_size_for_equal_length_content.
#[test]
fn update_assets_changed_file_appears_in_updated_files() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = unique_relative_path("roles", "asset-manager-updated", "json");
    let source = PackagedAssetGuard::create(&relative, r#"{"id":"updated","version":2}"#);
    initialize_assets(&cfg);
    fs::write(
        cfg.assets_path.join(&relative),
        r#"{"id":"updated","version":1}"#,
    )
    .expect("write modified runtime asset");

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("### Updated Files"),
        "expected updated files section: {output}"
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
        "expected updated file in summary: {output}"
    );
    assert!(
        output.contains("Updated files: 1"),
        "expected count line for one updated file: {output}"
    );
}

// T-12.7: RTM NFR-1 — REWRITE. Simplified: initialize_assets on an isolated config produces a
// clean source==target state without a synthetic PackagedAssetGuard. Added negative assertions
// for Added/Updated categories to verify neither is emitted.
#[test]
fn update_assets_identical_file_is_not_reported_as_added_or_updated() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    initialize_assets(&cfg);

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("No changes detected"),
        "identical source and target should produce a no-change summary: {output}"
    );
    assert!(
        !output.contains("Added files:"),
        "no-change output must not contain Added files category: {output}"
    );
    assert!(
        !output.contains("Updated files:"),
        "no-change output must not contain Updated files category: {output}"
    );
}
