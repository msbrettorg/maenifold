#![cfg_attr(not(feature = "model-download"), allow(unused_imports))]

use super::context::ToolContext;
use super::ort_runtime_catalog::{load_ort_runtime_catalog, pick_ort_runtime_entry};
use crate::error::{MaenifoldError, Result};
use crate::mcp::schema::EmbeddingModelsParams;
use serde_json::{Value, json};

#[cfg(feature = "model-download")]
pub(super) fn handle_download_runtime(
    context: &ToolContext,
    params: &EmbeddingModelsParams,
) -> Result<Value> {
    let catalog =
        load_ort_runtime_catalog(&context.config.assets_path, params.trust_external_catalog);
    let entry = pick_ort_runtime_entry(
        &catalog,
        params.target_os.as_deref(),
        params.target_arch.as_deref(),
    )
    .map_err(MaenifoldError::Message)?
    .clone();
    let response_entry = entry.clone();
    let assets_path = context.config.assets_path.clone();

    let download_future =
        async move { super::ort_runtime_download::download_runtime(&entry, &assets_path).await };
    let result = match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            if !matches!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ) {
                return Err(MaenifoldError::Message(
                    "ONNX Runtime download requires a multi-thread Tokio runtime when invoked from async context".to_string(),
                ));
            }
            tokio::task::block_in_place(|| handle.block_on(download_future))
        }
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| {
                    MaenifoldError::Message(format!(
                        "Failed to build runtime for ONNX Runtime download: {error}"
                    ))
                })?;
            rt.block_on(download_future)
        }
    };

    match result {
        Ok(path) => Ok(json!({
            "downloaded": true,
            "runtimeId": response_entry.id,
            "version": response_entry.version,
            "targetOs": response_entry.target_os,
            "targetArch": response_entry.target_arch,
            "targetPath": path
                .strip_prefix(&context.config.maenifold_root)
                .ok()
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| "assets/lib/libonnxruntime.dylib".to_string()),
        })),
        Err(error) => Err(MaenifoldError::Message(format!(
            "Download failed for ONNX Runtime {}: {error}",
            response_entry.version
        ))),
    }
}

#[cfg(not(feature = "model-download"))]
pub(super) fn handle_download_runtime(
    _context: &ToolContext,
    _params: &EmbeddingModelsParams,
) -> Result<Value> {
    Err(MaenifoldError::Message(
        super::embedding_download::MODEL_DOWNLOAD_FEATURE_ERROR.to_string(),
    ))
}
