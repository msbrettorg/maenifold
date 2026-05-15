use std::collections::HashSet;

use serde_json::{Value, json};
use serde_yml::Value as YamlValue;

use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, read_markdown};
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};

use super::PipelineDefinition;
use super::acp_reaper::{live_acp_sessions, orphaned_acp_worker_count};
use super::engine_runs::compute_pipeline_runs;
use super::engine_status::{
    completed_node_count, definition_node_count, derive_pipeline_list_status, pipeline_node_states,
};
use super::view_nodes::pipeline_nodes_json_with_live_sessions;

pub(super) fn list_pipelines(context: &ToolContext) -> Result<String> {
    list_pipelines_impl(context, read_markdown)
}

pub(super) fn list_pipelines_impl(
    context: &ToolContext,
    read_markdown_fn: fn(
        &str,
        &crate::config::Config,
    ) -> Result<(Option<crate::markdown::Frontmatter>, String, String)>,
) -> Result<String> {
    let pipelines_dir = context.config.memory_path.join("pipelines");
    let mut pipelines = Vec::new();
    crate::db::GraphDatabase::initialize_database(&context.config)?;
    let connection = crate::db::open_connection(&context.config.database_path)?;
    if pipelines_dir.is_dir() {
        let live_sessions = live_acp_sessions();
        for entry in walkdir::WalkDir::new(&pipelines_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            let Some(session_id) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            let uri = pipeline_session_uri(session_id);
            match read_markdown_fn(&uri, &context.config) {
                Ok((Some(frontmatter), _, _)) => {
                    if frontmatter.get("type").and_then(YamlValue::as_str) != Some("pipeline") {
                        continue;
                    }
                    let node_states = pipeline_node_states(&frontmatter);
                    let status = derive_pipeline_list_status(
                        session_id,
                        frontmatter.get("status").and_then(YamlValue::as_str),
                        &node_states,
                        &context.config,
                    );
                    let entry = augment_list_payload(
                        context,
                        session_id,
                        &frontmatter,
                        status,
                        &live_sessions,
                    )?;
                    pipelines.push(entry);
                }
                Ok((None, _, _)) => {
                    if let Some(entry) = crate::db::lookup_pipeline_degraded_json(
                        &connection,
                        &uri,
                        "markdown_unreadable",
                        "pipeline markdown had no frontmatter",
                    )? {
                        pipelines.push(entry);
                    }
                }
                Err(error) => {
                    let reason = if matches!(error, MaenifoldError::FileDataless { .. }) {
                        "dataless"
                    } else {
                        "markdown_unreadable"
                    };
                    if let Some(entry) = crate::db::lookup_pipeline_degraded_json(
                        &connection,
                        &uri,
                        reason,
                        &error.to_string(),
                    )? {
                        pipelines.push(entry);
                    }
                }
            }
        }
    }
    pipelines.sort_by(|a, b| {
        a.get("pipelineId")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("pipelineId").and_then(Value::as_str).unwrap_or(""))
    });
    let result = json!({ "pipelines": pipelines });
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

pub(super) fn augment_list_payload(
    context: &ToolContext,
    session_id: &str,
    frontmatter: &Frontmatter,
    status: String,
    live_sessions: &HashSet<String>,
) -> Result<Value> {
    let node_states = pipeline_node_states(frontmatter);
    let definition = frontmatter
        .get("definition")
        .and_then(|value| serde_yml::from_value::<PipelineDefinition>(value.clone()).ok());
    let nodes = match definition.as_ref() {
        Some(definition) => pipeline_nodes_json_with_live_sessions(
            context,
            definition,
            &node_states,
            "",
            live_sessions,
        )?,
        None => Vec::new(),
    };
    let orphaned_workers = if status != "running" {
        orphaned_acp_worker_count(&node_states, live_sessions)
    } else {
        0
    };
    let mut entry = json!({
        "pipelineId": frontmatter.get("permalink").and_then(serde_yml::Value::as_str).unwrap_or(session_id),
        "status": status,
        "nodeCount": definition_node_count(frontmatter.get("definition")),
        "completedCount": completed_node_count(&node_states),
        "runs": compute_pipeline_runs(&node_states),
        "nodes": nodes,
    });
    if let Some(name) = frontmatter
        .get("name")
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        entry["name"] = json!(name);
    }
    if let Some(description) = frontmatter
        .get("description")
        .and_then(YamlValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        entry["description"] = json!(description);
    }
    if orphaned_workers > 0 {
        entry["degraded"] = json!({
            "reason": "orphaned-acp-workers",
            "count": orphaned_workers,
            "orphanedAcpWorkers": orphaned_workers,
        });
    }
    Ok(entry)
}
