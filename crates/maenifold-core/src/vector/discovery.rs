use super::output::{map_ort_error, select_best_session_output};
use super::tokenizer::load_tokenizer_assets;
use super::{ModelState, model_state};
use crate::Config;
use crate::error::{MaenifoldError, Result};
use crate::tools::embedding_models::load_catalog;
use ort::session::Session;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static ORT_INITIALIZED: OnceLock<std::result::Result<(), String>> = OnceLock::new();
const ORT_RUNTIME_NOT_FOUND_MESSAGE: &str = "ONNX Runtime library not found. Set ORT_DYLIB_PATH or place the library in the assets/lib directory.";

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
    let model_dir = model_path.parent().ok_or_else(|| {
        MaenifoldError::Message("embedding model has no parent directory".to_string())
    })?;
    init_ort_runtime(model_dir)?;
    let session = Session::builder()
        .map_err(map_ort_error)?
        .commit_from_file(&model_path)
        .map_err(map_ort_error)?;
    let embedding_dims = discover_embedding_dimensions(&session)?;
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

fn init_ort_runtime(model_dir: &Path) -> Result<()> {
    let init_result = ORT_INITIALIZED.get_or_init(|| init_ort_runtime_once(model_dir));

    match init_result {
        Ok(()) => Ok(()),
        Err(message) => Err(MaenifoldError::Message(message.clone())),
    }
}

fn init_ort_runtime_once(model_dir: &Path) -> std::result::Result<(), String> {
    let dylib_path = resolve_ort_dylib_path(model_dir)?;
    ort::init_from(dylib_path)
        .map_err(|_| ORT_RUNTIME_NOT_FOUND_MESSAGE.to_string())?
        .commit();
    Ok(())
}

fn resolve_ort_dylib_path(model_dir: &Path) -> std::result::Result<PathBuf, String> {
    if let Ok(path) = env::var("ORT_DYLIB_PATH") {
        let trimmed = path.trim();
        let dylib_path = PathBuf::from(trimmed);
        if !trimmed.is_empty() && dylib_path.is_file() {
            return Ok(dylib_path);
        }
    }

    let dylib_filename = ort_dylib_filename();
    let adjacent = model_dir.join(dylib_filename);
    if adjacent.is_file() {
        return Ok(adjacent);
    }

    let config = Config::current();
    let asset_lib = config.assets_path.join("lib").join(dylib_filename);
    if asset_lib.is_file() {
        return Ok(asset_lib);
    }

    Err(ORT_RUNTIME_NOT_FOUND_MESSAGE.to_string())
}

fn ort_dylib_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "onnxruntime.dll"
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        "libonnxruntime.so"
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        "libonnxruntime.dylib"
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        "libonnxruntime.so"
    }
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

pub(crate) fn model_candidate_dirs(config: &Config) -> Result<Vec<PathBuf>> {
    // T-20.5: RTM FR-7 / RTM-147 — canonicalize the executable path first so
    // symlinked installs (for example `/usr/local/bin/maenifold`) still resolve
    // executable-adjacent assets from the real binary location.
    let executable = env::current_exe()?.canonicalize()?;
    let base_dir = executable
        .parent()
        .ok_or(MaenifoldError::MissingExecutableParent)?;

    // T-202.9a: collect candidate model directories in priority order so we can
    // resolve the explicitly-configured `active_model` across release, dev, and
    // shared runtime layouts.
    let mut candidate_dirs: Vec<PathBuf> = Vec::new();
    candidate_dirs.push(base_dir.join("assets").join("models"));
    for ancestor in base_dir.ancestors() {
        if ancestor.join("Cargo.toml").exists() {
            candidate_dirs.push(ancestor.join("assets").join("models"));
        }
    }
    // T-20.5: RTM FR-7 / RTM-148 — also check the shared runtime root used by
    // the C# runtime: `~/maenifold/assets/models`.
    candidate_dirs.push(config.assets_path.join("models"));
    Ok(candidate_dirs)
}

fn find_model_path() -> Result<PathBuf> {
    let config = Config::current();
    let candidate_dirs = model_candidate_dirs(&config)?;

    // T-220.9: require explicit runtime model selection. Do NOT silently scan
    // for a fallback model; choosing the largest top-level ONNX file can load a
    // different vector space than the one used to create existing embeddings.
    let active_id = config
        .settings
        .vector
        .active_model
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            MaenifoldError::Message(
                "No active embedding model configured. Set settings.vector.active_model \
                 (serialized as settings.vector.activeModel) by running EmbeddingModels \
                 activate for an installed catalog model."
                    .to_string(),
            )
        })?;

    if !is_valid_model_id(active_id) {
        return Err(MaenifoldError::Message(format!(
            "Invalid active embedding model id '{active_id}'. Configure a catalog model id \
             containing only ASCII letters, digits, '.', '_' or '-'."
        )));
    }

    if !load_catalog(&config.assets_path)
        .iter()
        .any(|model| model.id == active_id)
    {
        return Err(MaenifoldError::Message(format!(
            "Configured model '{active_id}' is not in the curated catalog. Use EmbeddingModels activate to select an installed catalog model."
        )));
    }

    for dir in &candidate_dirs {
        let subdir_model = dir.join(active_id).join("model.onnx");
        if let Ok(metadata) = subdir_model.metadata()
            && metadata.is_file()
            && metadata.len() > 0
        {
            return Ok(subdir_model);
        }

        let flat_model = dir.join(format!("{active_id}.onnx"));
        if let Ok(metadata) = flat_model.metadata()
            && metadata.is_file()
            && metadata.len() > 0
        {
            return Ok(flat_model);
        }
    }

    Err(MaenifoldError::Message(format!(
        "Active embedding model '{active_id}' is not installed or unavailable in any \
         candidate models directory. Download it or activate a different installed model."
    )))
}

fn is_valid_model_id(model_id: &str) -> bool {
    !model_id.is_empty()
        && model_id != "."
        && !model_id.contains("..")
        && !model_id.contains('/')
        && !model_id.contains('\\')
        && model_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
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
