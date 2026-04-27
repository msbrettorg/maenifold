use super::context::ToolContext;
use super::embedding_models::{CatalogModel, load_catalog_with_external_trust};
use crate::acp_config::{AgentsConfig, MaenifoldSettings};
use crate::db;
use crate::error::{MaenifoldError, Result};
use crate::mcp::schema::EmbeddingModelsParams;
use crate::vector;
use serde_json::{Value, json};
use std::path::Path;

#[cfg(feature = "model-download")]
pub(super) fn handle_download(
    context: &ToolContext,
    params: &EmbeddingModelsParams,
) -> Result<Value> {
    let model_id = params.model_id.as_deref().ok_or_else(|| {
        MaenifoldError::Message("Missing required parameter 'modelId'".to_string())
    })?;

    let catalog = load_catalog_with_external_trust(
        &context.config.assets_path,
        params.trust_external_catalog,
    );
    let entry = catalog
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| MaenifoldError::Message(format!("Unknown model: {model_id}")))?;

    let models_dir = context.config.assets_path.join("models");

    require_license_opt_in(entry, params.allow_restricted_license)?;
    let model = entry.clone();
    let dir = models_dir.clone();

    // T-202.37 P0: previously this used `tokio::spawn` fire-and-forget, which
    // caused the UI to show "Downloading…" indefinitely because (1) the spawned
    // task could be dropped when the MCP tool dispatch returned, and (2) any
    // download error was only emitted to tracing and never surfaced to the
    // caller. Run the download synchronously from the (sync) tool handler so
    // success/failure flows back to the MCP response and the frontend's poll
    // observes installed=true (or surfaces the error in its catch block).
    let download_future =
        async move { super::embedding_download::download_model(&model, &dir).await };

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
        Ok(_path) => {
            tracing::info!("Downloaded embedding model {model_id}");
            Ok(json!({
                "downloaded": true,
                "modelId": model_id,
                "installLayout": "subdir",
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

#[cfg(not(feature = "model-download"))]
pub(super) fn handle_download(
    _context: &ToolContext,
    _params: &EmbeddingModelsParams,
) -> Result<Value> {
    Err(MaenifoldError::Message(
        super::embedding_download::MODEL_DOWNLOAD_FEATURE_ERROR.to_string(),
    ))
}

/// T-202.9a: persist `active_model` selection into the on-disk maenifold.json.
/// Verifies the model is present in the catalog and installed under
/// `{models_dir}/{model_id}/model.onnx` (subdirectory layout) before writing.
/// Daemon restart + sync is required for the change to take effect.
pub(super) fn handle_activate(
    context: &ToolContext,
    params: &EmbeddingModelsParams,
) -> Result<Value> {
    let model_id = params.model_id.as_deref().ok_or_else(|| {
        MaenifoldError::Message("Missing required parameter 'modelId'".to_string())
    })?;

    let catalog = load_catalog_with_external_trust(
        &context.config.assets_path,
        params.trust_external_catalog,
    );
    let entry = catalog.iter().find(|m| m.id == model_id);
    let Some(entry) = entry else {
        return Err(MaenifoldError::Message(format!(
            "Unknown model: {model_id}"
        )));
    };
    require_license_opt_in(entry, params.allow_restricted_license)?;

    let models_dir = context.config.assets_path.join("models");
    let subdir_path = models_dir.join(model_id).join("model.onnx");
    let flat_path = models_dir.join(format!("{model_id}.onnx"));
    let installed = subdir_path
        .metadata()
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false)
        || flat_path
            .metadata()
            .map(|m| m.is_file() && m.len() > 0)
            .unwrap_or(false);
    if !installed {
        return Err(MaenifoldError::Message(format!(
            "Model '{model_id}' is not installed in a supported layout — download it first."
        )));
    }

    // T-202.9 P0 (Bug 2): determine whether the active model is actually changing.
    // Different models with the SAME embedding dimensionality still produce
    // incompatible vector spaces, so we must always clear vectors on any model
    // swap — not just when dims differ.
    let previous_model_id = context.config.settings.vector.active_model.clone();
    let model_changing = previous_model_id.as_deref() != Some(model_id);

    let root = &context.config.maenifold_root;
    let mut config = AgentsConfig::load(root)?;
    let mut settings = config.settings.unwrap_or_else(MaenifoldSettings::default);
    settings.vector.active_model = Some(model_id.to_string());
    config.settings = Some(settings);
    write_agents_config(root, &config)?;

    // The daemon's existing sync logic will rebuild vec_concepts/vec_memory_files
    // on the next sync once the new model is loaded (handled by
    // `db::ensure_vec0_tables` + `sync::vector_embeddings`).
    let new_dims = Some(entry.dimensions);
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

fn require_license_opt_in(model: &CatalogModel, allow_restricted_license: bool) -> Result<()> {
    if is_permissive_license(&model.license) {
        return Ok(());
    }

    if allow_restricted_license {
        tracing::warn!(
            "Restricted embedding model license explicitly allowed for model {} with license {}",
            model.id,
            model.license
        );
        return Ok(());
    }

    Err(MaenifoldError::Message(format!(
        "Model '{}' has license '{}'. Downloads or activation for non-MIT/Apache licenses require allowRestrictedLicense=true.",
        model.id, model.license
    )))
}

fn is_permissive_license(license: &str) -> bool {
    matches!(
        license.trim().to_ascii_lowercase().as_str(),
        "mit" | "apache-2.0" | "apache 2.0" | "apache2"
    )
}

/// T-202.9a: write the agents config to `{root}/maenifold.json`.
/// Mirrors the formatting/path semantics of `api::config_manager::save_config`
/// without taking an HTTP-layer dependency.
fn write_agents_config(root: &Path, config: &AgentsConfig) -> Result<()> {
    let path = root.join("maenifold.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            MaenifoldError::Message(format!("Failed to ensure config directory: {e}"))
        })?;
    }
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| MaenifoldError::Message(format!("Failed to serialize config: {e}")))?;
    std::fs::write(&path, content)
        .map_err(|e| MaenifoldError::Message(format!("Failed to write config: {e}")))
}
