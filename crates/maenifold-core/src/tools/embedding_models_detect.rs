use super::context::ToolContext;
use super::embedding_models::{CatalogModel, load_catalog};
use crate::db;
use crate::error::Result;
use crate::vector;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn handle_list(context: &ToolContext) -> Value {
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
            let download_progress = super::embedding_download::get_download_progress(&model.id)
                .map(|p| {
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

    let current_dims =
        vector::embedding_dimensions().unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS);

    json!({
        "models": models,
        "currentDimensions": current_dims,
        "activeModelId": active_id,
    })
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

/// T-202.9a: delete an installed model directory from disk. Refuses to delete
/// the currently-active model to avoid leaving the daemon without a usable
/// model on next restart.
pub(super) fn handle_delete(context: &ToolContext, payload: &Value) -> Result<Value> {
    let model_id = payload
        .get("modelId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            crate::error::MaenifoldError::Message(
                "Missing required parameter 'modelId'".to_string(),
            )
        })?;

    let active_model = context.config.settings.vector.active_model.as_deref();
    if active_model == Some(model_id) {
        return Err(crate::error::MaenifoldError::Message(format!(
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
        return Err(crate::error::MaenifoldError::Message(
            "Cannot delete while no active model is configured. Activate a different model first."
                .to_string(),
        ));
    }

    if !target_dir.exists() {
        return Err(crate::error::MaenifoldError::Message(format!(
            "Model directory not found: {}",
            target_dir.display()
        )));
    }

    std::fs::remove_dir_all(&target_dir).map_err(|e| {
        crate::error::MaenifoldError::Message(format!(
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

/// T-202.9 remediation: clear the recorded embedding dimensions so the next
/// sync (running in the daemon with vec0 loaded) can rebuild embeddings from
/// scratch against the currently-active model. Does not change the active
/// model.
pub(super) fn handle_regenerate(context: &ToolContext, _payload: &Value) -> Result<Value> {
    let conn = db::open_connection(&context.config.database_path).map_err(|e| {
        crate::error::MaenifoldError::Message(format!(
            "Failed to open database at '{}': {e}",
            context.config.database_path.display()
        ))
    })?;

    // Idempotent: table may not exist on first run before any sync
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS model_metadata (key TEXT PRIMARY KEY, value TEXT); \
         DELETE FROM model_metadata WHERE key = 'embedding_dimensions';",
    )
    .map_err(|e| {
        crate::error::MaenifoldError::Message(format!("Failed to clear embedding metadata: {e}"))
    })?;

    tracing::info!(
        "Embedding metadata cleared via regenerate action; vectors will rebuild on next sync."
    );

    Ok(json!({
        "regenerated": true,
        "message": "Embedding metadata cleared. Vectors will rebuild on next sync.",
    }))
}
