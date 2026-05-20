use std::collections::HashSet;

use super::inference::select_active_input_names;

/// T-334.8: Qwen3-style models declare only input_ids + attention_mask.
/// The inference path must NOT emit token_type_ids for such models.
#[test]
fn inference_emits_only_declared_inputs_for_qwen_style_model() {
    let mut input_names = HashSet::new();
    input_names.insert("input_ids".to_string());
    input_names.insert("attention_mask".to_string());

    let active = select_active_input_names(&input_names);

    assert_eq!(active.len(), 2);
    assert!(active.contains(&"input_ids"));
    assert!(active.contains(&"attention_mask"));
    assert!(!active.contains(&"token_type_ids"));
}

/// T-334.8: BERT-style models declare all three inputs.
/// The inference path must emit all three for such models.
#[test]
fn inference_emits_all_three_inputs_for_bert_style_model() {
    let mut input_names = HashSet::new();
    input_names.insert("input_ids".to_string());
    input_names.insert("attention_mask".to_string());
    input_names.insert("token_type_ids".to_string());

    let active = select_active_input_names(&input_names);

    assert_eq!(active.len(), 3);
    assert!(active.contains(&"input_ids"));
    assert!(active.contains(&"attention_mask"));
    assert!(active.contains(&"token_type_ids"));
}

/// T-334.8: Integration smoke test — loads the real qwen3-embedding-0.6b model
/// and verifies a single embedding can be produced without error.
/// Marked #[ignore] so CI doesn't depend on model availability.
#[test]
#[ignore]
fn inference_embeds_real_qwen3_model_smoke() {
    let home = std::env::var("HOME").expect("HOME env var not set");
    let model_dir =
        std::path::PathBuf::from(home).join("maenifold/assets/models/qwen3-embedding-0.6b");

    if !model_dir.join("model.onnx").exists() {
        println!(
            "Skipping: qwen3-embedding-0.6b model not found at {}",
            model_dir.display()
        );
        return;
    }

    let result = super::inference::generate_embedding("hello world");
    assert!(result.is_ok(), "embedding failed: {:?}", result.err());
    let embedding = result.unwrap();
    assert!(!embedding.is_empty(), "embedding vector is empty");
    assert!(
        embedding.iter().any(|v| *v != 0.0),
        "embedding is all zeros"
    );
}
