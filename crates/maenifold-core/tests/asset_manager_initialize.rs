// T-12.7: RTM NFR-1, NFR-2 — Integration tests for AssetManager::initialize_assets().
//
// NOTE: PackagedAssetGuard writes to the live workspace `assets/` directory (resolved via
// CARGO_MANIFEST_DIR), not the TempDir. This is necessary because resolve_packaged_assets_root()
// walks up from the executable to find the assets bundle. Tests are serialized via TEST_MUTEX
// to prevent concurrent mutations to the shared assets directory. If a test process is killed
// (e.g. SIGKILL from a timeout), guard cleanup will not run, potentially orphaning files in
// the workspace assets tree.

#[path = "asset_manager_support/mod.rs"]
mod asset_manager_support;

use asset_manager_support::{
    ConfigResetGuard, PackagedAssetGuard, TEST_MUTEX, hide_packaged_asset_roots, initialize_assets,
    isolated_config, unique_relative_path, unique_suffix, write_file,
};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;

// T-12.7: RTM NFR-1 - split bootstrap coverage from legacy asset_manager_tests.rs.
#[test]
fn initialize_assets_with_source_files_copies_files_to_target() {
    let _guard = TEST_MUTEX.lock();
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
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = format!("{}.txt", unique_suffix("asset-manager-init-skip"));
    let source_content = "source content";
    let pre_existing_target = "original target content";
    // R-1: Make the distinguishing condition explicit — this test is only meaningful if the
    // source and pre-existing target have different content, otherwise skip-vs-copy is ambiguous.
    assert_ne!(
        source_content, pre_existing_target,
        "test is only meaningful if source != pre-existing target"
    );

    let _source = PackagedAssetGuard::create(&relative, source_content);
    let target = cfg.assets_path.join(&relative);
    write_file(&target, pre_existing_target);

    initialize_assets(&cfg);

    assert_eq!(
        fs::read_to_string(&target).expect("read target"),
        pre_existing_target,
        "initialize_assets must not overwrite an existing target file"
    );
}

// T-12.7: RTM NFR-1 - R-2: Cover Branch A (directory creation) of initialize_assets.
#[test]
fn initialize_assets_creates_subdirectories_for_nested_files() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = unique_relative_path("subdir", "asset-manager-init-nested", "txt");
    let expected = "nested asset content";
    let _source = PackagedAssetGuard::create(&relative, expected);
    let target = cfg.assets_path.join(&relative);
    let _ = fs::remove_file(&target);

    initialize_assets(&cfg);

    assert!(
        target.exists(),
        "expected nested file at {}",
        target.display()
    );
    assert!(
        target.parent().expect("parent dir").is_dir(),
        "subdirectory should have been created by initialize_assets"
    );
    assert_eq!(
        fs::read_to_string(&target).expect("read target"),
        expected,
        "nested file content should match source"
    );
}

// T-12.7: RTM NFR-2 - R-3: Cover the graceful no-op when resolve_packaged_assets_root() returns None.
#[test]
fn initialize_assets_is_no_op_when_no_packaged_assets_exist() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    let _hidden = hide_packaged_asset_roots();

    initialize_assets(&cfg); // must not panic or error

    // assets_path should exist (ensure_directories) but be otherwise empty
    let entries: Vec<_> = fs::read_dir(&cfg.assets_path)
        .expect("read assets dir")
        .collect();
    assert!(
        entries.is_empty(),
        "no files should be copied when source is absent"
    );
}

#[cfg(unix)]
#[test]
fn initialize_assets_skips_directory_symlinks_and_continues_copying_regular_assets() {
    struct DirectorySymlinkGuard {
        link: std::path::PathBuf,
        target: std::path::PathBuf,
    }

    impl Drop for DirectorySymlinkGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.link);
            let _ = fs::remove_dir(&self.target);
        }
    }

    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let symlinked_dir_name = unique_suffix("asset-manager-init-symlink-dir");
    let symlinked_dir = asset_manager_support::workspace_assets_root().join(&symlinked_dir_name);
    let real_dir =
        asset_manager_support::workspace_assets_root().join(format!("{symlinked_dir_name}-target"));
    fs::create_dir_all(&real_dir).expect("create symlink target dir");
    symlink(&real_dir, &symlinked_dir).expect("create packaged directory symlink");
    let _symlink_guard = DirectorySymlinkGuard {
        link: symlinked_dir,
        target: real_dir,
    };

    let copied_relative = unique_relative_path("roles", "asset-manager-init-regular", "json");
    let copied_content = r#"{"id":"asset-manager-init-regular"}"#;
    let _copied = PackagedAssetGuard::create(&copied_relative, copied_content);

    initialize_assets(&cfg);

    assert!(
        cfg.assets_path.join(&copied_relative).exists(),
        "regular packaged assets should still be copied when directory symlinks are present"
    );
}
