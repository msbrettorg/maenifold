use super::context::ToolContext;
use super::embedding_models::CatalogModel;
use crate::vector;
use std::path::Path;

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
pub(super) fn installed_layout(models_dir: &Path, model_id: &str) -> Option<&'static str> {
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
pub(super) fn effective_active_model_id(
    context: &ToolContext,
    catalog: &[CatalogModel],
) -> Option<String> {
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
