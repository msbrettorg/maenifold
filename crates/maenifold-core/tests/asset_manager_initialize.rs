#[path = "asset_manager_support/mod.rs"]
mod asset_manager_support;

use asset_manager_support::{
    ConfigResetGuard, PackagedAssetGuard, initialize_assets, isolated_config, test_mutex,
    unique_suffix, write_file,
};
use std::fs;

// T-12.7: RTM NFR-1 - split bootstrap coverage from legacy asset_manager_tests.rs.
#[test]
fn initialize_assets_with_source_files_copies_files_to_target() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = format!("{}.txt", unique_suffix("asset-manager-init-copy"));
    let expected = "hello from InitializeAssets test";
    let _source = PackagedAssetGuard::create(&relative, expected);
    let target = cfg.assets_path.join(&relative);
    let _ = fs::remove_file(&target);

    initialize_assets(&cfg);

    assert!(
        target.exists(),
        "expected copied file at {}",
        target.display()
    );
    assert_eq!(
        fs::read_to_string(&target).expect("read target"),
        expected,
        "copied file content should match source content"
    );
}

#[test]
fn initialize_assets_skips_existing_files_and_does_not_overwrite() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = format!("{}.txt", unique_suffix("asset-manager-init-skip"));
    let _source = PackagedAssetGuard::create(&relative, "source content");
    let target = cfg.assets_path.join(&relative);
    write_file(&target, "original target content");

    initialize_assets(&cfg);

    assert_eq!(
        fs::read_to_string(&target).expect("read target"),
        "original target content",
        "initialize_assets must not overwrite an existing target file"
    );
}
