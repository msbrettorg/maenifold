use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, from_str_safe, success};
use regex::Regex;
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use uuid::Uuid;

const MAX_WORKFLOW_ASSET_BYTES: usize = 512 * 1024;

static WORKFLOW_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").expect("workflow id regex must compile")
});

pub fn save_workflow_asset(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = payload
        .get("id")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("id"))?;
    validate_workflow_id(id)?;

    let content = payload
        .get("content")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("content"))?;
    validate_workflow_content(id, content)?;

    let target_path = context
        .config
        .assets_path
        .join("workflows")
        .join(format!("{id}.json"));
    write_atomically(&target_path, content)?;

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({ "id": id })),
        OutputFormat::Markdown => format!("Saved workflow asset '{id}'."),
    })
}

fn validate_workflow_id(id: &str) -> Result<()> {
    // T-122.3: strict kebab-case — regex rejects path separators, spaces,
    // uppercase, leading/trailing dashes, and consecutive dashes.
    if !WORKFLOW_ID_REGEX.is_match(id) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Invalid workflow id '{id}'. \
             Workflow IDs must be kebab-case (^[a-z0-9]+(-[a-z0-9]+)*$)."
        )));
    }

    Ok(())
}

fn validate_workflow_content(id: &str, content: &str) -> Result<()> {
    if content.len() > MAX_WORKFLOW_ASSET_BYTES {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Workflow content exceeds the 512KB limit for '{id}'."
        )));
    }

    let value = from_str_safe::<Value>(content).map_err(|error| {
        MaenifoldError::InvalidArgument(format!("Workflow content must be valid JSON: {error}"))
    })?;

    let object = value.as_object().ok_or_else(|| {
        MaenifoldError::InvalidArgument("Workflow content must be a JSON object.".to_string())
    })?;

    let content_id = object.get("id").and_then(Value::as_str).ok_or_else(|| {
        MaenifoldError::InvalidArgument(
            "Workflow content must contain an 'id' field matching the payload id.".to_string(),
        )
    })?;

    if content_id != id {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Workflow content id '{content_id}' must match payload id '{id}'."
        )));
    }

    Ok(())
}

fn write_atomically(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Workflow asset path has no parent directory: {}",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent)?;

    for _ in 0..8 {
        let temp_path = create_sidecar_path(path, "workflow-tmp");
        let backup_path = create_sidecar_path(path, "workflow-prev");
        let result = (|| -> std::io::Result<()> {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            drop(file);

            let existing_moved = if path.exists() {
                fs::rename(path, &backup_path)?;
                true
            } else {
                false
            };

            if let Err(error) = fs::rename(&temp_path, path) {
                if existing_moved {
                    let _ = fs::rename(&backup_path, path);
                    sync_directory(parent);
                }
                return Err(error);
            }

            sync_directory(parent);

            if existing_moved {
                fs::remove_file(&backup_path)?;
                sync_directory(parent);
            }

            Ok(())
        })();

        let _ = fs::remove_file(&temp_path);
        let _ = fs::remove_file(&backup_path);

        match result {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }

    Err(MaenifoldError::Message(format!(
        "Failed to allocate atomic temp file for {}",
        path.display()
    )))
}

fn create_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("workflow.json");

    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.{suffix}-{}", Uuid::new_v4()))
}

fn sync_directory(path: &Path) {
    let _ = fs::File::open(path).and_then(|file| file.sync_all());
}

// ── T-122.3: SaveWorkflowAsset contract tests ──────────────────────────────

#[cfg(test)]
#[path = "save_tests.rs"]
mod tests;
