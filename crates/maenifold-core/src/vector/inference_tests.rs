use super::ModelInputMetadata;
use super::inference::{
    ChunkTokenInputs, EncodedRow, build_cache_tensor_shape, build_chunk_token_inputs,
    build_model_inputs, build_position_ids_from_attention_mask, parse_past_key_value_inputs,
    plan_model_inputs, select_kv_zero_axis,
};
use super::tokenizer::resolve_max_tokens;
use ort::value::TensorElementType;
use std::fs;

fn metadata(
    name: &str,
    dtype: TensorElementType,
    dims: &[i64],
    dimension_symbols: &[&str],
) -> ModelInputMetadata {
    ModelInputMetadata {
        name: name.to_string(),
        dtype,
        dims: dims.to_vec(),
        dimension_symbols: dimension_symbols
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn chunk_inputs() -> ChunkTokenInputs {
    build_chunk_token_inputs(
        &[EncodedRow {
            input_ids: vec![11, 12, 13],
            attention_mask: vec![1, 1, 1],
            token_type_ids: vec![0, 0, 0],
        }],
        0,
        32_768,
    )
}

#[test]
fn qwen_token_limit_resolution_uses_trusted_source_over_tokenizer_hint() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp_dir.path().join("config.json"),
        r#"{"max_position_embeddings":32768}"#,
    )
    .expect("write config.json");

    let resolved = resolve_max_tokens(
        temp_dir.path(),
        "qwen3-embedding-0.6b",
        8_192,
        Some(131_072),
    )
    .expect("resolve max tokens");

    assert_eq!(resolved.value, 32_768);
    assert_eq!(resolved.source_tag, "config.json:max_position_embeddings");
}

#[test]
fn chunk_local_padding_uses_actual_request_width_not_model_ceiling() {
    let chunk = build_chunk_token_inputs(
        &[
            EncodedRow {
                input_ids: vec![1, 2, 3],
                attention_mask: vec![1, 1, 1],
                token_type_ids: vec![0, 0, 0],
            },
            EncodedRow {
                input_ids: vec![4],
                attention_mask: vec![1],
                token_type_ids: vec![0],
            },
        ],
        99,
        32_768,
    );

    assert_eq!(chunk.seq_len, 3);
    assert_eq!(chunk.input_ids.len(), 6);
    assert_eq!(chunk.input_ids, vec![1, 2, 3, 4, 99, 99]);
    assert_ne!(chunk.seq_len, 32_768);
}

#[test]
fn position_ids_follow_attention_mask_and_zero_pad_tail() {
    let position_ids = build_position_ids_from_attention_mask(&[1, 1, 1, 0, 0], 1, 5);
    assert_eq!(position_ids, vec![0, 1, 2, 0, 0]);
}

#[test]
fn kv_parser_derives_layers_from_declared_names() {
    let metadata = vec![
        metadata(
            "past_key_values.3.value",
            TensorElementType::Float32,
            &[-1, 8, -1, 128],
            &[],
        ),
        metadata(
            "past_key_values.1.key",
            TensorElementType::Float32,
            &[-1, 8, -1, 128],
            &[],
        ),
        metadata(
            "past_key_values.0.value",
            TensorElementType::Float32,
            &[-1, 8, -1, 128],
            &[],
        ),
        metadata(
            "past_key_values.3.key",
            TensorElementType::Float32,
            &[-1, 8, -1, 128],
            &[],
        ),
    ];

    let parsed = parse_past_key_value_inputs(&metadata);

    assert_eq!(parsed.len(), 4);
    assert_eq!(
        parsed.iter().map(|input| input.layer).collect::<Vec<_>>(),
        vec![0, 1, 3, 3]
    );
    assert_eq!(
        parsed
            .iter()
            .map(|input| input.kind.clone())
            .collect::<Vec<_>>(),
        vec![
            "value".to_string(),
            "key".to_string(),
            "key".to_string(),
            "value".to_string(),
        ]
    );
}

#[test]
fn token_input_dtype_dispatch_uses_declared_integer_type() {
    let metadata = vec![
        metadata(
            "input_ids",
            TensorElementType::Int32,
            &[-1, -1],
            &["batch_size", "sequence_length"],
        ),
        metadata(
            "attention_mask",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch_size", "sequence_length"],
        ),
    ];

    let inputs = plan_model_inputs(&metadata, 1, &chunk_inputs()).expect("plan inputs");
    let input_ids = inputs
        .iter()
        .find(|input| {
            matches!(
                input,
                super::inference::PlannedModelInput::Integer { name, .. } if name == "input_ids"
            )
        })
        .expect("input_ids plan");

    match input_ids {
        super::inference::PlannedModelInput::Integer { dtype, values, .. } => {
            assert_eq!(*dtype, TensorElementType::Int32);
            assert_eq!(values, &vec![11, 12, 13]);
        }
        other => panic!("expected integer input plan, got {other:?}"),
    }
}

#[test]
fn unsupported_integer_dtype_error_includes_contract_details() {
    let metadata = vec![
        metadata(
            "input_ids",
            TensorElementType::Bool,
            &[-1, -1],
            &["batch", "sequence"],
        ),
        metadata(
            "attention_mask",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
    ];

    let error = build_model_inputs(&metadata, 1, &chunk_inputs()).expect_err("unsupported dtype");
    let message = error.to_string();

    assert!(message.contains("declared integer dtype is unsupported for token input"));
    assert!(message.contains("input=input_ids"));
    assert!(message.contains("dtype=bool"));
    assert!(message.contains("shape=[-1, -1]"));
    assert!(message.contains("symbols=[\"batch\", \"sequence\"]"));
}

#[test]
fn kv_dtype_dispatch_uses_declared_cache_type() {
    let metadata = vec![
        metadata(
            "input_ids",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
        metadata(
            "attention_mask",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
        metadata(
            "past_key_values.0.key",
            TensorElementType::Float64,
            &[-1, 8, -1, 128],
            &[
                "batch_size",
                "num_heads",
                "past_sequence_length",
                "head_dim",
            ],
        ),
    ];

    let inputs = plan_model_inputs(&metadata, 1, &chunk_inputs()).expect("plan inputs");
    let kv = inputs
        .iter()
        .find(|input| {
            matches!(
                input,
                super::inference::PlannedModelInput::Cache { name, .. } if name == "past_key_values.0.key"
            )
        })
        .expect("kv plan");

    match kv {
        super::inference::PlannedModelInput::Cache { dtype, shape, .. } => {
            assert_eq!(*dtype, TensorElementType::Float64);
            assert_eq!(shape, &vec![1, 8, 0, 128]);
        }
        other => panic!("expected cache input plan, got {other:?}"),
    }
}

#[test]
fn unsupported_cache_dtype_error_includes_contract_details() {
    let metadata = vec![
        metadata(
            "input_ids",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
        metadata(
            "attention_mask",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
        metadata(
            "past_key_values.0.key",
            TensorElementType::Float16,
            &[-1, 8, -1, 128],
            &[
                "batch_size",
                "num_heads",
                "past_sequence_length",
                "head_dim",
            ],
        ),
    ];

    let error =
        build_model_inputs(&metadata, 1, &chunk_inputs()).expect_err("unsupported cache dtype");
    let message = error.to_string();

    assert!(message.contains("declared cache dtype is unsupported for zero-length KV tensors"));
    assert!(message.contains("input=past_key_values.0.key"));
    assert!(message.contains("dtype=f16"));
}

#[test]
fn metadata_driven_kv_zero_axis_selection_prefers_symbols() {
    let metadata = metadata(
        "past_key_values.0.key",
        TensorElementType::Float32,
        &[-1, 8, -1, 128],
        &[
            "batch_size",
            "num_heads",
            "past_sequence_length",
            "head_dim",
        ],
    );

    assert_eq!(select_kv_zero_axis(&metadata).expect("zero axis"), 2);
    assert_eq!(
        build_cache_tensor_shape(&metadata, 2).expect("cache shape"),
        vec![2, 8, 0, 128]
    );
}

#[test]
fn ambiguous_kv_layout_reports_rich_error() {
    let metadata = metadata(
        "past_key_values.0.key",
        TensorElementType::Float32,
        &[-1, -1, -1, 128],
        &["", "", "", ""],
    );

    let error = build_cache_tensor_shape(&metadata, 1).expect_err("ambiguous layout");
    let message = error.to_string();

    assert!(message.contains("missing symbolic cache axis"));
    assert!(message.contains("input=past_key_values.0.key"));
    assert!(message.contains("dtype=f32"));
    assert!(message.contains("shape=[-1, -1, -1, 128]"));
    assert!(message.contains("symbols=[\"\", \"\", \"\", \"\"]"));
}

#[test]
fn bert_regression_emits_token_type_ids_only_when_declared() {
    let without_token_type = vec![
        metadata(
            "input_ids",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
        metadata(
            "attention_mask",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
    ];
    let with_token_type = vec![
        metadata(
            "input_ids",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
        metadata(
            "attention_mask",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
        metadata(
            "token_type_ids",
            TensorElementType::Int64,
            &[-1, -1],
            &["batch", "sequence"],
        ),
    ];

    let without = plan_model_inputs(&without_token_type, 1, &chunk_inputs())
        .expect("bert without token types");
    let with =
        plan_model_inputs(&with_token_type, 1, &chunk_inputs()).expect("bert with token types");

    assert!(!without.iter().any(|input| {
        matches!(
            input,
            super::inference::PlannedModelInput::Integer { name, .. } if name == "token_type_ids"
        )
    }));
    assert!(with.iter().any(|input| {
        matches!(
            input,
            super::inference::PlannedModelInput::Integer { name, .. } if name == "token_type_ids"
        )
    }));
}

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
    let embedding = result.expect("embedding");
    assert!(!embedding.is_empty(), "embedding vector is empty");
    assert!(
        embedding.iter().any(|value| *value != 0.0),
        "embedding is all zeros"
    );
}
