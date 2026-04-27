use super::context::ToolContext;
use super::embedding_models::load_catalog_with_external_trust;
use super::embedding_models_scan::{
    active_model_status, effective_active_model_id, installed_layout,
};
use crate::db;
use crate::error::Result;
use crate::mcp::schema::EmbeddingModelsParams;
use crate::vector;
use crate::vector::model_candidate_dirs;
use serde_json::{Value, json};

pub(super) fn handle_list(context: &ToolContext, params: &EmbeddingModelsParams) -> Value {
    let models_dir = context.config.assets_path.join("models");
    let candidate_dirs = model_candidate_dirs(&context.config).unwrap_or_else(|e| {
        tracing::warn!("Failed to resolve runtime model candidate directories: {e}");
        vec![models_dir.clone()]
    });

    let catalog = load_catalog_with_external_trust(
        &context.config.assets_path,
        params.trust_external_catalog,
    );
    // T-202.9 P0 (Bug 3): determine the effective active model id once, so the
    // UI can render the "Active" badge regardless of layout (subdir/flat) and
    // regardless of whether the ONNX session has been lazily loaded yet.
    let active_id = effective_active_model_id(context, &catalog);
    let active_status = active_model_status(context, &catalog);

    let models: Vec<Value> = catalog
        .iter()
        .map(|model| {
            let layout = candidate_dirs
                .iter()
                .find_map(|models_dir| installed_layout(models_dir, &model.id));
            let is_active = active_id.as_deref() == Some(model.id.as_str());
            // T-202.9 P0 (Bug 1): a legacy flat install ({models_dir}/{id}.onnx)
            // counts as installed even though catalog tooling normally manages
            // only the subdir layout.
            let installed = layout.is_some();

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
        "activeModelStatus": active_status.to_json(),
    })
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

    // Reject path traversal and degenerate model IDs
    if model_id.is_empty()
        || model_id == "."
        || model_id.contains("..")
        || model_id.contains('/')
        || model_id.contains('\\')
        || !model_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err(crate::error::MaenifoldError::Message(format!(
            "Invalid model ID: '{model_id}'"
        )));
    }

    let active_model = context.config.settings.vector.active_model.as_deref();
    if active_model == Some(model_id) {
        return Err(crate::error::MaenifoldError::Message(format!(
            "Refusing to delete '{model_id}': it is the currently-active model. Activate a different model first."
        )));
    }

    let models_dir = context.config.assets_path.join("models");
    let target_dir = models_dir.join(model_id);
    let flat_path = models_dir.join(format!("{model_id}.onnx"));

    let exists_subdir = target_dir.is_dir();
    let exists_flat = flat_path.is_file();

    if exists_subdir {
        std::fs::remove_dir_all(&target_dir).map_err(|e| {
            crate::error::MaenifoldError::Message(format!("Failed to delete model directory: {e}"))
        })?;
    } else if exists_flat {
        std::fs::remove_file(&flat_path).map_err(|e| {
            crate::error::MaenifoldError::Message(format!(
                "Failed to delete legacy model file: {e}"
            ))
        })?;
    } else {
        return Err(crate::error::MaenifoldError::Message(format!(
            "Model '{model_id}' was not found in any supported layout"
        )));
    }

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
        crate::error::MaenifoldError::Message(format!("Failed to open database: {e}"))
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
