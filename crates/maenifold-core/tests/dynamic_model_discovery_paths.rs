#[path = "dynamic_model/support.rs"]
mod dynamic_model_support;

use dynamic_model_support::{
    TEST_MUTEX, TestContext, model_output_dimensions, packaged_model_fixture_available,
};
use maenifold_core::vector;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;

const SYMLINK_HELPER_ENV: &str = "MAENIFOLD_T20_5_SYMLINK_HELPER";
const SYMLINK_HELPER_VALUE: &str =
    "t20_5_symlinked_model_discovery_uses_canonicalized_executable_path";

#[cfg(unix)]
fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}

// T-20.5: RTM FR-7 / RTM-147 — a symlink-invoked test binary should still
// resolve runtime models from the real executable's adjacent assets directory.
// P3: Skip on non-Unix platforms to surface the skip visibly instead of silently
// passing (symlink creation may fail on Windows without developer mode).
#[test]
#[cfg_attr(not(unix), ignore)]
fn t20_5_symlinked_model_discovery_uses_canonicalized_executable_path() {
    if env::var_os(SYMLINK_HELPER_ENV).as_deref() == Some(OsStr::new(SYMLINK_HELPER_VALUE)) {
        vector::cleanup();
        // P2: Derive expected dimensions from the shadow model at the canonicalized exe
        // path instead of hardcoding 384 — keeps the assertion correct when the packaged
        // model is replaced with a different-dimension model.
        let shadow_model = env::current_exe()
            .expect("current test executable")
            .canonicalize()
            .expect("canonicalize test executable")
            .parent()
            .expect("test binary parent")
            .join("assets/models/symlinked-shadow.onnx");
        let expected_dims = model_output_dimensions(&shadow_model);

        vector::load_model().expect("load model through symlink helper");
        assert!(
            vector::is_initialized(),
            "model should be initialized after load"
        );
        assert_eq!(
            vector::model_filename().as_deref(),
            Some("symlinked-shadow.onnx")
        );
        assert_eq!(vector::embedding_dimensions(), Some(expected_dims));
        return;
    }

    let _guard = TEST_MUTEX.lock();
    if !packaged_model_fixture_available() {
        eprintln!("Skipping dynamic model test: packaged ONNX fixture is unavailable");
        return;
    }
    let mut ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("symlinked-shadow.onnx");
    ctx.hide_packaged_model();

    let test_binary = env::current_exe().expect("current test executable");
    let symlink_dir = ctx.config().maenifold_root.join("symlink-bin");
    fs::create_dir_all(&symlink_dir).expect("create symlink dir");
    let symlink_path = symlink_dir.join(
        test_binary
            .file_name()
            .expect("test executable file name available"),
    );
    if symlink_path.exists() {
        fs::remove_file(&symlink_path).expect("remove stale test symlink");
    }
    if let Err(error) = create_file_symlink(&test_binary, &symlink_path) {
        eprintln!("Skipping symlink discovery test: {error}");
        return;
    }

    let output = Command::new(&symlink_path)
        .arg("--exact")
        .arg("t20_5_symlinked_model_discovery_uses_canonicalized_executable_path")
        .arg("--nocapture")
        .env(SYMLINK_HELPER_ENV, SYMLINK_HELPER_VALUE)
        .output()
        .expect("spawn symlinked helper process");

    assert!(
        output.status.success(),
        "symlink helper failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// T-20.5: RTM FR-7 / RTM-148 — when neither the executable-adjacent nor Cargo
// walk-up roots contain a model, discovery should fall back to the shared runtime
// assets root at Config::current().assets_path/models.
#[test]
fn t20_5_model_discovery_falls_back_to_shared_runtime_root() {
    let _guard = TEST_MUTEX.lock();
    if !packaged_model_fixture_available() {
        eprintln!("Skipping dynamic model test: packaged ONNX fixture is unavailable");
        return;
    }
    let mut ctx = TestContext::new();
    // P1: install_valid_shared_model_with_fallback_assets copies from the packaged
    // model path. hide_packaged_model() renames that path. Install MUST come first;
    // reversing these calls would cause the install to fail with "file not found".
    ctx.install_valid_shared_model_with_fallback_assets("shared-runtime.onnx");
    ctx.hide_packaged_model();

    // P2: Derive expected dimensions from the installed model instead of hardcoding
    // 384 — keeps the assertion correct when the packaged model is replaced.
    let expected_dims = model_output_dimensions(
        &ctx.config()
            .assets_path
            .join("models")
            .join("shared-runtime.onnx"),
    );

    vector::load_model().expect("load model from shared runtime root");
    assert!(
        vector::is_initialized(),
        "model should be initialized after load"
    );

    assert_eq!(
        vector::model_filename().as_deref(),
        Some("shared-runtime.onnx")
    );
    assert_eq!(vector::embedding_dimensions(), Some(expected_dims));
}
