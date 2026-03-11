#[path = "vector_tools/support.rs"]
mod vector_tools_support;

use maenifold_core::vector;
use vector_tools_support::TEST_MUTEX;

// T-12.6: RTM NFR-1 — runtime vector integration coverage remains organized after the split.
// T-12.7: RTM NFR-1 — acceptance hygiene trims smoke-only runtime vector tests that overclaim behavior.

// T-8.4: RTM FR-33.1 — cleanup drops model resources and a later load reinitializes them safely.
#[test]
fn cleanup_properly_disposes_resources() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    vector::load_model().expect("initial model load");
    assert!(
        vector::is_initialized(),
        "model should be initialized before cleanup"
    );
    vector::cleanup();
    assert!(
        !vector::is_initialized(),
        "cleanup should dispose initialized resources"
    );
    vector::load_model().expect("reload after cleanup");
    assert!(
        vector::is_initialized(),
        "model should reinitialize after cleanup"
    );
}

// T-8.4: RTM FR-33.1 — real ONNX embeddings contain signal rather than fallback-like zero or uniform vectors.
#[test]
fn embedding_quality_differs_between_onnx_and_fallback() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    let embedding = vector::generate_embedding("graph semantic search quality").expect("embedding");
    assert!(
        embedding.iter().any(|value| value.abs() > f32::EPSILON),
        "real embedding should not be all zeros"
    );
    let first = embedding[0];
    assert!(
        embedding
            .iter()
            .any(|value| (value - first).abs() > f32::EPSILON),
        "real embedding should not be uniform"
    );
}
