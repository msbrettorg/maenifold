use super::context::{ToolContext, load_help_doc, render};
use crate::acp_config::{AgentsConfig, MaenifoldSettings};
use crate::db;
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

fn handle_list(context: &ToolContext) -> Value {
    let models_dir = context.config.assets_path.join("models");

    let catalog = load_catalog(&context.config.assets_path);
    // T-202.9 P0 (Bug 3): determine the effective active model id once, so the
    // UI can render the "Active" badge regardless of layout (subdir/flat) and
    // regardless of whether the ONNX session has been lazily loaded yet.
    let active_id = effective_active_model_id(context, &catalog);

    let models: Vec<Value> = catalog
        .iter()
        .map(|model| {
            let layout = installed_layout(&models_dir, &model.id);
            let is_active = active_id.as_deref() == Some(model.id.as_str());
            // T-202.9 P0 (Bug 1): a legacy flat install ({models_dir}/{id}.onnx)
            // counts as installed even though catalog tooling normally manages
            // only the subdir layout. If the model is the currently-active one
            // (e.g. discovered by runtime scan), surface it as installed too so
            // the UI doesn't show "Download" next to a model that's already in
            // use.
            let installed = layout.is_some() || is_active;

            // T-202.39: surface in-flight download progress so the desktop
            // UI can render byte counts beside the "Downloading…" button.
            let download_progress =
                super::embedding_download::get_download_progress(&model.id).map(|p| {
                    json!({
                        "current": p.current_bytes,
                        "total": p.total_bytes,
                        "filename": p.filename,
                    })
                });

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
                "installedLayout": layout,
                "isActive": is_active,
                "downloadProgress": download_progress,
            })
        })
        .collect();

    let current_dims = vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS);

    json!({
        "models": models,
        "currentDimensions": current_dims,
        "activeModelId": active_id,
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

    // T-202.37 P0: previously this used `tokio::spawn` fire-and-forget, which
    // caused the UI to show "Downloading…" indefinitely because (1) the spawned
    // task could be dropped when the MCP tool dispatch returned, and (2) any
    // download error was only emitted to tracing and never surfaced to the
    // caller. Run the download synchronously from the (sync) tool handler so
    // success/failure flows back to the MCP response and the frontend's poll
    // observes installed=true (or surfaces the error in its catch block).
    let download_future = async move {
        super::embedding_download::download_model(&id, &repo, onnx.as_deref(), &dir).await
    };

    let result = match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            if !matches!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ) {
                return Err(MaenifoldError::Message(
                    "Embedding model download requires a multi-thread Tokio runtime when invoked from async context".to_string(),
                ));
            }
            tokio::task::block_in_place(|| handle.block_on(download_future))
        }
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    MaenifoldError::Message(format!(
                        "Failed to build runtime for embedding model download: {e}"
                    ))
                })?;
            rt.block_on(download_future)
        }
    };

    match result {
        Ok(path) => {
            tracing::info!("Downloaded {model_id} to {}", path.display());
            Ok(json!({
                "downloaded": true,
                "modelId": model_id,
                "path": path.display().to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Download failed for {model_id}: {e}");
            Err(MaenifoldError::Message(format!(
                "Download failed for {model_id}: {e}"
            )))
        }
    }
}

/// T-202.9 P0 (Bug 1): detect whether a model is installed and which on-disk
/// layout it uses.
///
/// - `Some("subdir")` — `{models_dir}/{model_id}/*.onnx` (catalog-managed
///   layout used by the download flow).
/// - `Some("flat")` — `{models_dir}/{model_id}.onnx` (legacy layout that the
///   runtime still discovers via `discovery.rs::discover_model`).
/// - `None` — no installed ONNX file found in either layout.
///
/// `activate`/`delete` continue to operate only on the subdir layout (see
/// their respective handlers); this helper exists so the catalog `list`
/// response can accurately report installation state for legacy installs that
/// predate the subdir convention.
fn installed_layout(models_dir: &Path, model_id: &str) -> Option<&'static str> {
    let sub_dir = models_dir.join(model_id);
    if sub_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&sub_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("onnx")
                && path.is_file()
                && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
            {
                return Some("subdir");
            }
        }
    }

    let flat = models_dir.join(format!("{model_id}.onnx"));
    if let Ok(meta) = flat.metadata()
        && meta.is_file()
        && meta.len() > 0
    {
        return Some("flat");
    }

    None
}

/// T-202.9 P0 (Bug 3): determine the catalog id of the model that is (or would
/// become) active. Resolution order mirrors `discovery.rs::find_model_path`:
///
/// 1. If an ONNX session is already loaded, derive the id from its on-disk
///    path (works for both subdir and flat layouts).
/// 2. Otherwise, honor an explicit `settings.vector.active_model` pin.
/// 3. Otherwise, replicate `discover_model`: pick the largest installed
///    `*.onnx` (flat layout) under `{assets_path}/models` and strip the
///    extension. Subdir-only installs without an explicit pin remain
///    ambiguous and return `None`.
fn effective_active_model_id(context: &ToolContext, catalog: &[CatalogModel]) -> Option<String> {
    if let Some(path) = vector::model_path() {
        for model in catalog {
            if is_active_model(&path, &model.id) {
                return Some(model.id.clone());
            }
        }
    }

    if let Some(id) = context.config.settings.vector.active_model.as_deref()
        && !id.is_empty()
    {
        return Some(id.to_string());
    }

    let models_dir = context.config.assets_path.join("models");
    largest_flat_onnx_id(&models_dir)
}

/// Scan `models_dir` for top-level `*.onnx` files (legacy flat layout) and
/// return the id (filename without extension) of the largest one. Mirrors
/// `discovery.rs::discover_model` so `effective_active_model_id` reports what
/// the daemon would actually load on next start when no `active_model` pin
/// exists.
fn largest_flat_onnx_id(models_dir: &Path) -> Option<String> {
    let mut candidates: Vec<(u64, String)> = std::fs::read_dir(models_dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let extension = path.extension()?.to_str()?;
            if !extension.eq_ignore_ascii_case("onnx") {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() || metadata.len() == 0 {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_string();
            Some((metadata.len(), stem))
        })
        .collect();
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates.into_iter().next().map(|(_, stem)| stem)
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

    // T-202.9 P0 (Bug 2): determine whether the active model is actually changing.
    // Different models with the SAME embedding dimensionality still produce
    // incompatible vector spaces, so we must always clear vectors on any model
    // swap — not just when dims differ.
    let previous_model_id = context
        .config
        .settings
        .vector
        .active_model
        .clone();
    let model_changing = previous_model_id.as_deref() != Some(model_id);

    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root);
    let mut settings = config.settings.unwrap_or_else(MaenifoldSettings::default);
    settings.vector.active_model = Some(model_id.to_string());
    config.settings = Some(settings);
    write_agents_config(root, &config)?;

    // The daemon's existing sync logic will rebuild vec_concepts/vec_memory_files
    // on the next sync once the new model is loaded (handled by
    // `db::ensure_vec0_tables` + `sync::vector_embeddings`).
    let new_dims = load_catalog(&context.config.assets_path)
        .iter()
        .find(|m| m.id == model_id)
        .map(|m| m.dimensions);
    let current_dims = vector::embedding_dimensions().or_else(|| {
        db::open_connection(&context.config.database_path)
            .ok()
            .and_then(|conn| db::get_model_metadata(&conn, "embedding_dimensions"))
            .and_then(|s| s.parse::<usize>().ok())
    });
    // `dims_differ` is now informational only — used to shape the user-facing
    // message. Whether we clear vectors is governed by `model_changing`.
    let dims_differ = matches!((new_dims, current_dims), (Some(n), Some(c)) if n != c);

    let mut vectors_cleared = false;
    if model_changing {
        match db::open_connection(&context.config.database_path) {
            Ok(conn) => {
                // T-202.9: Drop both vec0 virtual tables so `ensure_vec0_tables`
                // rebuilds them with the correct dimensionality on next startup.
                // Use targeted upserts on `model_metadata` (instead of a blanket
                // DELETE) so we record the new active_model_id immediately and
                // — when known — the new embedding_dimensions, without nuking
                // unrelated metadata rows.
                let drop_sql = "DROP TABLE IF EXISTS vec_concepts; \
                                DROP TABLE IF EXISTS vec_memory_files;";
                let mut ok = conn.execute_batch(drop_sql).is_ok();
                if ok {
                    if let Err(e) = conn.execute(
                        "INSERT INTO model_metadata (key, value) VALUES (?1, ?2) \
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                        rusqlite::params!["active_model_id", model_id],
                    ) {
                        tracing::warn!(
                            "Failed to upsert active_model_id during activate for '{model_id}': {e}"
                        );
                        ok = false;
                    }
                    if let Some(dims) = new_dims {
                        if let Err(e) = conn.execute(
                            "INSERT INTO model_metadata (key, value) VALUES (?1, ?2) \
                             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                            rusqlite::params!["embedding_dimensions", dims.to_string()],
                        ) {
                            tracing::warn!(
                                "Failed to upsert embedding_dimensions during activate for '{model_id}': {e}"
                            );
                        }
                    }
                }

                if ok {
                    vectors_cleared = true;
                    tracing::info!(
                        "Vector index cleared for model swap '{:?}' -> '{model_id}' \
                         (current={current_dims:?} dims, new={new_dims:?} dims). \
                         Vectors will rebuild on next sync.",
                        previous_model_id
                    );
                } else {
                    tracing::warn!(
                        "Failed to clear vector tables during activate for '{model_id}'"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Could not open database to clear vectors during activate for '{model_id}': {e}"
                );
            }
        }
    }

    let message = if vectors_cleared && dims_differ {
        "Model changed (different dimensions). Vector index cleared. Restart daemon to load the new model; vectors will rebuild on next sync."
    } else if vectors_cleared {
        "Model changed. Vector index cleared (different models produce incompatible vector spaces even at matching dimensions). Restart daemon and re-sync to regenerate embeddings."
    } else if model_changing {
        "Model changed but vector tables could not be cleared automatically — restart daemon and run sync; existing vectors may need to be removed manually."
    } else {
        "Model already active. Restart daemon if it has not yet picked up this configuration."
    };

    Ok(json!({
        "activated": true,
        "modelId": model_id,
        "previousModelId": previous_model_id,
        "modelChanged": model_changing,
        "reembedRequired": model_changing,
        "vectorsCleared": vectors_cleared,
        "previousDimensions": current_dims,
        "newDimensions": new_dims,
        "dimensionsDiffer": dims_differ,
        "message": message,
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

/// T-202.9 remediation: drop vec0 virtual tables and clear the recorded
/// embedding dimensions so the next sync rebuilds embeddings from scratch
/// against the currently-active model. Does not change the active model.
fn handle_regenerate(context: &ToolContext, _payload: &Value) -> Result<Value> {
    let conn = db::open_connection(&context.config.database_path).map_err(|e| {
        MaenifoldError::Message(format!(
            "Failed to open database at '{}': {e}",
            context.config.database_path.display()
        ))
    })?;

    let sql = "DROP TABLE IF EXISTS vec_concepts; \
               DROP TABLE IF EXISTS vec_memory_files; \
               DELETE FROM model_metadata WHERE key = 'embedding_dimensions';";
    conn.execute_batch(sql)
        .map_err(|e| MaenifoldError::Message(format!("Failed to clear embeddings: {e}")))?;

    tracing::info!("Embeddings cleared via regenerate action; vectors will rebuild on next sync.");

    Ok(json!({
        "regenerated": true,
        "message": "Embeddings cleared. Run sync to rebuild.",
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
