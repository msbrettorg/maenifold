//! HuggingFace download engine for embedding models.
//!
//! Downloads ONNX + tokenizer assets to a staging directory, then performs
//! an atomic rename into the final model directory under `{models_dir}/{model_id}/`.

#[cfg(feature = "model-download")]
use super::embedding_models::CatalogFile;
use super::embedding_models::CatalogModel;
#[cfg(feature = "model-download")]
use hf_hub::api::tokio::{Api, Progress};
#[cfg(feature = "model-download")]
use hf_hub::{Repo, RepoType};
#[cfg(feature = "model-download")]
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
#[cfg(feature = "model-download")]
use tokio::{fs, io::AsyncReadExt};

#[allow(dead_code)]
pub const MODEL_DOWNLOAD_FEATURE_ERROR: &str = "Model download requires the `model-download` feature. Rebuild with `cargo build --features model-download`.";

/// Snapshot of an in-flight download, exposed to the catalog `list` handler so
/// the desktop UI can render progress beside the "Downloading…" button. Bytes
/// reflect the *currently downloading file* (typically `model.onnx`); the
/// secondary `model.onnx_data` file restarts the counter.
#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    pub current_bytes: u64,
    pub total_bytes: u64,
    pub filename: String,
}

static PROGRESS: OnceLock<Mutex<HashMap<String, ProgressSnapshot>>> = OnceLock::new();

fn progress_map() -> &'static Mutex<HashMap<String, ProgressSnapshot>> {
    PROGRESS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Read the current download progress for a model id, if a download is in
/// flight. Returns `None` once the model is fully downloaded (the entry is
/// removed in `Progress::finish`).
pub fn get_download_progress(model_id: &str) -> Option<ProgressSnapshot> {
    progress_map().lock().ok()?.get(model_id).cloned()
}

#[cfg(feature = "model-download")]
#[derive(Clone)]
struct DownloadTracker {
    model_id: String,
}

#[cfg(feature = "model-download")]
impl Progress for DownloadTracker {
    async fn init(&mut self, size: usize, filename: &str) {
        let snap = ProgressSnapshot {
            current_bytes: 0,
            total_bytes: size as u64,
            filename: filename.to_string(),
        };
        if let Ok(mut map) = progress_map().lock() {
            map.insert(self.model_id.clone(), snap);
        }
    }

    async fn update(&mut self, size: usize) {
        if let Ok(mut map) = progress_map().lock()
            && let Some(entry) = map.get_mut(&self.model_id)
        {
            entry.current_bytes = entry.current_bytes.saturating_add(size as u64);
        }
    }

    async fn finish(&mut self) {
        if let Ok(mut map) = progress_map().lock() {
            map.remove(&self.model_id);
        }
    }
}

/// Download a model's ONNX + tokenizer files from HuggingFace.
///
/// ONNX files are pulled from `onnx_repo` if provided (some models do not
/// host pre-converted ONNX in their primary repo), otherwise from `hf_repo`.
/// Tokenizer/config files are always pulled from `hf_repo`.
///
/// Files are written to a hidden staging directory and atomically renamed
/// into place on success. A pre-existing install short-circuits with an error.
#[cfg(feature = "model-download")]
pub async fn download_model(model: &CatalogModel, models_dir: &Path) -> Result<PathBuf, String> {
    let model_id = model.id.as_str();
    let final_dir = models_dir.join(model_id);
    if final_dir.exists() {
        return Err(format!("Model {model_id} is already installed"));
    }

    // T-202.9 remediation: refuse to download into the catalog layout when a
    // legacy flat install (`{models_dir}/{model_id}.onnx`) is present. The two
    // layouts cannot coexist cleanly — the runtime scan would still pick the
    // flat file by size, so the catalog install would be invisible. Force the
    // user to remove the legacy artifact first.
    let legacy_flat = models_dir.join(format!("{model_id}.onnx"));
    if legacy_flat.is_file() {
        return Err(format!(
            "Legacy model exists for {model_id}; remove it first before downloading the catalog version."
        ));
    }

    validate_catalog_entry(model)?;

    let staging = models_dir.join(format!(".{}.downloading", model_id));
    let _ = fs::remove_dir_all(&staging).await; // clean any prior failed attempt
    fs::create_dir_all(&staging)
        .await
        .map_err(|e| e.to_string())?;

    let api = Api::new().map_err(|e| e.to_string())?;

    // ONNX files from onnx_repo (or hf_repo if no override)
    let onnx_source = model
        .onnx_repo
        .as_deref()
        .unwrap_or(&model.huggingface_repo);
    let onnx_revision = model.onnx_revision.as_deref().unwrap_or(&model.revision);
    let onnx_api = api.repo(Repo::with_revision(
        onnx_source.to_string(),
        RepoType::Model,
        onnx_revision.to_string(),
    ));
    let tok_api = api.repo(Repo::with_revision(
        model.huggingface_repo.clone(),
        RepoType::Model,
        model.revision.clone(),
    ));

    // T-202.39: report progress on the large ONNX files only — the tokenizer/
    // config files below are tiny and not worth wiring through the tracker.
    let tracker = DownloadTracker {
        model_id: model_id.to_string(),
    };

    for file in &model.files {
        let api = match file.source.as_str() {
            "onnx" => &onnx_api,
            "hf" => &tok_api,
            other => return Err(format!("Unknown file source '{other}' for {}", file.name)),
        };
        let source = if file.source == "onnx" {
            api.download_with_progress(&file.source_path, tracker.clone())
                .await
        } else {
            api.get(&file.source_path).await
        };
        match source {
            Ok(path) => {
                let staged = staging.join(&file.name);
                fs::copy(&path, &staged).await.map_err(|e| e.to_string())?;
                verify_sha256(&staged, file).await?;
            }
            Err(e) if file.optional => {
                tracing::warn!(
                    "Optional file {} was not available for embedding model {model_id}: {e}",
                    file.name
                );
            }
            Err(e) => return Err(format!("Failed to download {}: {e}", file.name)),
        }
    }

    // Ensure the progress entry is cleared even if `download_with_progress`
    // exited without calling `finish` (e.g. metadata fetch failure for the
    // optional `model.onnx_data`).
    if let Ok(mut map) = progress_map().lock() {
        map.remove(model_id);
    }

    // Atomic rename: staging → final
    fs::rename(&staging, &final_dir)
        .await
        .map_err(|e| format!("Failed to finalize: {e}"))?;

    Ok(final_dir)
}

#[cfg(not(feature = "model-download"))]
#[allow(dead_code)]
pub async fn download_model(_model: &CatalogModel, _models_dir: &Path) -> Result<PathBuf, String> {
    Err(MODEL_DOWNLOAD_FEATURE_ERROR.to_string())
}

#[cfg(feature = "model-download")]
fn validate_catalog_entry(model: &CatalogModel) -> Result<(), String> {
    if model.revision.trim().is_empty() {
        return Err(format!("Model {} is missing immutable revision", model.id));
    }
    if model.files.is_empty() {
        return Err(format!(
            "Model {} has no verified files in catalog",
            model.id
        ));
    }
    if !model
        .files
        .iter()
        .any(|file| file.name == "model.onnx" && !file.optional)
    {
        return Err(format!("Model {} is missing required model.onnx", model.id));
    }
    for file in &model.files {
        if file.name.trim().is_empty()
            || file.source_path.trim().is_empty()
            || file.sha256.trim().is_empty()
        {
            return Err(format!(
                "Model {} has incomplete verification metadata for a catalog file",
                model.id
            ));
        }
        if file.sha256.len() != 64 || !file.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "Model {} has invalid SHA-256 metadata for {}",
                model.id, file.name
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "model-download")]
async fn verify_sha256(path: &Path, file: &CatalogFile) -> Result<(), String> {
    let mut input = fs::File::open(path).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != file.sha256.to_ascii_lowercase() {
        return Err(format!(
            "SHA-256 verification failed for {}: expected {}, got {}",
            file.name, file.sha256, actual
        ));
    }
    Ok(())
}
