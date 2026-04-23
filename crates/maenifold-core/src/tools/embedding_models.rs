use super::context::{ToolContext, load_help_doc, render};
use super::embedding_models_actions::{handle_activate, handle_download};
use super::embedding_models_detect::{handle_delete, handle_list, handle_regenerate};
use crate::error::Result;
use crate::util::json::{OutputFormat, success};
use crate::vector;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

/// Bundled catalog JSON, embedded at compile time. Used when no external
/// `{assets_path}/models/catalog.json` override is present.
const BUNDLED_CATALOG_JSON: &str = include_str!("../../assets/models/catalog.json");

/// Owned catalog entry deserialized from JSON (or converted from the
/// compile-time `CURATED_MODELS` fallback).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogModel {
    pub id: String,
    pub name: String,
    pub dimensions: usize,
    pub max_tokens: usize,
    pub size_bytes: u64,
    pub license: String,
    pub huggingface_repo: String,
    #[serde(default)]
    pub onnx_repo: Option<String>,
    pub released: String,
    #[serde(default)]
    pub mrl_dimensions: Option<Vec<usize>>,
    #[serde(default)]
    pub mteb_retrieval: Option<f32>,
}

impl CatalogModel {
    fn from_curated(m: &CuratedModel) -> Self {
        Self {
            id: m.id.to_string(),
            name: m.name.to_string(),
            dimensions: m.dimensions,
            max_tokens: m.max_tokens,
            size_bytes: m.size_bytes,
            license: m.license.to_string(),
            huggingface_repo: m.huggingface_repo.to_string(),
            onnx_repo: m.onnx_repo.map(str::to_string),
            released: m.released.to_string(),
            mrl_dimensions: m.mrl_dimensions.map(<[usize]>::to_vec),
            mteb_retrieval: m.mteb_retrieval,
        }
    }
}

/// Load the curated embedding model catalog.
///
/// Resolution order:
/// 1. External JSON at `{assets_path}/models/catalog.json` (user-overridable).
/// 2. Bundled JSON via `include_str!` (default ships with binary).
/// 3. Compile-time `CURATED_MODELS` static array (defensive fallback in case
///    the bundled JSON ever fails to parse — should be unreachable).
pub(crate) fn load_catalog(assets_path: &Path) -> Vec<CatalogModel> {
    let external = assets_path.join("models").join("catalog.json");
    if external.is_file() {
        match std::fs::read_to_string(&external) {
            Ok(text) => match serde_json::from_str::<Vec<CatalogModel>>(&text) {
                Ok(models) => return models,
                Err(e) => tracing::warn!(
                    "Failed to parse {}: {e}; falling back to bundled catalog",
                    external.display()
                ),
            },
            Err(e) => tracing::warn!(
                "Failed to read {}: {e}; falling back to bundled catalog",
                external.display()
            ),
        }
    }

    match serde_json::from_str::<Vec<CatalogModel>>(BUNDLED_CATALOG_JSON) {
        Ok(models) => models,
        Err(e) => {
            tracing::error!(
                "Failed to parse bundled catalog JSON: {e}; falling back to compiled CURATED_MODELS"
            );
            CURATED_MODELS
                .iter()
                .map(CatalogModel::from_curated)
                .collect()
        }
    }
}

/// Curated embedding model catalog entry (compile-time fallback only).
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
    onnx_repo: Option<&'static str>,
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
        onnx_repo: None,
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
        onnx_repo: None,
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
        onnx_repo: None,
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
        onnx_repo: None,
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
        onnx_repo: None,
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
        onnx_repo: None,
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
        onnx_repo: Some("onnx-community/Qwen3-Embedding-0.6B-ONNX"),
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
        onnx_repo: None,
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
            "Missing required parameter 'action'. Available actions: status, list, download, activate, delete, regenerate".to_string()
        ))?;

    let response = match action {
        "status" => handle_status(),
        "list" => handle_list(context),
        "download" => handle_download(context, payload)?,
        "activate" => handle_activate(context, payload)?,
        "delete" => handle_delete(context, payload)?,
        "regenerate" => handle_regenerate(context, payload)?,
        _ => {
            return Err(crate::error::MaenifoldError::Message(format!(
                "Unknown EmbeddingModels action '{action}'. Available: status, list, download, activate, delete, regenerate"
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
