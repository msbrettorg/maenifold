//! HuggingFace download engine for embedding models.
//!
//! Downloads ONNX + tokenizer assets to a staging directory, then performs
//! an atomic rename into the final model directory under `{models_dir}/{model_id}/`.

use hf_hub::api::tokio::Api;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Download a model's ONNX + tokenizer files from HuggingFace.
///
/// ONNX files are pulled from `onnx_repo` if provided (some models do not
/// host pre-converted ONNX in their primary repo), otherwise from `hf_repo`.
/// Tokenizer/config files are always pulled from `hf_repo`.
///
/// Files are written to a hidden staging directory and atomically renamed
/// into place on success. A pre-existing install short-circuits with an error.
pub async fn download_model(
    model_id: &str,
    hf_repo: &str,
    onnx_repo: Option<&str>,
    models_dir: &Path,
) -> Result<PathBuf, String> {
    let final_dir = models_dir.join(model_id);
    if final_dir.exists() {
        return Err(format!(
            "Model {} already installed at {}",
            model_id,
            final_dir.display()
        ));
    }

    // T-202.9 remediation: refuse to download into the catalog layout when a
    // legacy flat install (`{models_dir}/{model_id}.onnx`) is present. The two
    // layouts cannot coexist cleanly — the runtime scan would still pick the
    // flat file by size, so the catalog install would be invisible. Force the
    // user to remove the legacy artifact first.
    let legacy_flat = models_dir.join(format!("{model_id}.onnx"));
    if legacy_flat.is_file() {
        return Err(format!(
            "Legacy model exists at {}, remove it first before downloading the catalog version.",
            legacy_flat.display()
        ));
    }

    let staging = models_dir.join(format!(".{}.downloading", model_id));
    let _ = fs::remove_dir_all(&staging).await; // clean any prior failed attempt
    fs::create_dir_all(&staging)
        .await
        .map_err(|e| e.to_string())?;

    let api = Api::new().map_err(|e| e.to_string())?;

    // ONNX files from onnx_repo (or hf_repo if no override)
    let onnx_source = onnx_repo.unwrap_or(hf_repo);
    let onnx_api = api.model(onnx_source.to_string());

    // Download model.onnx
    let onnx_path = onnx_api
        .get("onnx/model.onnx")
        .await
        .map_err(|e| format!("ONNX download failed: {e}"))?;
    fs::copy(&onnx_path, staging.join("model.onnx"))
        .await
        .map_err(|e| e.to_string())?;

    // Try model.onnx_data (external data file — some models have it)
    if let Ok(data_path) = onnx_api.get("onnx/model.onnx_data").await {
        fs::copy(&data_path, staging.join("model.onnx_data"))
            .await
            .map_err(|e| e.to_string())?;
    }

    // Tokenizer files from the original repo (not onnx_repo)
    let tok_api = api.model(hf_repo.to_string());
    for filename in ["tokenizer.json", "tokenizer_config.json", "config.json"] {
        match tok_api.get(filename).await {
            Ok(path) => {
                fs::copy(&path, staging.join(filename))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Err(_) if filename == "config.json" => {} // optional
            Err(e) => return Err(format!("Failed to download {filename}: {e}")),
        }
    }

    // Atomic rename: staging → final
    fs::rename(&staging, &final_dir)
        .await
        .map_err(|e| format!("Failed to finalize: {e}"))?;

    Ok(final_dir)
}
