#[path = "dynamic_model/support.rs"]
mod dynamic_model_support;

use dynamic_model_support::{TEST_MUTEX, TestContext, model_output_dimensions};
use maenifold_core::vector;

// T-9.6: RTM FR-7 / RTM-050 / RTM-051 / RTM-053 — runtime model discovery should
// find a valid *.onnx file in assets/models and load fallback tokenizer assets from that directory.
#[test]
fn t9_6_model_discovery_finds_shadow_onnx_file() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("test-model.onnx");
    // T-9.6: RTM-051 — derive expected dims from ONNX tensor metadata, not a hardcoded constant.
    let expected_dims = model_output_dimensions(&ctx.shadow_models_dir().join("test-model.onnx"));

    vector::load_model().expect("load discovered shadow model");

    assert!(
        vector::is_initialized(),
        "model should be initialized after successful load"
    );
    assert_eq!(vector::model_filename().as_deref(), Some("test-model.onnx"));
    assert_eq!(vector::embedding_dimensions(), Some(expected_dims));

    let embedding = vector::generate_embedding("dynamic model loading").expect("embedding");
    assert_eq!(embedding.len(), expected_dims);
}

// T-9.6: RTM FR-7 / RTM-051 — runtime embedding dimensions must match the loaded
// ONNX output tensor metadata instead of a hardcoded constant.
#[test]
fn t9_6_embedding_dimensions_match_model_output_tensor() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("output-dims.onnx");
    let expected_dims = model_output_dimensions(&ctx.shadow_models_dir().join("output-dims.onnx"));

    vector::load_model().expect("load model for output dimension check");

    assert_eq!(vector::embedding_dimensions(), Some(expected_dims));
    assert_eq!(
        vector::generate_embedding("runtime dimensions")
            .expect("embedding with runtime dimensions")
            .len(),
        expected_dims
    );
}

// T-9.6: RTM FR-7 / RTM-055 — when no *.onnx file exists anywhere in assets/models,
// embedding APIs should degrade gracefully without panicking or reporting stale dimensions.
#[test]
fn t9_6_no_model_generate_embedding_returns_err_and_dimensions_none() {
    let _guard = TEST_MUTEX.lock();
    let mut ctx = TestContext::new();
    ctx.hide_packaged_model();

    // T-9.6: RTM-055 — exercise the discovery-failure path: load_model() must return Err
    // when no *.onnx file is found, not just rely on the already-cleaned None state.
    let load_result = vector::load_model();
    assert!(
        load_result.is_err(),
        "load_model() must return Err when no model exists"
    );
    assert!(
        !vector::is_initialized(),
        "model must not be initialized after failed load"
    );
    assert_eq!(vector::embedding_dimensions(), None);
    assert_eq!(vector::model_filename(), None);

    let result = vector::generate_embedding("hello world");

    assert!(
        result.is_err(),
        "missing model should return Err, not panic"
    );
    // Verify no side-effect mutation from a failed generate_embedding() call.
    assert_eq!(vector::embedding_dimensions(), None);
    assert_eq!(vector::model_filename(), None);
}

// T-9.6: RTM FR-7 / RTM-056 — when multiple *.onnx files exist, the largest file
// should be selected for runtime model loading.
#[test]
fn t9_6_multiple_onnx_files_select_largest() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("large.onnx");
    ctx.write_dummy_onnx("small.onnx", 100);

    vector::load_model().expect("load largest discovered model");

    assert_eq!(vector::model_filename().as_deref(), Some("large.onnx"));
    assert_eq!(vector::embedding_dimensions(), Some(384));
}
