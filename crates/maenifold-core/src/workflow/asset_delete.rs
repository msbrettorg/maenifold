use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use regex::Regex;
use serde_json::{Value, json};
use serde_yml::Value as YamlValue;
use std::fs;
use std::io::ErrorKind;
use std::sync::LazyLock;
use walkdir::WalkDir;

use super::pipeline::PipelineDefinition;

static WORKFLOW_ASSET_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").expect("workflow asset name regex must compile")
});
pub(crate) fn delete_workflow_asset(context: &ToolContext, payload: &Value) -> Result<String> {
    let name = parse_workflow_asset_name(payload)?;
    // asset-store lock — held across ref-check+unlink, NOT across resource notification,
    // NOT while holding a pipeline lease.
    let _guard = super::asset_store::lock_asset_store()?;
    let references = running_pipeline_references(context, name)?;
    if !references.is_empty() {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Workflow asset '{name}' is referenced by running pipeline(s): {}",
            references.join(", ")
        )));
    }

    let path = context
        .config
        .assets_path
        .join("workflows")
        .join(format!("{name}.json"));
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            MaenifoldError::AssetNotFound("workflow".to_string(), name.to_string())
        } else {
            MaenifoldError::from(error)
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Workflow asset '{name}' is a symlink; refusing to delete"
        )));
    }
    fs::remove_file(&path)?;

    let deleted_files = vec![path.to_string_lossy().to_string()];
    let result = json!({
        "name": name,
        "deletedFiles": deleted_files,
    });
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn parse_workflow_asset_name(payload: &Value) -> Result<&str> {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("name"))?
        .trim();
    if name.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "Workflow asset name must not be empty".to_string(),
        ));
    }
    if !WORKFLOW_ASSET_NAME.is_match(name) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Invalid workflow asset name '{name}'. Names must be kebab-case \
             (^[a-z0-9]+(-[a-z0-9]+)*$)."
        )));
    }
    Ok(name)
}

fn running_pipeline_references(context: &ToolContext, workflow_name: &str) -> Result<Vec<String>> {
    let pipelines_dir = context.config.memory_path.join("pipelines");
    if !pipelines_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut references = Vec::new();
    for entry in WalkDir::new(&pipelines_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(session_id) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if running_pipeline_references_workflow(context, session_id, workflow_name)? {
            references.push(session_id.to_string());
        }
    }
    references.sort();
    Ok(references)
}

fn running_pipeline_references_workflow(
    context: &ToolContext,
    session_id: &str,
    workflow_name: &str,
) -> Result<bool> {
    let (frontmatter, _, _) = read_markdown(&pipeline_session_uri(session_id), &context.config)?;
    let Some(frontmatter) = frontmatter else {
        return Ok(false);
    };
    if frontmatter.get("type").and_then(YamlValue::as_str) != Some("pipeline") {
        return Ok(false);
    }
    if !is_active_pipeline_status(frontmatter.get("status").and_then(YamlValue::as_str)) {
        return Ok(false);
    }
    let Some(definition_value) = frontmatter.get("definition").cloned() else {
        return Ok(false);
    };
    let mut definition: PipelineDefinition = serde_yml::from_value(definition_value)?;
    definition.migrate_legacy_edges();
    Ok(definition
        .nodes
        .iter()
        .any(|node| node.workflow_id() == Some(workflow_name)))
}

fn is_active_pipeline_status(stored_status: Option<&str>) -> bool {
    matches!(stored_status, Some("active" | "retrying" | "running"))
}

#[cfg(test)]
#[path = "asset_delete_tests.rs"]
mod tests;
