use super::context::{ToolContext, load_help_doc, render};
use crate::error::Result;
use crate::util::json::{OutputFormat, success};
use crate::vector;
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;

/// Curated embedding model catalog entry
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CuratedModel {
    id: &'static str,
    name: &'static str,
    dimensions: usize,
    max_tokens: usize,
    size_bytes: u64,
    license: &'static str,
    huggingface_repo: &'static str,
    released: &'static str,
    mrl_dimensions: Option<&'static [usize]>,
    mteb_retrieval: Option<f32>,
}

/// Curated embedding model catalog.
///
/// Selection criteria:
/// - ONNX Runtime compatible (verified on Apple Silicon)
/// - Permissive license (Apache-2.0, MIT) — CC-BY-NC flagged
/// - Spectrum: tiny/fast → balanced → high-quality → long-context
/// - Matryoshka (MRL) models preferred for runtime dimension flexibility
/// - MTEB retrieval scores from https://huggingface.co/spaces/mteb/leaderboard (April 2026)
const CURATED_MODELS: &[CuratedModel] = &[
    CuratedModel {
        id: "all-MiniLM-L6-v2",
        name: "all-MiniLM-L6-v2",
        dimensions: 384,
        max_tokens: 256,
        size_bytes: 90_405_214,
        license: "Apache-2.0",
        huggingface_repo: "sentence-transformers/all-MiniLM-L6-v2",
        released: "2021-08",
        mrl_dimensions: None,
        mteb_retrieval: Some(41.9),
    },
    CuratedModel {
        id: "bge-small-en-v1.5",
        name: "BGE Small English v1.5",
        dimensions: 384,
        max_tokens: 512,
        size_bytes: 67_000_000,
        license: "MIT",
        huggingface_repo: "BAAI/bge-small-en-v1.5",
        released: "2023-09",
        mrl_dimensions: None,
        mteb_retrieval: Some(51.7),
    },
    CuratedModel {
        id: "snowflake-arctic-embed-m-v2.0",
        name: "Snowflake Arctic Embed M v2.0",
        dimensions: 768,
        max_tokens: 8192,
        size_bytes: 610_000_000,
        license: "Apache-2.0",
        huggingface_repo: "Snowflake/snowflake-arctic-embed-m-v2.0",
        released: "2024-12",
        mrl_dimensions: Some(&[256, 512, 768]),
        mteb_retrieval: Some(55.4),
    },
    CuratedModel {
        id: "mxbai-embed-large-v1",
        name: "mxbai Embed Large v1",
        dimensions: 1024,
        max_tokens: 512,
        size_bytes: 670_000_000,
        license: "Apache-2.0",
        huggingface_repo: "mixedbread-ai/mxbai-embed-large-v1",
        released: "2024-03",
        mrl_dimensions: Some(&[256, 512, 768, 1024]),
        mteb_retrieval: Some(54.4),
    },
    CuratedModel {
        id: "nomic-embed-text-v1.5",
        name: "Nomic Embed Text v1.5",
        dimensions: 768,
        max_tokens: 8192,
        size_bytes: 200_000_000,
        license: "Apache-2.0",
        huggingface_repo: "nomic-ai/nomic-embed-text-v1.5",
        released: "2024-02",
        mrl_dimensions: Some(&[64, 128, 256, 512, 768]),
        mteb_retrieval: Some(53.0),
    },
    CuratedModel {
        id: "jina-embeddings-v3",
        name: "Jina Embeddings v3",
        dimensions: 1024,
        max_tokens: 8192,
        size_bytes: 1_100_000_000,
        license: "CC-BY-NC-4.0",
        huggingface_repo: "jinaai/jina-embeddings-v3",
        released: "2024-09",
        mrl_dimensions: Some(&[32, 64, 128, 256, 512, 768, 1024]),
        mteb_retrieval: Some(55.8),
    },
    CuratedModel {
        id: "qwen3-embedding-0.6b",
        name: "Qwen3 Embedding 0.6B",
        dimensions: 1024,
        max_tokens: 32768,
        size_bytes: 1_200_000_000,
        license: "Apache-2.0",
        huggingface_repo: "Qwen/Qwen3-Embedding-0.6B",
        released: "2025-06",
        mrl_dimensions: Some(&[32, 64, 128, 256, 512, 768, 1024]),
        mteb_retrieval: Some(64.3),
    },
    CuratedModel {
        id: "bge-large-en-v1.5",
        name: "BGE Large English v1.5",
        dimensions: 1024,
        max_tokens: 512,
        size_bytes: 400_000_000,
        license: "MIT",
        huggingface_repo: "BAAI/bge-large-en-v1.5",
        released: "2023-09",
        mrl_dimensions: None,
        mteb_retrieval: Some(54.3),
    },
];

pub fn embedding_models(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "EmbeddingModels")
            .unwrap_or_else(|| "ERROR: Help file not found for EmbeddingModels".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let action = payload
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| crate::error::MaenifoldError::Message(
            "Missing required parameter 'action'. Available actions: status, list".to_string()
        ))?;

    let response = match action {
        "status" => handle_status(),
        "list" => handle_list(context),
        _ => {
            return Err(crate::error::MaenifoldError::Message(format!(
                "Unknown EmbeddingModels action '{action}'. Available: status, list"
            )));
        }
    };

    Ok(render(context.output_format, response))
}

fn handle_status() -> Value {
    let loaded = vector::is_initialized();
    if !loaded {
        return json!({ "loaded": false });
    }

    json!({
        "loaded": true,
        "model": {
            "filename": vector::model_filename().unwrap_or_default(),
            "dimensions": vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS),
            "maxTokens": vector::max_tokens().unwrap_or(vector::DEFAULT_MAX_TOKENS),
        }
    })
}

fn handle_list(context: &ToolContext) -> Value {
    let models_dir = context.config.assets_path.join("models");
    let active_model_path = vector::model_path();

    let models: Vec<Value> = CURATED_MODELS
        .iter()
        .map(|model| {
            let is_active = active_model_path
                .as_deref()
                .map(|p| is_active_model(p, model.id))
                .unwrap_or(false);
            let installed = is_model_installed(&models_dir, model.id) || is_active;

            json!({
                "id": model.id,
                "name": model.name,
                "dimensions": model.dimensions,
                "maxTokens": model.max_tokens,
                "sizeBytes": model.size_bytes,
                "license": model.license,
                "huggingfaceRepo": model.huggingface_repo,
                "released": model.released,
                "mrlDimensions": model.mrl_dimensions,
                "mtebRetrieval": model.mteb_retrieval,
                "installed": installed,
                "isActive": is_active,
            })
        })
        .collect();

    let current_dims = vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS);

    json!({
        "models": models,
        "currentDimensions": current_dims,
    })
}

/// Check if a model is installed by looking for ONNX files in both flat and subdirectory layouts.
fn is_model_installed(models_dir: &Path, model_id: &str) -> bool {
    // Flat layout: {models_dir}/{model_id}.onnx
    let flat_path = models_dir.join(format!("{model_id}.onnx"));
    if flat_path.is_file() && flat_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return true;
    }

    // Subdirectory layout: {models_dir}/{model_id}/*.onnx
    let sub_dir = models_dir.join(model_id);
    if sub_dir.is_dir() && let Ok(entries) = std::fs::read_dir(&sub_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("onnx")
                && path.is_file()
                && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
            {
                return true;
            }
        }
    }

    false
}

/// Check if a loaded model path matches a catalog model ID.
fn is_active_model(model_path: &Path, model_id: &str) -> bool {
    let filename = model_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    let without_ext = filename.strip_suffix(".onnx").unwrap_or(filename);

    // Direct match: "all-MiniLM-L6-v2.onnx" matches "all-MiniLM-L6-v2"
    if without_ext == model_id {
        return true;
    }

    // Subdirectory match: "models/bge-small-en-v1.5/model.onnx"
    // Check if parent directory name matches model_id
    if filename == "model.onnx" {
        if let Some(parent) = model_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
        {
            return parent == model_id;
        }
    }

    false
}
