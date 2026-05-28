use super::super::{ModelInputMetadata, input_metadata_by_name};
use crate::error::{MaenifoldError, Result};
use ort::value::{DynTensor, Tensor, TensorElementType};
use std::fmt::{Debug, Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedRow {
    pub(crate) input_ids: Vec<u32>,
    pub(crate) attention_mask: Vec<u32>,
    pub(crate) token_type_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChunkTokenInputs {
    pub(crate) seq_len: usize,
    pub(crate) input_ids: Vec<i64>,
    pub(crate) attention_mask: Vec<i64>,
    pub(crate) token_type_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PastKeyValueInput {
    pub(crate) layer: usize,
    pub(crate) kind: String,
    pub(crate) metadata: ModelInputMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlannedModelInput {
    Integer {
        name: String,
        dtype: TensorElementType,
        shape: [usize; 2],
        values: Vec<i64>,
    },
    Cache {
        name: String,
        dtype: TensorElementType,
        shape: Vec<usize>,
    },
}

pub(crate) fn build_chunk_token_inputs(
    rows: &[EncodedRow],
    pad_id: u32,
    truncation_ceiling: usize,
) -> ChunkTokenInputs {
    let seq_len = rows
        .iter()
        .map(|row| row.input_ids.len().min(truncation_ceiling))
        .max()
        .unwrap_or(0)
        .max(1);
    let batch_size = rows.len();
    let mut input_ids = vec![i64::from(pad_id); batch_size * seq_len];
    let mut attention_mask = vec![0_i64; batch_size * seq_len];
    let mut token_type_ids = vec![0_i64; batch_size * seq_len];

    for (batch_index, row) in rows.iter().enumerate() {
        let row_offset = batch_index * seq_len;
        let max_copy = row.input_ids.len().min(seq_len);
        for token_index in 0..max_copy {
            input_ids[row_offset + token_index] = i64::from(row.input_ids[token_index]);
            attention_mask[row_offset + token_index] = i64::from(row.attention_mask[token_index]);
            token_type_ids[row_offset + token_index] = i64::from(row.token_type_ids[token_index]);
        }
    }

    ChunkTokenInputs {
        seq_len,
        input_ids,
        attention_mask,
        token_type_ids,
    }
}

pub(crate) fn build_model_inputs(
    metadata: &[ModelInputMetadata],
    batch_size: usize,
    chunk_inputs: &ChunkTokenInputs,
) -> Result<Vec<(String, DynTensor)>> {
    plan_model_inputs(metadata, batch_size, chunk_inputs)?
        .into_iter()
        .map(materialize_planned_input)
        .collect()
}

pub(crate) fn plan_model_inputs(
    metadata: &[ModelInputMetadata],
    batch_size: usize,
    chunk_inputs: &ChunkTokenInputs,
) -> Result<Vec<PlannedModelInput>> {
    let shape = [batch_size, chunk_inputs.seq_len];
    let input_ids = required_input(metadata, "input_ids")?;
    let attention_mask = required_input(metadata, "attention_mask")?;
    validate_integer_dtype(input_ids)?;
    validate_integer_dtype(attention_mask)?;
    let mut inputs = vec![
        PlannedModelInput::Integer {
            name: input_ids.name.clone(),
            dtype: input_ids.dtype,
            shape,
            values: chunk_inputs.input_ids.clone(),
        },
        PlannedModelInput::Integer {
            name: attention_mask.name.clone(),
            dtype: attention_mask.dtype,
            shape,
            values: chunk_inputs.attention_mask.clone(),
        },
    ];

    if let Some(token_type_ids) = input_metadata_by_name(metadata, "token_type_ids") {
        validate_integer_dtype(token_type_ids)?;
        inputs.push(PlannedModelInput::Integer {
            name: token_type_ids.name.clone(),
            dtype: token_type_ids.dtype,
            shape,
            values: chunk_inputs.token_type_ids.clone(),
        });
    }

    if let Some(position_ids) = input_metadata_by_name(metadata, "position_ids") {
        validate_integer_dtype(position_ids)?;
        let values = build_position_ids_from_attention_mask(
            &chunk_inputs.attention_mask,
            batch_size,
            chunk_inputs.seq_len,
        );
        inputs.push(PlannedModelInput::Integer {
            name: position_ids.name.clone(),
            dtype: position_ids.dtype,
            shape,
            values,
        });
    }

    for kv_input in parse_past_key_value_inputs(metadata) {
        validate_cache_dtype(&kv_input.metadata)?;
        inputs.push(PlannedModelInput::Cache {
            name: kv_input.metadata.name.clone(),
            dtype: kv_input.metadata.dtype,
            shape: build_cache_tensor_shape(&kv_input.metadata, batch_size)?,
        });
    }

    Ok(inputs)
}

pub(crate) fn parse_past_key_value_inputs(
    metadata: &[ModelInputMetadata],
) -> Vec<PastKeyValueInput> {
    let mut inputs = metadata
        .iter()
        .filter_map(|input| {
            let suffix = input.name.strip_prefix("past_key_values.")?;
            let (layer, kind) = suffix.split_once('.')?;
            let layer = layer.parse::<usize>().ok()?;
            (kind == "key" || kind == "value").then(|| PastKeyValueInput {
                layer,
                kind: kind.to_string(),
                metadata: input.clone(),
            })
        })
        .collect::<Vec<_>>();
    inputs.sort_by(|left, right| {
        left.layer
            .cmp(&right.layer)
            .then(left.kind.cmp(&right.kind))
            .then(left.metadata.name.cmp(&right.metadata.name))
    });
    inputs
}

pub(crate) fn build_position_ids_from_attention_mask(
    attention_mask: &[i64],
    batch_size: usize,
    seq_len: usize,
) -> Vec<i64> {
    let mut position_ids = vec![0_i64; batch_size * seq_len];
    for batch_index in 0..batch_size {
        let row_offset = batch_index * seq_len;
        let row = &attention_mask[row_offset..row_offset + seq_len];
        let mut next_position = 0_i64;
        for (token_index, attended) in row.iter().copied().enumerate() {
            if attended > 0 {
                position_ids[row_offset + token_index] = next_position;
                next_position += 1;
            }
        }
    }
    position_ids
}

fn required_input<'a>(
    metadata: &'a [ModelInputMetadata],
    name: &str,
) -> Result<&'a ModelInputMetadata> {
    input_metadata_by_name(metadata, name).ok_or_else(|| {
        MaenifoldError::Message(format!(
            "embedding model is missing required input `{name}`"
        ))
    })
}

fn build_integer_tensor(
    metadata: &ModelInputMetadata,
    shape: [usize; 2],
    values: &[i64],
) -> Result<DynTensor> {
    validate_integer_dtype(metadata)?;
    match metadata.dtype {
        TensorElementType::Int8 => build_integer_tensor_as::<i8>(metadata, shape, values),
        TensorElementType::Uint8 => build_integer_tensor_as::<u8>(metadata, shape, values),
        TensorElementType::Int16 => build_integer_tensor_as::<i16>(metadata, shape, values),
        TensorElementType::Uint16 => build_integer_tensor_as::<u16>(metadata, shape, values),
        TensorElementType::Int32 => build_integer_tensor_as::<i32>(metadata, shape, values),
        TensorElementType::Uint32 => build_integer_tensor_as::<u32>(metadata, shape, values),
        TensorElementType::Int64 => build_integer_tensor_as::<i64>(metadata, shape, values),
        TensorElementType::Uint64 => build_integer_tensor_as::<u64>(metadata, shape, values),
        _ => Err(unsupported_contract_error(
            metadata,
            "declared integer dtype is unsupported for token input".to_string(),
        )),
    }
}

fn build_integer_tensor_as<T>(
    metadata: &ModelInputMetadata,
    shape: [usize; 2],
    values: &[i64],
) -> Result<DynTensor>
where
    T: TryFrom<i64> + ort::value::PrimitiveTensorElementType + Debug + Clone + 'static,
    <T as TryFrom<i64>>::Error: Display,
{
    let cast = values
        .iter()
        .copied()
        .map(|value| {
            T::try_from(value).map_err(|error| {
                MaenifoldError::Message(format!(
                    "embedding input value {value} is out of range for {}: {error}; {}",
                    metadata.dtype,
                    metadata.describe_contract()
                ))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Tensor::from_array((shape, cast.into_boxed_slice()))
        .map(|tensor| tensor.upcast())
        .map_err(super::map_ort_error)
}

fn materialize_planned_input(input: PlannedModelInput) -> Result<(String, DynTensor)> {
    match input {
        PlannedModelInput::Integer {
            name,
            dtype,
            shape,
            values,
        } => {
            let metadata = ModelInputMetadata {
                name: name.clone(),
                dtype,
                dims: shape.iter().map(|dim| *dim as i64).collect(),
                dimension_symbols: Vec::new(),
            };
            let tensor = build_integer_tensor(&metadata, shape, &values)?;
            Ok((name, tensor))
        }
        PlannedModelInput::Cache { name, dtype, shape } => {
            let metadata = ModelInputMetadata {
                name: name.clone(),
                dtype,
                dims: shape.iter().map(|dim| *dim as i64).collect(),
                dimension_symbols: Vec::new(),
            };
            let tensor = match dtype {
                TensorElementType::Float32 => {
                    Tensor::from_array((shape, Vec::<f32>::new().into_boxed_slice()))
                        .map(|tensor| tensor.upcast())
                        .map_err(super::map_ort_error)?
                }
                TensorElementType::Float64 => {
                    Tensor::from_array((shape, Vec::<f64>::new().into_boxed_slice()))
                        .map(|tensor| tensor.upcast())
                        .map_err(super::map_ort_error)?
                }
                _ => {
                    return Err(unsupported_contract_error(
                        &metadata,
                        "declared cache dtype is unsupported for zero-length KV tensors"
                            .to_string(),
                    ));
                }
            };
            Ok((name, tensor))
        }
    }
}

fn validate_integer_dtype(metadata: &ModelInputMetadata) -> Result<()> {
    match metadata.dtype {
        TensorElementType::Int8
        | TensorElementType::Uint8
        | TensorElementType::Int16
        | TensorElementType::Uint16
        | TensorElementType::Int32
        | TensorElementType::Uint32
        | TensorElementType::Int64
        | TensorElementType::Uint64 => Ok(()),
        _ => Err(unsupported_contract_error(
            metadata,
            "declared integer dtype is unsupported for token input".to_string(),
        )),
    }
}

fn validate_cache_dtype(metadata: &ModelInputMetadata) -> Result<()> {
    match metadata.dtype {
        TensorElementType::Float32 | TensorElementType::Float64 => Ok(()),
        _ => Err(unsupported_contract_error(
            metadata,
            "declared cache dtype is unsupported for zero-length KV tensors".to_string(),
        )),
    }
}

pub(crate) fn build_cache_tensor_shape(
    metadata: &ModelInputMetadata,
    batch_size: usize,
) -> Result<Vec<usize>> {
    let zero_axis = select_kv_zero_axis(metadata)?;
    let allow_fallback = metadata
        .dimension_symbols
        .iter()
        .all(|symbol| symbol.trim().is_empty());
    let is_qwen_fallback = allow_fallback && matches_qwen_kv_fallback(metadata);

    metadata
        .dims
        .iter()
        .enumerate()
        .map(|(index, dim)| {
            if index == zero_axis {
                Ok(0)
            } else if *dim >= 0 {
                usize::try_from(*dim).map_err(|error| {
                    MaenifoldError::Message(format!(
                        "embedding cache dimension conversion failed for {} at axis {index}: {error}; {}",
                        metadata.name,
                        metadata.describe_contract()
                    ))
                })
            } else if metadata
                .dimension_symbols
                .get(index)
                .is_some_and(|symbol| is_batch_symbol(symbol))
            {
                Ok(batch_size)
            } else if is_qwen_fallback && index == 0 {
                Ok(batch_size)
            } else {
                Err(ambiguous_contract_error(
                    metadata,
                    format!(
                        "cannot resolve non-zero cache dimension at axis {index} for zero-length KV tensor"
                    ),
                ))
            }
        })
        .collect()
}

pub(crate) fn select_kv_zero_axis(metadata: &ModelInputMetadata) -> Result<usize> {
    let candidates = metadata
        .dimension_symbols
        .iter()
        .enumerate()
        .filter_map(|(index, symbol)| is_cache_sequence_symbol(symbol).then_some(index))
        .collect::<Vec<_>>();

    if candidates.len() == 1 {
        return Ok(candidates[0]);
    }
    if !metadata
        .dimension_symbols
        .iter()
        .all(|symbol| symbol.trim().is_empty())
    {
        return Err(ambiguous_contract_error(
            metadata,
            format!(
                "unable to identify a unique cache sequence axis from symbols {:?}",
                metadata.dimension_symbols
            ),
        ));
    }
    if matches_qwen_kv_fallback(metadata) {
        return Ok(2);
    }

    Err(ambiguous_contract_error(
        metadata,
        "missing symbolic cache axis and no verified qwen rank-4 fallback matched".to_string(),
    ))
}

fn matches_qwen_kv_fallback(metadata: &ModelInputMetadata) -> bool {
    metadata.dims.len() == 4
        && metadata.dims.first().is_some_and(|dim| *dim < 0)
        && metadata.dims.get(1).is_some_and(|dim| *dim > 0)
        && metadata.dims.get(2).is_some_and(|dim| *dim < 0)
        && metadata.dims.get(3).is_some_and(|dim| *dim > 0)
}

fn is_cache_sequence_symbol(symbol: &str) -> bool {
    let symbol = normalize_symbol(symbol);
    !symbol.is_empty()
        && !symbol.contains("batch")
        && !symbol.contains("head")
        && (symbol.contains("past_sequence")
            || symbol.contains("cache_sequence")
            || symbol.contains("sequence_length")
            || symbol.contains("seq_len"))
}

fn is_batch_symbol(symbol: &str) -> bool {
    normalize_symbol(symbol).contains("batch")
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn unsupported_contract_error(metadata: &ModelInputMetadata, reason: String) -> MaenifoldError {
    MaenifoldError::Message(format!("{reason}; {}", metadata.describe_contract()))
}

fn ambiguous_contract_error(metadata: &ModelInputMetadata, reason: String) -> MaenifoldError {
    MaenifoldError::Message(format!("{reason}; {}", metadata.describe_contract()))
}
