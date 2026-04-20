use super::context::{ToolContext, load_help_doc, render};
use crate::acp_config::{AgentsConfig, MaenifoldSettings};
use crate::error::{MaenifoldError, Result};
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
            CURATED_MODELS.iter().map(CatalogModel::from_curated).collect()
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
            "Missing required parameter 'action'. Available actions: status, list, download, activate, delete".to_string()
        ))?;

    let response = match action {
        "status" => handle_status(),
        "list" => handle_list(context),
        "download" => handle_download(context, payload)?,
        "activate" => handle_activate(context, payload)?,
        "delete" => handle_delete(context, payload)?,
        _ => {
            return Err(crate::error::MaenifoldError::Message(format!(
                "Unknown EmbeddingModels action '{action}'. Available: status, list, download, activate, delete"
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

    let catalog = load_catalog(&context.config.assets_path);
    let models: Vec<Value> = catalog
        .iter()
        .map(|model| {
            let is_active = active_model_path
                .as_deref()
                .map(|p| is_active_model(p, &model.id))
                .unwrap_or(false);
            let installed = is_model_installed(&models_dir, &model.id) || is_active;

            json!({
                "id": model.id,
                "name": model.name,
                "dimensions": model.dimensions,
                "maxTokens": model.max_tokens,
                "sizeBytes": model.size_bytes,
                "license": model.license,
                "huggingfaceRepo": model.huggingface_repo,
                "onnxRepo": model.onnx_repo,
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

fn handle_download(context: &ToolContext, payload: &Value) -> Result<Value> {
    let model_id = payload
        .get("modelId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            crate::error::MaenifoldError::Message(
                "Missing required parameter 'modelId'".to_string(),
            )
        })?;

    let catalog = load_catalog(&context.config.assets_path);
    let entry = catalog
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| {
            crate::error::MaenifoldError::Message(format!("Unknown model: {model_id}"))
        })?;

    let models_dir = context.config.assets_path.join("models");

    let id = model_id.to_string();
    let repo = entry.huggingface_repo.clone();
    let onnx = entry.onnx_repo.clone();
    let dir = models_dir.clone();
    tokio::spawn(async move {
        match super::embedding_download::download_model(&id, &repo, onnx.as_deref(), &dir).await {
            Ok(path) => tracing::info!("Downloaded {id} to {}", path.display()),
            Err(e) => tracing::error!("Download failed for {id}: {e}"),
        }
    });

    Ok(json!({ "accepted": true, "modelId": model_id }))
}

/// Check if a model is installed under the catalog-managed subdirectory layout.
///
/// Only `{models_dir}/{model_id}/*.onnx` counts as "installed" for the catalog.
/// Flat/legacy `{models_dir}/{model_id}.onnx` files are discovered by the
/// runtime scan in `discovery.rs` but are NOT catalog-managed and therefore
/// must not be reported as installed by the catalog tooling — activate/delete
/// only operate on the subdir layout.
fn is_model_installed(models_dir: &Path, model_id: &str) -> bool {
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

/// T-202.9a: persist `active_model` selection into the on-disk maenifold.json.
/// Verifies the model is present in the catalog and installed under
/// `{models_dir}/{model_id}/model.onnx` (subdirectory layout) before writing.
/// Daemon restart + sync is required for the change to take effect.
fn handle_activate(context: &ToolContext, payload: &Value) -> Result<Value> {
    let model_id = payload
        .get("modelId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            MaenifoldError::Message("Missing required parameter 'modelId'".to_string())
        })?;

    if !load_catalog(&context.config.assets_path)
        .iter()
        .any(|m| m.id == model_id)
    {
        return Err(MaenifoldError::Message(format!(
            "Unknown model: {model_id}"
        )));
    }

    let models_dir = context.config.assets_path.join("models");
    let installed_path = models_dir.join(model_id).join("model.onnx");
    let installed = installed_path
        .metadata()
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false);
    if !installed {
        return Err(MaenifoldError::Message(format!(
            "Model '{model_id}' is not installed at {} — download it first.",
            installed_path.display()
        )));
    }

    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root);
    let mut settings = config.settings.unwrap_or_else(MaenifoldSettings::default);
    settings.vector.active_model = Some(model_id.to_string());
    config.settings = Some(settings);
    write_agents_config(root, &config)?;

    Ok(json!({
        "activated": true,
        "modelId": model_id,
        "reembedRequired": true,
        "message": "Restart daemon to apply. All vectors will be rebuilt on next sync.",
    }))
}

/// T-202.9a: delete an installed model directory from disk. Refuses to delete
/// the currently-active model to avoid leaving the daemon without a usable
/// model on next restart.
fn handle_delete(context: &ToolContext, payload: &Value) -> Result<Value> {
    let model_id = payload
        .get("modelId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            MaenifoldError::Message("Missing required parameter 'modelId'".to_string())
        })?;

    let active_model = context.config.settings.vector.active_model.as_deref();
    if active_model == Some(model_id) {
        return Err(MaenifoldError::Message(format!(
            "Refusing to delete '{model_id}': it is the currently-active model. Activate a different model first."
        )));
    }

    let models_dir = context.config.assets_path.join("models");
    let target_dir = models_dir.join(model_id);

    // T-202.9 remediation: when no active_model is configured, the runtime
    // falls back to scanning (`discovery.rs::discover_model`) and picks the
    // largest *.onnx — meaning any installed model could become the active
    // one on next restart. Refuse to delete in that ambiguous state and
    // require the user to explicitly pin an active model first.
    if active_model.is_none() && target_dir.is_dir() {
        return Err(MaenifoldError::Message(
            "Cannot delete while no active model is configured. Activate a different model first.".to_string(),
        ));
    }

    if !target_dir.exists() {
        return Err(MaenifoldError::Message(format!(
            "Model directory not found: {}",
            target_dir.display()
        )));
    }

    std::fs::remove_dir_all(&target_dir).map_err(|e| {
        MaenifoldError::Message(format!(
            "Failed to delete '{}': {}",
            target_dir.display(),
            e
        ))
    })?;

    Ok(json!({
        "deleted": true,
        "modelId": model_id,
    }))
}

/// T-202.9a: write the agents config to `{root}/maenifold.json`.
/// Mirrors the formatting/path semantics of `api::config_manager::save_config`
/// without taking an HTTP-layer dependency.
fn write_agents_config(root: &Path, config: &AgentsConfig) -> Result<()> {
    let path = root.join("maenifold.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            MaenifoldError::Message(format!(
                "Failed to ensure config directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| MaenifoldError::Message(format!("Failed to serialize config: {e}")))?;
    std::fs::write(&path, content).map_err(|e| {
        MaenifoldError::Message(format!("Failed to write '{}': {}", path.display(), e))
    })
}
