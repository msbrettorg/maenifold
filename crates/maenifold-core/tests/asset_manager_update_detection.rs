mod asset_manager_support;

use asset_manager_support::{
    ConfigResetGuard, PackagedAssetGuard, initialize_assets, invoke_update_assets, isolated_config,
    test_mutex, unique_relative_path,
};
use std::fs;
use std::path::Path;

// T-12.7: RTM NFR-1 - split update-detection coverage from legacy asset_manager_tests.rs.
#[test]
fn update_assets_dry_run_reports_new_files_without_copying() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    initialize_assets(&cfg);

    let relative = unique_relative_path("workflows", "asset-manager-dryrun-new", "json");
    let file_name = Path::new(&relative)
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let _source = PackagedAssetGuard::create(&relative, r#"{"id":"dry-run-source"}"#);

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains(&file_name),
        "expected missing file in dry-run summary: {output}"
    );
    assert!(
        !cfg.assets_path.join(&relative).exists(),
        "dry-run should not copy files into runtime assets"
    );
}

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
        output.contains("Added files: 1") || output.contains("Updated files: 1"),
        "expected copy summary: {output}"
    );
    assert_eq!(
        fs::read_to_string(cfg.assets_path.join(&relative)).expect("read copied file"),
        expected,
        "applied file content must match source content"
    );
}

#[test]
fn update_assets_new_file_in_source_appears_in_added_files() {
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
}

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
}

#[test]
fn update_assets_identical_file_is_not_reported_as_added_or_updated() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = unique_relative_path("colors", "asset-manager-identical", "json");
    let content = r#"{"id":"same","value":1}"#;
    let _source = PackagedAssetGuard::create(&relative, content);
    initialize_assets(&cfg);

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("No changes detected"),
        "identical source and target should produce a no-change summary: {output}"
    );
}
