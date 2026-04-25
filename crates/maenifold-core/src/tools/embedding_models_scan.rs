use super::context::ToolContext;
use super::embedding_models::CatalogModel;
use crate::vector::model_candidate_dirs;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// T-202.9 P0 (Bug 1): detect whether a model is installed and which on-disk
/// layout it uses.
///
/// - `Some("subdir")` — `{models_dir}/{model_id}/*.onnx` (catalog-managed
///   layout used by the download flow).
/// - `Some("flat")` — `{models_dir}/{model_id}.onnx` (legacy layout that can
///   still be selected explicitly via `settings.vector.active_model`).
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActiveModelState {
    Available,
    NoActiveModel,
    InvalidModel,
    UnavailableModel,
}

impl ActiveModelState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::NoActiveModel => "noActiveModel",
            Self::InvalidModel => "invalidModel",
            Self::UnavailableModel => "unavailableModel",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ActiveModelStatus {
    pub model_id: Option<String>,
    pub state: ActiveModelState,
    pub installed_layout: Option<&'static str>,
    pub message: String,
}

impl ActiveModelStatus {
    pub fn to_json(&self) -> Value {
        json!({
            "state": self.state.as_str(),
            "modelId": self.model_id.as_deref(),
            "installedLayout": self.installed_layout,
            "message": self.message.as_str(),
        })
    }
}

/// Determine the configured active model status for UI surfaces. Runtime model
/// discovery now requires an explicit `settings.vector.active_model`; this
/// helper deliberately does not infer an active model from installed ONNX files.
pub(super) fn active_model_status(
    context: &ToolContext,
    catalog: &[CatalogModel],
) -> ActiveModelStatus {
    let configured = context
        .config
        .settings
        .vector
        .active_model
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());

    let Some(model_id) = configured else {
        return ActiveModelStatus {
            model_id: None,
            state: ActiveModelState::NoActiveModel,
            installed_layout: None,
            message: "No active embedding model is configured.".to_string(),
        };
    };

    if !catalog.iter().any(|model| model.id == model_id) {
        return ActiveModelStatus {
            model_id: Some(model_id.to_string()),
            state: ActiveModelState::InvalidModel,
            installed_layout: None,
            message: format!(
                "Configured active embedding model '{model_id}' is not in the catalog."
            ),
        };
    }

    let layout = model_candidate_dirs(&context.config)
        .ok()
        .and_then(|dirs| installed_layout_in_candidates(&dirs, model_id));
    if layout.is_none() {
        return ActiveModelStatus {
            model_id: Some(model_id.to_string()),
            state: ActiveModelState::UnavailableModel,
            installed_layout: None,
            message: format!("Configured active embedding model '{model_id}' is not installed."),
        };
    }

    ActiveModelStatus {
        model_id: Some(model_id.to_string()),
        state: ActiveModelState::Available,
        installed_layout: layout,
        message: format!("Configured active embedding model '{model_id}' is available."),
    }
}

/// T-202.9 P0 (Bug 3): determine the explicitly configured active catalog id.
/// No implicit "largest installed ONNX file" fallback is allowed.
pub(super) fn effective_active_model_id(
    context: &ToolContext,
    catalog: &[CatalogModel],
) -> Option<String> {
    active_model_status(context, catalog).model_id
}

fn installed_layout_in_candidates(
    candidate_dirs: &[PathBuf],
    model_id: &str,
) -> Option<&'static str> {
    candidate_dirs
        .iter()
        .find_map(|models_dir| installed_layout(models_dir, model_id))
}
