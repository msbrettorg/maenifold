use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, delete_file, file_exists, read_markdown, write_markdown};
use crate::sessions::{date_parts_from_session_id, pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::workflow::initial_workflow_frontmatter;
use chrono::Utc;
use serde::de::DeserializeOwned;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;

use super::{NodeState, NodeStatus, PipelineDefinition, compute_pipeline_status};

#[cfg(test)]
mod test_support {
    pub(super) use super::super::test_support::*;
}

pub(super) fn read_pipeline_session(
    context: &ToolContext,
    session_id: &str,
) -> Result<(String, Frontmatter, String)> {
    let uri = pipeline_session_uri(session_id);
    if !file_exists(&uri, &context.config)? {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} doesn't exist"
        )));
    }

    let (frontmatter, content, _) = read_markdown(&uri, &context.config)?;
    let frontmatter = frontmatter
        .ok_or_else(|| MaenifoldError::Message("ERROR: Invalid session file format".to_string()))?;

    if frontmatter.get("type").and_then(YamlValue::as_str) != Some("pipeline") {
        return Err(MaenifoldError::Message(format!(
            "Session {session_id} isn't a pipeline session"
        )));
    }

    Ok((uri, frontmatter, content))
}

pub(super) fn derive_pipeline_node_states(
    context: &ToolContext,
    node_states: &HashMap<String, NodeState>,
) -> Result<HashMap<String, NodeState>> {
    let mut derived = node_states.clone();

    for state in derived.values_mut() {
        let workflow_uri = workflow_session_uri(&state.session_id);
        if !file_exists(&workflow_uri, &context.config)? {
            return Err(MaenifoldError::Message(format!(
                "Workflow session {} doesn't exist",
                state.session_id
            )));
        }

        let (frontmatter, _, _) = read_markdown(&workflow_uri, &context.config)?;
        let frontmatter = frontmatter.ok_or_else(|| {
            MaenifoldError::Message(format!(
                "ERROR: Workflow session {} has invalid format",
                state.session_id
            ))
        })?;

        if frontmatter.get("type").and_then(YamlValue::as_str) != Some("workflow") {
            return Err(MaenifoldError::Message(format!(
                "Session {} isn't a workflow session",
                state.session_id
            )));
        }

        if let Some(status) = frontmatter
            .get("status")
            .and_then(YamlValue::as_str)
            .and_then(workflow_status_to_node_status)
        {
            state.status = status;
        }
    }

    Ok(derived)
}

fn workflow_status_to_node_status(status: &str) -> Option<NodeStatus> {
    match status {
        "pending" => Some(NodeStatus::Pending),
        "ready" => Some(NodeStatus::Ready),
        "active" => Some(NodeStatus::Active),
        "completed" => Some(NodeStatus::Completed),
        "abandoned" | "cancelled" => Some(NodeStatus::Abandoned),
        "stuck" => Some(NodeStatus::Stuck),
        _ => None,
    }
}

pub(super) struct NodeFile {
    pub(super) session_id: String,
    pub(super) node_id: String,
    pub(super) workflow_id: String,
}

pub(super) struct DeletePipelineSessionFilesResult {
    pub(super) cleaned_up_files: usize,
    pub(super) skipped_files: usize,
}

pub(super) fn frontmatter_value<T>(frontmatter: &Frontmatter, key: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let value = frontmatter.get(key).cloned().ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline session missing frontmatter '{key}'"
        ))
    })?;
    serde_yaml::from_value(value).map_err(Into::into)
}

pub(super) fn generate_pipeline_node_session_id(timestamp_millis: i64, node_id: &str) -> String {
    format!("workflow-{timestamp_millis}-{node_id}")
}

pub(super) fn allocate_node_session_id(context: &ToolContext, node_id: &str) -> Result<String> {
    let mut timestamp_millis = Utc::now().timestamp_millis();

    loop {
        let session_id = generate_pipeline_node_session_id(timestamp_millis, node_id);
        let workflow_uri = workflow_session_uri(&session_id);
        if !file_exists(&workflow_uri, &context.config)? {
            return Ok(session_id);
        }
        timestamp_millis += 1;
    }
}

pub(super) fn persist_pipeline_state(
    frontmatter: &mut Frontmatter,
    node_states: &HashMap<String, NodeState>,
    uri: &str,
    pipeline_content: &str,
    context: &ToolContext,
) -> Result<()> {
    persist_pipeline_state_with_status(
        frontmatter,
        node_states,
        None,
        uri,
        pipeline_content,
        context,
    )
}

pub(super) fn persist_pipeline_state_with_status(
    frontmatter: &mut Frontmatter,
    node_states: &HashMap<String, NodeState>,
    status_override: Option<&str>,
    uri: &str,
    pipeline_content: &str,
    context: &ToolContext,
) -> Result<()> {
    frontmatter.insert("nodeStates".to_string(), serde_yaml::to_value(node_states)?);
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String(
            status_override
                .unwrap_or_else(|| compute_pipeline_status(node_states))
                .to_string(),
        ),
    );
    write_markdown(uri, Some(frontmatter), pipeline_content, &context.config)
}

pub(super) fn delete_pipeline_session_files(
    context: &ToolContext,
    pipeline_session_id: &str,
    node_states: &HashMap<String, NodeState>,
) -> Result<DeletePipelineSessionFilesResult> {
    let mut cleaned_up_files = 0usize;
    let mut skipped_files = 0usize;
    let mut workflow_session_ids = HashSet::with_capacity(node_states.len());

    for state in node_states.values() {
        if workflow_session_ids.insert(state.session_id.clone()) {
            let workflow_uri = workflow_session_uri(&state.session_id);
            match delete_file(&workflow_uri, &context.config) {
                Ok(()) => cleaned_up_files += 1,
                Err(err) => {
                    skipped_files += 1;
                    eprintln!(
                        "Warning: failed to delete workflow session {}: {}",
                        state.session_id, err
                    );
                }
            }
        }
    }

    delete_file(&pipeline_session_uri(pipeline_session_id), &context.config)?;
    cleaned_up_files += 1;

    Ok(DeletePipelineSessionFilesResult {
        cleaned_up_files,
        skipped_files,
    })
}

pub(super) fn update_workflow_node_frontmatter(
    context: &ToolContext,
    workflow_session_id: &str,
    status: &str,
    conclusion: Option<&str>,
) -> Result<()> {
    let workflow_uri = workflow_session_uri(workflow_session_id);
    let (frontmatter, content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut frontmatter = frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Workflow session {workflow_session_id} has invalid format"
        ))
    })?;

    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    if let Some(conclusion) = conclusion {
        frontmatter.insert(
            "conclusion".to_string(),
            YamlValue::String(conclusion.to_string()),
        );
    }

    write_markdown(&workflow_uri, Some(&frontmatter), &content, &context.config)
}

pub(super) fn node_status_name(status: &NodeStatus) -> &'static str {
    match status {
        NodeStatus::Pending => "pending",
        NodeStatus::Ready => "ready",
        NodeStatus::Active => "active",
        NodeStatus::Completed => "completed",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Stuck => "stuck",
    }
}

pub(super) fn write_pipeline_session(
    context: &ToolContext,
    pipeline_id: &str,
    definition: &PipelineDefinition,
    node_states: &BTreeMap<String, YamlValue>,
    max_concurrency: u64,
) -> Result<()> {
    let uri = pipeline_session_uri(pipeline_id);
    let (year, month, day) = date_parts_from_session_id(pipeline_id);
    fs::create_dir_all(
        context
            .config
            .memory_path
            .join("pipelines")
            .join(format!("{year:04}"))
            .join(format!("{month:02}"))
            .join(format!("{day:02}")),
    )?;
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("Pipeline Session {pipeline_id}")),
    );
    frontmatter.insert(
        "permalink".to_string(),
        YamlValue::String(pipeline_id.to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("pipeline".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert("definition".to_string(), serde_yaml::to_value(definition)?);
    frontmatter.insert(
        "nodeStates".to_string(),
        YamlValue::Mapping(
            node_states
                .iter()
                .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                .collect(),
        ),
    );
    frontmatter.insert(
        "maxConcurrency".to_string(),
        serde_yaml::to_value(max_concurrency)?,
    );
    frontmatter.insert("first".to_string(), serde_yaml::to_value(0u64)?);
    frontmatter.insert("last".to_string(), serde_yaml::to_value(0u64)?);

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Pipeline Session\n",
        &context.config,
    )
}

pub(super) fn write_workflow_node_session(
    context: &ToolContext,
    pipeline_id: &str,
    node_file: &NodeFile,
) -> Result<()> {
    write_workflow_node_session_with_status(context, pipeline_id, node_file, "pending")
}

pub(super) fn write_workflow_node_session_with_status(
    context: &ToolContext,
    pipeline_id: &str,
    node_file: &NodeFile,
    status: &str,
) -> Result<()> {
    let uri = workflow_session_uri(&node_file.session_id);
    let mut frontmatter = initial_workflow_frontmatter(
        &node_file.session_id,
        std::slice::from_ref(&node_file.workflow_id),
        status,
    );
    frontmatter.insert(
        "pipelineId".to_string(),
        YamlValue::String(pipeline_id.to_string()),
    );
    frontmatter.insert(
        "nodeId".to_string(),
        YamlValue::String(node_file.node_id.to_string()),
    );
    frontmatter.insert(
        "workflowId".to_string(),
        YamlValue::String(node_file.workflow_id.to_string()),
    );

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Workflow Session\n",
        &context.config,
    )
}

#[cfg(test)]
mod tests {
    use super::super::handler::pipeline;
    use super::test_support::*;
    use super::*;
    use crate::markdown::read_markdown;
    use crate::sessions::{pipeline_session_uri, workflow_session_uri};
    use crate::util::json::OutputFormat;
    use serde_json::{Value, json};
    use serial_test::serial;
    use std::collections::HashMap;

    #[test]
    #[serial(config)]
    fn pipeline_view_rejects_missing_session() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let error = pipeline(
            &context,
            &json!({ "sessionId": "pipeline-1735689600999", "view": true }),
        )
        .expect_err("missing session should fail");

        assert_eq!(
            error.to_string(),
            "Session pipeline-1735689600999 doesn't exist"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_view_rejects_non_pipeline_session() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600001";
        seed_pipeline_view_session(&context, session_id, "workflow");

        let error = pipeline(&context, &json!({ "sessionId": session_id, "view": true }))
            .expect_err("non-pipeline session should fail");

        assert_eq!(
            error.to_string(),
            format!("Session {session_id} isn't a pipeline session")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_resume_view_derives_state_from_workflow_sessions() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600002";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600002-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600002-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
        update_workflow_node_frontmatter(
            &context,
            "workflow-1735689600002-root",
            "completed",
            Some("Completed [[root]]"),
        )
        .expect("mark workflow completed");

        let output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
            .expect("resume via view");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
        assert_eq!(
            data.get("ready").and_then(Value::as_array),
            Some(&vec![Value::String("child".to_string())])
        );
        let nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .expect("nodes array");
        let root = nodes
            .iter()
            .find(|node| node.get("nodeId") == Some(&Value::String("root".to_string())))
            .expect("root node");
        assert_eq!(
            root.get("status").and_then(Value::as_str),
            Some("completed")
        );

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);

        let (workflow_fm, _, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600002-root"),
            &context.config,
        )
        .expect("read workflow session");
        let workflow_fm = workflow_fm.expect("workflow frontmatter");
        assert_eq!(
            workflow_fm.get("status").and_then(YamlValue::as_str),
            Some("completed")
        );
    }
}
