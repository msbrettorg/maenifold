#[allow(clippy::duplicate_mod)]
#[path = "asset_manager_support/mod.rs"]
mod asset_manager_support;

use asset_manager_support::{
    ConfigResetGuard, PackagedAssetGuard, TEST_MUTEX, hide_packaged_asset_roots, initialize_assets,
    invoke_update_assets, isolated_config, unique_relative_path, unique_suffix, write_file,
};
use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::symlink;

// T-12.7: RTM NFR-1 - split error and summary coverage from legacy asset_manager_tests.rs.
#[test]
fn update_assets_when_target_dir_missing_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    initialize_assets(&cfg);
    fs::remove_dir_all(&cfg.assets_path).expect("remove runtime assets dir");

    let err = invoke_update_assets(&cfg, true).expect_err("missing runtime assets should error");

    assert!(
        err.to_string()
            .contains("Target assets directory does not exist. Run initialization first."),
        "unexpected error: {err}"
    );
}

// T-12.7: RTM NFR-1 - combined-state summary formatting (added + updated + error).
#[cfg(unix)]
#[test]
fn update_assets_summary_formats_combined_counts_and_sections() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    initialize_assets(&cfg);

    let added_relative = unique_relative_path("workflows", "asset-manager-summary-added", "json");
    let updated_relative = unique_relative_path("roles", "asset-manager-summary-updated", "json");
    let skipped_relative = unique_relative_path("colors", "asset-manager-summary-skipped", "json");

    let _added = PackagedAssetGuard::create(&added_relative, r#"{"id":"summary-added"}"#);
    let _updated =
        PackagedAssetGuard::create(&updated_relative, r#"{"id":"summary-updated","version":2}"#);
    let _skipped = PackagedAssetGuard::create(&skipped_relative, r#"{"id":"summary-skipped"}"#);

    write_file(
        &cfg.assets_path.join(&updated_relative),
        r#"{"id":"summary-updated","version":1}"#,
    );

    let skipped_target = cfg.assets_path.join(&skipped_relative);
    fs::create_dir_all(skipped_target.parent().expect("skipped target parent"))
        .expect("create skipped target dir");
    let symlink_target = cfg
        .assets_path
        .join(format!("{}.txt", unique_suffix("symlink-target")));
    write_file(&symlink_target, "symlink target content");
    symlink(&symlink_target, &skipped_target).expect("create target symlink");

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("Asset Update Summary"),
        "missing summary header: {output}"
    );
    assert!(
        output.contains("Added files: 1"),
        "missing added count: {output}"
    );
    assert!(
        output.contains("Updated files: 1"),
        "missing updated count: {output}"
    );
    assert!(
        output.contains("Errors: 1"),
        "missing error count: {output}"
    );
    assert!(
        output.contains("### Added Files"),
        "missing Added Files section: {output}"
    );
    assert!(
        output.contains("### Updated Files"),
        "missing Updated Files section: {output}"
    );
    assert!(
        output.contains("### Errors"),
        "missing Errors section: {output}"
    );
    assert!(
        output.contains(
            Path::new(&added_relative)
                .file_name()
                .expect("added file name")
                .to_string_lossy()
                .as_ref()
        ),
        "missing added file name: {output}"
    );
    assert!(
        output.contains(
            Path::new(&updated_relative)
                .file_name()
                .expect("updated file name")
                .to_string_lossy()
                .as_ref()
        ),
        "missing updated file name: {output}"
    );
    assert!(
        output.contains("skipped symlink target"),
        "missing symlink error detail: {output}"
    );
}

// T-12.7: RTM NFR-1 - PARITY-U01 guard: symlinks produce errors, not a "Skipped" category.
// The pre-parity "Skipped symlinks:" count line and "### Skipped Symlink Targets" section
// were deliberately removed for C# oracle parity. This test verifies they stay absent.
#[cfg(unix)]
#[test]
fn update_assets_parity_u01_omits_skipped_symlinks_count_and_section() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    initialize_assets(&cfg);

    let skipped_relative = unique_relative_path("colors", "asset-manager-parity-skipped", "json");
    let _skipped = PackagedAssetGuard::create(&skipped_relative, r#"{"id":"parity-skipped"}"#);

    let skipped_target = cfg.assets_path.join(&skipped_relative);
    fs::create_dir_all(skipped_target.parent().expect("skipped target parent"))
        .expect("create skipped target dir");
    let symlink_target = cfg
        .assets_path
        .join(format!("{}.txt", unique_suffix("parity-symlink-target")));
    write_file(&symlink_target, "symlink target content");
    symlink(&symlink_target, &skipped_target).expect("create target symlink");

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        !output.contains("Skipped symlinks:"),
        "Skipped symlinks count line should not appear (PARITY-U01): {output}"
    );
    assert!(
        !output.contains("### Skipped Symlink Targets"),
        "Skipped Symlink Targets section should not appear (PARITY-U01): {output}"
    );
}

// T-12.7: RTM NFR-1 - source-missing error path returns Err without partial summary rendering.
// Also verifies the basic error-type check (Err, not Ok) previously covered by the
// removed update_assets_when_source_dir_missing_returns_error test.
#[test]
fn update_assets_source_dir_missing_errors_without_summary_counts() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    let _hidden_roots = hide_packaged_asset_roots();

    let err = invoke_update_assets(&cfg, true).expect_err("missing packaged assets should error");
    let text = err.to_string();

    assert!(
        text.contains("Source assets directory not found"),
        "missing source error should surface clearly: {text}"
    );
    assert!(
        !text.contains("Added files:"),
        "top-level error should not render summary counts: {text}"
    );
}

#[test]
fn update_assets_source_target_mapping_scans_all_four_asset_catalog_subdirectories() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    initialize_assets(&cfg);

    let categories = ["workflows", "roles", "colors", "perspectives"];
    let mut expected_names = Vec::new();
    let mut _guards = Vec::new();

    for category in categories {
        let relative =
            unique_relative_path(category, &format!("asset-manager-map-{category}"), "json");
        expected_names.push(
            Path::new(&relative)
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned(),
        );
        _guards.push(PackagedAssetGuard::create(
            &relative,
            &format!(r#"{{"id":"{category}-mapping"}}"#),
        ));
    }

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    for file_name in expected_names {
        assert!(
            output.contains(&file_name),
            "expected mapped file {file_name} to appear in summary: {output}"
        );
    }
}
