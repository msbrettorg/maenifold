use crate::assets::AssetManager;
use crate::error::{MaenifoldError, Result};
use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use chrono::Utc;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

use super::handler_validation::{validate_workflow_node_definitions, workflow_node_file};
use super::session_io::{
    delete_pipeline_session_files, derive_pipeline_node_states, frontmatter_value,
    generate_pipeline_node_session_id, read_pipeline_session, write_pipeline_session,
    write_workflow_node_session,
};
use super::view::view_pipeline_session;
use super::{
    lease, NodeState, NodeStatus, PipelineDefinition, compute_ready_set, validate_pipeline,
};

pub(super) fn create_pipeline(context: &ToolContext, payload: &Value) -> Result<String> {
    let Some(definition_value) = payload.get("definition") else {
        return Err(MaenifoldError::Message(
            "Pipeline definition is required".to_string(),
        ));
    };

    AssetManager::initialize_assets(&context.config)?;

    let definition: PipelineDefinition = serde_json::from_value(definition_value.clone())?;
    validate_pipeline(&definition)?;
    validate_workflow_node_definitions(context, &definition.nodes)?;

    let max_concurrency = parse_max_concurrency(payload)?;
    let prefix = parse_pipeline_id_prefix(payload)?;
    let timestamp_millis = Utc::now().timestamp_millis();
    let pipeline_id = format!("{prefix}-{timestamp_millis}");
    let mut node_states = HashMap::with_capacity(definition.nodes.len());
    let mut node_states_frontmatter = BTreeMap::new();
    let mut node_files = Vec::with_capacity(definition.nodes.len());
    let mut response_nodes = Vec::with_capacity(definition.nodes.len());

    for node in &definition.nodes {
        let session_id = generate_pipeline_node_session_id(timestamp_millis, node.id());
        let state = NodeState {
            session_id: session_id.clone(),
            status: NodeStatus::Pending,
            attempts: 0,
            thinking_session_id: None,
            output: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        };
        node_states.insert(node.id().to_string(), state.clone());
        node_states_frontmatter.insert(node.id().to_string(), serde_yaml::to_value(&state)?);
        node_files.push(workflow_node_file(node, session_id.clone()));
        response_nodes.push(json!({
            "nodeId": node.id(),
            "workflowId": node.workflow_id(),
            "sessionId": session_id,
        }));
    }

    let ready = compute_ready_set(&definition, &node_states);
    write_pipeline_session(
        context,
        &pipeline_id,
        &definition,
        &node_states_frontmatter,
        max_concurrency,
    )?;
    for node_file in &node_files {
        write_workflow_node_session(context, &pipeline_id, node_file)?;
    }

    let result = json!({
        "pipelineId": pipeline_id,
        "ready": ready,
        "nodes": response_nodes,
    });
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

#[allow(clippy::too_many_lines)]
pub(super) fn list_pipelines(context: &ToolContext) -> Result<String> {
    let pipelines_dir = context.config.memory_path.join("pipelines");
    let mut pipelines = Vec::new();

    if pipelines_dir.is_dir() {
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
            let Ok((Some(frontmatter), _, _)) = read_markdown(&uri, &context.config) else {
                continue;
            };
            if frontmatter.get("type").and_then(YamlValue::as_str) != Some("pipeline") {
                continue;
            }

            let node_states = frontmatter
                .get("nodeStates")
                .and_then(|value| {
                    serde_yaml::from_value::<HashMap<String, NodeState>>(value.clone()).ok()
                })
                .unwrap_or_default();
            let completed_count = node_states
                .values()
                .filter(|state| matches!(state.status, NodeStatus::Completed))
                .count();
            let status = frontmatter
                .get("status")
                .and_then(YamlValue::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| super::compute_pipeline_status(&node_states).to_string());
            let node_count = frontmatter
                .get("definition")
                .and_then(|value| serde_yaml::from_value::<PipelineDefinition>(value.clone()).ok())
                .map(|definition| definition.nodes.len())
                .unwrap_or(0);

            pipelines.push(json!({
                "pipelineId": frontmatter
                    .get("permalink")
                    .and_then(YamlValue::as_str)
                    .unwrap_or(session_id),
                "status": status,
                "nodeCount": node_count,
                "completedCount": completed_count,
            }));
        }
    }

    pipelines.sort_by(|a, b| {
        let a_id = a.get("pipelineId").and_then(Value::as_str).unwrap_or("");
        let b_id = b.get("pipelineId").and_then(Value::as_str).unwrap_or("");
        a_id.cmp(b_id)
    });

    let result = json!({ "pipelines": pipelines });
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

pub(super) fn delete_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    lease::with_exclusive_session_lock(
        session_id,
        &context.config,
        || {
            MaenifoldError::InvalidArgument(format!(
                "Pipeline {session_id} is running; cancel it before deleting"
            ))
        },
        || {
            let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
            let node_states = match frontmatter_value(&frontmatter, "nodeStates") {
                Ok(raw_node_states) => match derive_pipeline_node_states(context, &raw_node_states) {
                    Ok(derived_node_states) => derived_node_states,
                    Err(err) => {
                        eprintln!(
                            "Warning: failed to derive node states for pipeline {session_id}; \
                             falling back to raw frontmatter nodeStates: {err}"
                        );
                        raw_node_states
                    }
                },
                Err(err) => {
                    eprintln!(
                        "Warning: failed to read raw nodeStates for corrupted pipeline \
                         {session_id}; deleting pipeline without node cleanup metadata: {err}"
                    );
                    HashMap::new()
                }
            };

            let has_active_work = node_states
                .values()
                .any(|state| matches!(state.status, NodeStatus::Active | NodeStatus::Retrying));
            if has_active_work {
                return Err(MaenifoldError::InvalidArgument(format!(
                    "Pipeline {session_id} has active work; cancel it before deleting"
                )));
            }

            let deleted_files = delete_pipeline_session_files(context, session_id, &node_states)?;
            let result = json!({
                "deletedSessionId": session_id,
                "cleanedUpFiles": deleted_files.cleaned_up_files,
                "skippedFiles": deleted_files.skipped_files,
            });
            Ok(match context.output_format {
                OutputFormat::Json => success(result),
                OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
            })
        },
    )
}

pub(super) fn view_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    view_pipeline_session(context, session_id)
}

fn parse_max_concurrency(payload: &Value) -> Result<u64> {
    match payload.get("maxConcurrency") {
        None => Ok(8),
        Some(Value::Number(value)) => value.as_u64().filter(|value| *value > 0).ok_or_else(|| {
            MaenifoldError::InvalidArgument("maxConcurrency must be a positive integer".to_string())
        }),
        Some(_) => Err(MaenifoldError::InvalidArgument(
            "maxConcurrency must be a positive integer".to_string(),
        )),
    }
}

fn parse_pipeline_id_prefix(payload: &Value) -> Result<&str> {
    let prefix = match payload.get("name") {
        None | Some(Value::Null) => return Ok("pipeline"),
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok("pipeline");
            }
            trimmed
        }
        Some(_) => {
            return Err(MaenifoldError::InvalidArgument(
                "name must be a string".to_string(),
            ));
        }
    };

    let mut chars = prefix.chars();
    let Some(first) = chars.next() else {
        return Ok("pipeline");
    };
    if !first.is_ascii_alphabetic() || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        return Err(MaenifoldError::InvalidArgument(
            "name must start with a letter and contain only alphanumeric characters or hyphens"
                .to_string(),
        ));
    }
    if prefix.len() > 64 {
        return Err(MaenifoldError::InvalidArgument(
            "name must be 64 characters or fewer".to_string(),
        ));
    }

    Ok(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::{delete_file, file_exists, read_markdown, write_markdown};
    use crate::sessions::{pipeline_session_uri, workflow_session_uri};
    use crate::util::json::OutputFormat;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::io::Write as _;

    use super::super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
    use super::super::PipelineNode;

    const TS: i64 = 1_735_689_600_000;

    fn workflow_node(id: &str) -> PipelineNode {
        PipelineNode::Workflow {
            id: id.to_string(),
            workflow_id: "test-workflow-a".to_string(),
            acp_client: Some(format!("agent-{id}")),
            prompt: None,
            timeout: None,
        }
    }

    fn states(specs: &[(&str, NodeStatus, u32)]) -> HashMap<String, NodeState> {
        specs
            .iter()
            .map(|(node_id, status, attempts)| {
                (
                    (*node_id).to_string(),
                    NodeState {
                        session_id: format!("workflow-{TS}-{node_id}"),
                        status: status.clone(),
                        attempts: *attempts,
                        thinking_session_id: None,
                        output: None,
                        failure_reason: None,
                        retry_after: None,
                        retry_count: None,
                    },
                )
            })
            .collect()
    }

    #[test]
    #[serial]
    fn delete_pipeline_refuses_running_pipeline_before_reading_pipeline_state() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600000-delete-running-lock-first";
        let definition = PipelineDefinition {
            nodes: vec![workflow_node("node")],
            edges: vec![],
        };
        let node_states = states(&[("node", NodeStatus::Completed, 1)]);
        seed_pipeline_session_with_workflows(
            &context,
            session_id,
            definition,
            node_states.clone(),
            1,
        );

        let lock_path = lease::pipeline_lock_path(session_id, &context.config);
        std::fs::create_dir_all(lock_path.parent().expect("lock parent")).expect("create lock dir");
        let file = std::fs::File::create(&lock_path).expect("create lock file");
        let mut rw = fd_lock::RwLock::new(file);
        let mut guard = rw.try_write().expect("acquire pipeline lock");
        guard.write_all(b"test-holder").expect("write lock holder");

        delete_file(
            &workflow_session_uri(&node_states["node"].session_id),
            &context.config,
        )
        .expect("delete workflow session after lock");

        let error = delete_pipeline(&context, session_id).expect_err("delete must refuse running pipeline");
        assert!(
            error
                .to_string()
                .contains("is running; cancel it before deleting"),
            "expected running-pipeline refusal, got: {error}"
        );
    }

    #[test]
    #[serial]
    fn delete_pipeline_falls_back_to_raw_node_states_when_derived_state_is_corrupt() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600000-delete-corrupt-derived-state";
        let definition = PipelineDefinition {
            nodes: vec![workflow_node("node")],
            edges: vec![],
        };
        let node_states = states(&[("node", NodeStatus::Completed, 1)]);
        seed_pipeline_session_with_workflows(
            &context,
            session_id,
            definition,
            node_states.clone(),
            1,
        );

        delete_file(
            &workflow_session_uri(&node_states["node"].session_id),
            &context.config,
        )
        .expect("delete workflow session to corrupt derived state");

        let response = delete_pipeline(&context, session_id).expect("delete should tolerate corruption");
        let payload: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(payload["success"], Value::Bool(true));
        assert_eq!(payload["data"]["deletedSessionId"], Value::String(session_id.to_string()));
        assert_eq!(
            payload["data"]["cleanedUpFiles"],
            Value::Number(serde_json::Number::from(1)),
        );
        assert_eq!(
            payload["data"]["skippedFiles"],
            Value::Number(serde_json::Number::from(1)),
        );
        assert!(
            !file_exists(&pipeline_session_uri(session_id), &context.config)
                .expect("check pipeline deletion"),
            "pipeline session should be deleted"
        );
    }

    #[test]
    #[serial]
    fn delete_pipeline_cleans_up_historical_orphaned_child_workflow_sessions() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600000-delete-orphaned-child-workflows";
        let definition = PipelineDefinition {
            nodes: vec![workflow_node("node")],
            edges: vec![],
        };
        let node_states = states(&[("node", NodeStatus::Completed, 1)]);
        seed_pipeline_session_with_workflows(
            &context,
            session_id,
            definition,
            node_states.clone(),
            1,
        );

        let orphan_session_id = "workflow-1735689600000-node-orphan";
        let (frontmatter, content, _) = read_markdown(
            &workflow_session_uri(&node_states["node"].session_id),
            &context.config,
        )
        .expect("read workflow session");
        let mut orphan_frontmatter = frontmatter.expect("workflow frontmatter");
        orphan_frontmatter.insert(
            "title".to_string(),
            format!("Workflow Session {orphan_session_id}").into(),
        );
        orphan_frontmatter.insert("permalink".to_string(), orphan_session_id.into());
        orphan_frontmatter.insert("nodeId".to_string(), "node-orphan".into());
        write_markdown(
            &workflow_session_uri(orphan_session_id),
            Some(&orphan_frontmatter),
            &content,
            &context.config,
        )
        .expect("write orphaned workflow session");

        let response = delete_pipeline(&context, session_id).expect("delete pipeline");
        let payload: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(payload["success"], Value::Bool(true));
        assert_eq!(
            payload["data"]["cleanedUpFiles"],
            Value::Number(serde_json::Number::from(3)),
        );
        assert!(
            !file_exists(&workflow_session_uri(orphan_session_id), &context.config)
                .expect("check orphan workflow deletion"),
            "historical orphaned workflow session should be deleted"
        );
    }

    #[test]
    #[serial]
    fn delete_pipeline_does_not_delete_sibling_pipeline_child_workflow_sessions() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let target_session_id = "pipeline-1735689600000-delete-target-pipeline";
        let sibling_session_id = "pipeline-1735689600000-delete-sibling-pipeline";
        let definition = PipelineDefinition {
            nodes: vec![workflow_node("target-node")],
            edges: vec![],
        };
        let sibling_definition = PipelineDefinition {
            nodes: vec![workflow_node("sibling-node")],
            edges: vec![],
        };
        let target_node_states = states(&[("target-node", NodeStatus::Completed, 1)]);
        let sibling_node_states = states(&[("sibling-node", NodeStatus::Completed, 1)]);
        seed_pipeline_session_with_workflows(
            &context,
            target_session_id,
            definition,
            target_node_states.clone(),
            1,
        );
        seed_pipeline_session_with_workflows(
            &context,
            sibling_session_id,
            sibling_definition,
            sibling_node_states.clone(),
            1,
        );

        let sibling_orphan_session_id = "workflow-1735689600000-sibling-orphan";
        let (frontmatter, content, _) = read_markdown(
            &workflow_session_uri(&sibling_node_states["sibling-node"].session_id),
            &context.config,
        )
        .expect("read sibling workflow session");
        let mut sibling_orphan_frontmatter = frontmatter.expect("sibling workflow frontmatter");
        sibling_orphan_frontmatter.insert(
            "title".to_string(),
            format!("Workflow Session {sibling_orphan_session_id}").into(),
        );
        sibling_orphan_frontmatter
            .insert("permalink".to_string(), sibling_orphan_session_id.into());
        sibling_orphan_frontmatter.insert("nodeId".to_string(), "sibling-orphan".into());
        write_markdown(
            &workflow_session_uri(sibling_orphan_session_id),
            Some(&sibling_orphan_frontmatter),
            &content,
            &context.config,
        )
        .expect("write sibling orphaned workflow session");

        let response = delete_pipeline(&context, target_session_id).expect("delete target pipeline");
        let payload: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(payload["success"], Value::Bool(true));
        assert_eq!(
            payload["data"]["cleanedUpFiles"],
            Value::Number(serde_json::Number::from(2)),
        );
        assert!(
            file_exists(
                &workflow_session_uri(&sibling_node_states["sibling-node"].session_id),
                &context.config
            )
            .expect("check sibling workflow preservation"),
            "sibling pipeline workflow session should be preserved"
        );
        assert!(
            file_exists(&workflow_session_uri(sibling_orphan_session_id), &context.config)
                .expect("check sibling orphan preservation"),
            "sibling pipeline orphaned workflow session should be preserved"
        );
    }

    #[test]
    #[serial]
    fn delete_pipeline_allows_delete_when_raw_node_states_are_unreadable() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1735689600000-delete-corrupt-raw-node-states";
        let definition = PipelineDefinition {
            nodes: vec![workflow_node("node")],
            edges: vec![],
        };
        let node_states = states(&[("node", NodeStatus::Completed, 1)]);
        seed_pipeline_session_with_workflows(
            &context,
            session_id,
            definition,
            node_states.clone(),
            1,
        );

        let pipeline_uri = pipeline_session_uri(session_id);
        let (frontmatter, content, _) =
            read_markdown(&pipeline_uri, &context.config).expect("read pipeline");
        let mut frontmatter = frontmatter.expect("pipeline frontmatter");
        frontmatter.remove("nodeStates");
        write_markdown(&pipeline_uri, Some(&frontmatter), &content, &context.config)
            .expect("write corrupted pipeline");

        let response = delete_pipeline(&context, session_id).expect("delete should tolerate missing raw node states");
        let payload: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(payload["success"], Value::Bool(true));
        assert_eq!(payload["data"]["deletedSessionId"], Value::String(session_id.to_string()));
        assert!(
            !file_exists(&pipeline_uri, &context.config).expect("check pipeline deletion"),
            "pipeline session should be deleted"
        );
        assert!(
            file_exists(
                &workflow_session_uri(&node_states["node"].session_id),
                &context.config
            )
            .expect("check workflow session remains"),
            "workflow session should remain when nodeStates are unreadable"
        );
    }
}
