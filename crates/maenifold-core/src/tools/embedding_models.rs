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
}

const CURATED_MODELS: &[CuratedModel] = &[
    CuratedModel {
        id: "all-MiniLM-L6-v2",
        name: "all-MiniLM-L6-v2",
        dimensions: 384,
        max_tokens: 256,
        size_bytes: 90_405_214,
        license: "Apache-2.0",
        huggingface_repo: "sentence-transformers/all-MiniLM-L6-v2",
    },
    CuratedModel {
        id: "bge-small-en-v1.5",
        name: "BGE Small English v1.5",
        dimensions: 384,
        max_tokens: 512,
        size_bytes: 67_000_000,
        license: "MIT",
        huggingface_repo: "BAAI/bge-small-en-v1.5",
    },
    CuratedModel {
        id: "bge-base-en-v1.5",
        name: "BGE Base English v1.5",
        dimensions: 768,
        max_tokens: 512,
        size_bytes: 220_000_000,
        license: "MIT",
        huggingface_repo: "BAAI/bge-base-en-v1.5",
    },
    CuratedModel {
        id: "bge-large-en-v1.5",
        name: "BGE Large English v1.5",
        dimensions: 1024,
        max_tokens: 512,
        size_bytes: 400_000_000,
        license: "MIT",
        huggingface_repo: "BAAI/bge-large-en-v1.5",
    },
    CuratedModel {
        id: "snowflake-arctic-embed-m",
        name: "Snowflake Arctic Embed M",
        dimensions: 768,
        max_tokens: 512,
        size_bytes: 240_000_000,
        license: "Apache-2.0",
        huggingface_repo: "Snowflake/snowflake-arctic-embed-m",
    },
    CuratedModel {
        id: "nomic-embed-text-v1.5",
        name: "Nomic Embed Text v1.5",
        dimensions: 768,
        max_tokens: 8192,
        size_bytes: 200_000_000,
        license: "Apache-2.0",
        huggingface_repo: "nomic-ai/nomic-embed-text-v1.5",
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
