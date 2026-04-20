use super::output::{map_ort_error, select_best_session_output};
use super::tokenizer::load_tokenizer_assets;
use super::{ModelState, model_state};
use crate::Config;
use crate::error::{MaenifoldError, Result};
use ort::session::Session;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Load/initialize the model (called lazily on first use)
pub fn load_model() -> Result<()> {
    // T-9.1: RTM FR-7 — model initialization remains lazy and idempotent after the
    // [[vector.rs]] split so runtime discovery still survives model swaps.
    let mut guard = model_state()
        .lock()
        .map_err(|_| MaenifoldError::Message("embedding model mutex poisoned".to_string()))?;
    if guard.is_some() {
        return Ok(());
    }

    let model_path = find_model_path()?;
    let session = Session::builder()
        .map_err(map_ort_error)?
        .commit_from_file(&model_path)
        .map_err(map_ort_error)?;
    let embedding_dims = discover_embedding_dimensions(&session)?;
    let model_dir = model_path.parent().ok_or_else(|| {
        MaenifoldError::Message("embedding model has no parent directory".to_string())
    })?;
    let (tokenizer, max_tokens) = load_tokenizer_assets(model_dir)?;
    let model_filename = model_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MaenifoldError::NonUtf8Path(model_path.clone()))?
        .to_string();

    *guard = Some(ModelState {
        session,
        tokenizer,
        embedding_dims,
        max_tokens,
        model_filename,
        model_path: model_path.clone(),
    });
    Ok(())
}

/// Cleanup/release model resources
pub fn cleanup() {
    if let Ok(mut guard) = model_state().lock() {
        *guard = None;
    }
}

/// Check if model is initialized
pub fn is_initialized() -> bool {
    model_state()
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

fn find_model_path() -> Result<PathBuf> {
    // T-20.5: RTM FR-7 / RTM-147 — canonicalize the executable path first so
    // symlinked installs (for example `/usr/local/bin/maenifold`) still resolve
    // executable-adjacent assets from the real binary location.
    let executable = env::current_exe()?.canonicalize()?;
    let base_dir = executable
        .parent()
        .ok_or(MaenifoldError::MissingExecutableParent)?;

    let direct_models_dir = base_dir.join("assets").join("models");
    if let Some(model_path) = discover_model(&direct_models_dir) {
        return Ok(model_path);
    }

    for ancestor in base_dir.ancestors() {
        let workspace_manifest = ancestor.join("Cargo.toml");
        let models_dir = ancestor.join("assets").join("models");
        if workspace_manifest.exists()
            && let Some(model_path) = discover_model(&models_dir)
        {
            return Ok(model_path);
        }
    }

    // T-20.5: RTM FR-7 / RTM-148 — after the existing exe-adjacent and Cargo
    // walk-up probes, also check the shared runtime root used by the C# runtime:
    // `~/maenifold/assets/models`.
    let shared_models_dir = Config::current().assets_path.join("models");
    if let Some(model_path) = discover_model(&shared_models_dir) {
        return Ok(model_path);
    }

    // T-9.1: RTM FR-7 — removed compile-time CARGO_MANIFEST_DIR fallback that baked
    // the build-machine path into the binary. The two strategies above (exe-adjacent +
    // Cargo.toml walk-up) already cover release installs and dev-mode builds.
    Err(MaenifoldError::Message(
        "embedding model not found: expected a non-empty *.onnx file under assets/models"
            .to_string(),
    ))
}

fn discover_model(models_dir: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(models_dir)
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

            Some((metadata.len(), path))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates.into_iter().next().map(|(_, path)| path)
}

fn discover_embedding_dimensions(session: &Session) -> Result<usize> {
    let chosen = select_best_session_output(session.outputs())?;
    chosen
        .dims
        .last()
        .copied()
        .filter(|dimension| *dimension > 0)
        .and_then(|dimension| usize::try_from(dimension).ok())
        .ok_or_else(|| {
            MaenifoldError::Message(format!(
                "embedding model output `{}` does not expose a concrete embedding dimension",
                chosen.name
            ))
        })
}
