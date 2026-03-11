use super::discovery::load_model;
use super::output::{
    decode_le_f32_bytes, extract_embedding_from_output, extract_output_data, map_ort_error,
    pad_or_truncate, select_best_output,
};
use super::{DEFAULT_MAX_BATCH, model_state, zero_embedding_vector};
use crate::error::{MaenifoldError, Result};
use base64::Engine as _;
use ort::value::Tensor;

/// Generate embedding for a single text
pub fn generate_embedding(text: &str) -> Result<Vec<f32>> {
    generate_embeddings(&[text], DEFAULT_MAX_BATCH).map(|mut embeddings| {
        embeddings
            .drain(..)
            .next()
            .unwrap_or_else(zero_embedding_vector)
    })
}

/// Generate embeddings for multiple texts in batched ONNX inference (max_batch=32 default)
pub fn generate_embeddings(texts: &[&str], max_batch: usize) -> Result<Vec<Vec<f32>>> {
    // T-9.1: RTM FR-7 — keep batched [[embedding]] inference behavior identical while
    // moving execution code behind smaller vector submodules.
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let non_empty = texts
        .iter()
        .enumerate()
        .filter_map(|(index, text)| {
            if text.trim().is_empty() {
                None
            } else {
                Some((index, text.chars().take(10_000).collect::<String>()))
            }
        })
        .collect::<Vec<_>>();

    if non_empty.is_empty() {
        return Ok(vec![zero_embedding_vector(); texts.len()]);
    }

    load_model()?;

    let effective_batch = if max_batch == 0 {
        DEFAULT_MAX_BATCH
    } else {
        max_batch
    };

    let mut guard = model_state()
        .lock()
        .map_err(|_| MaenifoldError::Message("embedding model mutex poisoned".to_string()))?;
    let state = guard.as_mut().ok_or_else(|| {
        MaenifoldError::Message("embedding model session was not initialized".to_string())
    })?;
    let mut results = vec![vec![0.0; state.embedding_dims]; texts.len()];

    for chunk in non_empty.chunks(effective_batch) {
        let batch_size = chunk.len();
        let max_tokens = state.max_tokens;

        let mut input_ids = vec![i64::from(state.tokenizer.pad_id); batch_size * max_tokens];
        let mut attention_mask = vec![0_i64; batch_size * max_tokens];
        let mut token_type_ids = vec![0_i64; batch_size * max_tokens];

        for (batch_index, (_, text)) in chunk.iter().enumerate() {
            let encoding = state
                .tokenizer
                .tokenizer
                .encode(text.as_str(), true)
                .map_err(|error| {
                    MaenifoldError::Message(format!("tokenizer encode failed: {error}"))
                })?;

            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let types = encoding.get_type_ids();
            let max_copy = ids.len().min(max_tokens);
            let row_offset = batch_index * max_tokens;

            for token_index in 0..max_copy {
                input_ids[row_offset + token_index] = i64::from(ids[token_index]);
                attention_mask[row_offset + token_index] = i64::from(mask[token_index]);
                token_type_ids[row_offset + token_index] = i64::from(types[token_index]);
            }
        }

        let outputs = state
            .session
            .run(ort::inputs! {
                "input_ids" => Tensor::from_array(([batch_size, max_tokens], input_ids.clone().into_boxed_slice())).map_err(map_ort_error)?,
                "attention_mask" => Tensor::from_array(([batch_size, max_tokens], attention_mask.clone().into_boxed_slice())).map_err(map_ort_error)?,
                "token_type_ids" => Tensor::from_array(([batch_size, max_tokens], token_type_ids.into_boxed_slice())).map_err(map_ort_error)?,
            })
            .map_err(map_ort_error)?;

        let chosen = select_best_output(&outputs)?;
        let (dims, flat_data) = extract_output_data(&outputs, &chosen)?;

        for (batch_index, (result_index, _)) in chunk.iter().enumerate() {
            let range = (batch_index * max_tokens)..((batch_index + 1) * max_tokens);
            let embedding = extract_embedding_from_output(
                &dims,
                &flat_data,
                batch_index,
                &attention_mask[range],
            );
            results[*result_index] = pad_or_truncate(&embedding, state.embedding_dims);
        }
    }

    Ok(results)
}

/// Encode embedding as base64 (for transport)
pub fn generate_embedding_base64(text: &str) -> Result<String> {
    let embedding = generate_embedding(text)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(to_sqlite_vector_blob(&embedding)))
}

/// Decode base64 embedding back to float array
pub fn decode_base64_embedding(base64: &str) -> Vec<f32> {
    if base64.trim().is_empty() {
        return zero_embedding_vector();
    }

    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(base64) else {
        return zero_embedding_vector();
    };

    decode_le_f32_bytes(&bytes).unwrap_or_else(|_| zero_embedding_vector())
}

/// Convert float array to SQLite blob format (little-endian f32 bytes)
pub fn to_sqlite_vector_blob(embedding: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(std::mem::size_of_val(embedding));
    for value in embedding {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}
