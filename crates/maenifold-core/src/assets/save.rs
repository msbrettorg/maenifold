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
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tools::ToolContext;
    use crate::util::json::OutputFormat;

    // ── ID validation ──────────────────────────────────────────────────────

    #[test]
    fn valid_ids_are_accepted() {
        for id in ["a", "my-workflow", "workflow-v2", "abc123", "a1-b2-c3"] {
            assert!(validate_workflow_id(id).is_ok(), "should accept '{id}'");
        }
    }

    #[test]
    fn invalid_ids_are_rejected() {
        for id in [
            "",
            "MyWorkflow",          // uppercase
            "my_workflow",         // underscore
            "my workflow",         // space
            "-leading",            // leading dash
            "trailing-",           // trailing dash
            "double--dash",        // consecutive dashes
            "../../etc/passwd",    // path traversal
            "a/b",                 // forward slash
            r"a\b",                // backslash
            "workflow.json",       // dot
        ] {
            assert!(
                validate_workflow_id(id).is_err(),
                "should reject '{id}'"
            );
        }
    }

    // ── Content validation ─────────────────────────────────────────────────

    #[test]
    fn valid_content_is_accepted() {
        let content = r#"{"id":"my-flow","nodes":[]}"#;
        assert!(validate_workflow_content("my-flow", content).is_ok());
    }

    #[test]
    fn non_json_content_is_rejected() {
        assert!(validate_workflow_content("x", "not json").is_err());
    }

    #[test]
    fn json_array_content_is_rejected() {
        assert!(validate_workflow_content("x", r#"[{"id":"x"}]"#).is_err());
    }

    #[test]
    fn missing_id_field_is_rejected() {
        assert!(validate_workflow_content("x", r#"{"name":"x"}"#).is_err());
    }

    #[test]
    fn mismatched_id_field_is_rejected() {
        let content = r#"{"id":"other","nodes":[]}"#;
        assert!(validate_workflow_content("my-flow", content).is_err());
    }

    #[test]
    fn content_exceeding_512kb_is_rejected() {
        let inner = "x".repeat(MAX_WORKFLOW_ASSET_BYTES + 1);
        let content = format!(r#"{{"id":"x","data":"{inner}"}}"#);
        assert!(validate_workflow_content("x", &content).is_err());
    }

    // ── Integration: full tool invocation ─────────────────────────────────

    fn make_context(root: &std::path::Path) -> ToolContext {
        Config::override_root(root);
        let config = Config::current();
        Config::reset_overrides();
        ToolContext::new(config, OutputFormat::Json)
    }

    #[test]
    fn save_workflow_asset_writes_file_and_returns_success() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = make_context(dir.path());
        let payload = json!({
            "id": "test-flow",
            "content": r#"{"id":"test-flow","nodes":[]}"#
        });

        let result = save_workflow_asset(&ctx, &payload).expect("save should succeed");

        // Response must signal success and echo the id
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("response is JSON");
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"]["id"], "test-flow");

        // File must exist on disk at the correct path
        let expected = dir
            .path()
            .join("assets")
            .join("workflows")
            .join("test-flow.json");
        assert!(expected.exists(), "workflow file should be on disk");
        let on_disk = fs::read_to_string(&expected).expect("read file");
        let on_disk_json: serde_json::Value =
            serde_json::from_str(&on_disk).expect("file is valid JSON");
        assert_eq!(on_disk_json["id"], "test-flow");
    }

    #[test]
    fn save_workflow_asset_overwrites_existing_file_atomically() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = make_context(dir.path());

        for version in ["v1", "v2"] {
            let content = format!(r#"{{"id":"my-flow","version":"{version}"}}"#);
            let payload = json!({ "id": "my-flow", "content": content });
            save_workflow_asset(&ctx, &payload).expect("save should succeed");
        }

        let path = dir
            .path()
            .join("assets")
            .join("workflows")
            .join("my-flow.json");
        let on_disk: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(on_disk["version"], "v2", "file should contain v2");
    }

    #[test]
    fn save_workflow_asset_rejects_invalid_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = make_context(dir.path());
        let payload = json!({
            "id": "../../etc/passwd",
            "content": r#"{"id":"../../etc/passwd"}"#
        });
        assert!(save_workflow_asset(&ctx, &payload).is_err());
    }

    #[test]
    fn save_workflow_asset_rejects_id_mismatch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = make_context(dir.path());
        let payload = json!({
            "id": "my-flow",
            "content": r#"{"id":"other-flow"}"#
        });
        assert!(save_workflow_asset(&ctx, &payload).is_err());
    }
}
