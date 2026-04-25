use crate::error::{MaenifoldError, Result};
use ort::{session::SessionOutputs, value::Shape};

use super::OUTPUT_NAME_PREFERENCES;

#[derive(Debug, Clone)]
pub(crate) struct OutputCandidate {
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) dims: Vec<usize>,
    pub(crate) element_type: OutputElementType,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum OutputElementType {
    Float32,
    Float64,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionOutputCandidate {
    pub(crate) name: String,
    pub(crate) dims: Vec<i64>,
}

pub(crate) fn select_best_output(outputs: &SessionOutputs<'_>) -> Result<OutputCandidate> {
    let mut candidates = Vec::new();
    for (index, (name, output)) in outputs.iter().enumerate() {
        if let Ok((shape, _)) = output.try_extract_tensor::<f32>() {
            candidates.push(OutputCandidate {
                index,
                name: name.to_string(),
                dims: shape_to_usize_vec(shape)?,
                element_type: OutputElementType::Float32,
            });
            continue;
        }

        if let Ok((shape, _)) = output.try_extract_tensor::<f64>() {
            candidates.push(OutputCandidate {
                index,
                name: name.to_string(),
                dims: shape_to_usize_vec(shape)?,
                element_type: OutputElementType::Float64,
            });
        }
    }

    choose_output(
        candidates,
        "embedding model produced no usable outputs",
        |candidate| candidate.name.to_ascii_lowercase(),
        |candidate| candidate.dims.len(),
    )
}

pub(crate) fn select_best_session_output(
    outputs: &[ort::value::Outlet],
) -> Result<SessionOutputCandidate> {
    let candidates = outputs
        .iter()
        .filter_map(|output| {
            let dims = output
                .dtype()
                .tensor_shape()?
                .iter()
                .copied()
                .collect::<Vec<_>>();
            Some(SessionOutputCandidate {
                name: output.name().to_string(),
                dims,
            })
        })
        .collect::<Vec<_>>();

    choose_output(
        candidates,
        "embedding model metadata exposes no tensor outputs",
        |candidate| candidate.name.to_ascii_lowercase(),
        |candidate| candidate.dims.len(),
    )
}

pub(crate) fn extract_output_data(
    outputs: &SessionOutputs<'_>,
    chosen: &OutputCandidate,
) -> Result<(Vec<usize>, Vec<f32>)> {
    let output = &outputs[chosen.index];
    match chosen.element_type {
        OutputElementType::Float32 => {
            let (shape, values) = output.try_extract_tensor::<f32>().map_err(map_ort_error)?;
            Ok((shape_to_usize_vec(shape)?, values.to_vec()))
        }
        OutputElementType::Float64 => {
            let (shape, values) = output.try_extract_tensor::<f64>().map_err(map_ort_error)?;
            Ok((
                shape_to_usize_vec(shape)?,
                values.iter().map(|value| *value as f32).collect(),
            ))
        }
    }
}

pub(crate) fn extract_embedding_from_output(
    dims: &[usize],
    flat_data: &[f32],
    batch_index: usize,
    attention_mask: &[i64],
) -> Vec<f32> {
    match dims {
        [batch, hidden] if batch_index < *batch => {
            let start = batch_index * *hidden;
            let end = start + *hidden;
            flat_data
                .get(start..end)
                .map(|slice| slice.to_vec())
                .unwrap_or_default()
        }
        [batch, seq_len, hidden] if batch_index < *batch => {
            let item_len = seq_len * hidden;
            let start = batch_index * item_len;
            let end = start + item_len;
            flat_data
                .get(start..end)
                .map(|slice| pool_sequence_by_attention(slice, *seq_len, *hidden, attention_mask))
                .unwrap_or_default()
        }
        [hidden] => {
            let mut vec = vec![0.0; *hidden];
            let start = batch_index * *hidden;
            let end = (start + *hidden).min(flat_data.len());
            if let Some(slice) = flat_data.get(start..end) {
                vec[..slice.len()].copy_from_slice(slice);
            }
            vec
        }
        _ => flat_data.to_vec(),
    }
}

pub(crate) fn pad_or_truncate(src: &[f32], desired: usize) -> Vec<f32> {
    let mut dst = vec![0.0; desired];
    let copy_len = src.len().min(desired);
    dst[..copy_len].copy_from_slice(&src[..copy_len]);
    dst
}

pub(crate) fn decode_le_f32_bytes(bytes: &[u8]) -> std::result::Result<Vec<f32>, String> {
    let chunks = bytes.chunks_exact(std::mem::size_of::<f32>());
    if !chunks.remainder().is_empty() {
        return Err("embedding bytes length is not divisible by 4".to_string());
    }

    Ok(chunks
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

pub(crate) fn map_ort_error(error: ort::Error) -> MaenifoldError {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("onnx runtime dylib") || lower.contains("failed to load from") {
        return MaenifoldError::Message(
            "ONNX Runtime library not found. Set ORT_DYLIB_PATH or place the library in the assets/lib directory."
                .to_string(),
        );
    }

    if contains_absolute_path(&message) {
        return MaenifoldError::Message("onnx runtime error: ort_operation_failed".to_string());
    }

    MaenifoldError::Message(format!("onnx runtime error: {message}"))
}

fn contains_absolute_path(message: &str) -> bool {
    message.starts_with('/')
        || message.contains(" /")
        || message.contains("`/")
        || message.contains(":\\")
        || message.contains("`\\\\")
}

fn choose_output<T, FName, FDims>(
    mut candidates: Vec<T>,
    empty_message: &str,
    name: FName,
    dims_len: FDims,
) -> Result<T>
where
    T: Clone,
    FName: Fn(&T) -> String,
    FDims: Fn(&T) -> usize,
{
    if candidates.is_empty() {
        return Err(MaenifoldError::Message(empty_message.to_string()));
    }

    for preferred_name in OUTPUT_NAME_PREFERENCES {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| name(candidate).contains(preferred_name))
        {
            return Ok(candidate.clone());
        }
    }

    for rank in [2, 1, 3] {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| dims_len(candidate) == rank)
        {
            return Ok(candidate.clone());
        }
    }

    Ok(candidates.remove(0))
}

fn pool_sequence_by_attention(
    flattened: &[f32],
    seq_len: usize,
    hidden: usize,
    attention_mask: &[i64],
) -> Vec<f32> {
    let mut pooled = vec![0.0; hidden];
    let mut count = 0usize;

    for token_index in 0..seq_len {
        let use_token = attention_mask.get(token_index).copied().unwrap_or_default();
        if use_token == 0 {
            continue;
        }

        count += 1;
        let base_index = token_index * hidden;
        for hidden_index in 0..hidden {
            pooled[hidden_index] += flattened[base_index + hidden_index];
        }
    }

    if count == 0 {
        return pooled;
    }

    let inverse = 1.0f32 / count as f32;
    for value in &mut pooled {
        *value *= inverse;
    }
    pooled
}

fn shape_to_usize_vec(shape: &Shape) -> Result<Vec<usize>> {
    shape
        .iter()
        .map(|dimension| {
            usize::try_from(*dimension).map_err(|_| {
                MaenifoldError::Message(format!(
                    "negative or unsupported tensor dimension encountered: {dimension}"
                ))
            })
        })
        .collect()
}
