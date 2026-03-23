use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use crate::workflow::{format_workflow_step, read_workflow_definition, workflow_steps};
use serde_json::json;
use serde_yaml::Value as YamlValue;

use super::session_io::{
    derive_pipeline_node_states, frontmatter_value, persist_pipeline_state, read_pipeline_session,
};
#[cfg(test)]
use super::test_support;
use super::view::node_ids_with_status;
use super::{
    NodeStatus, PipelineDefinition, PipelineNode, compute_ready_set, workflow_id_for_node,
};

// T-45.7: RTM-337 / RTM-342 / RTM-343 / RTM-344
// Dispatch computes the ready set, atomically claims eligible nodes (pending→active),
// renders the first workflow step into each dispatched session, and returns the result.
pub(super) fn dispatch_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let mut node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let max_concurrency: u64 = frontmatter
        .get("maxConcurrency")
        .and_then(YamlValue::as_u64)
        .unwrap_or(8);

    // Compute ready set and apply maxConcurrency limit.
    let ready = compute_ready_set(&definition, &node_states);
    let active_count = node_states
        .values()
        .filter(|state| matches!(state.status, NodeStatus::Active))
        .count() as u64;
    let dispatch_slots = max_concurrency.saturating_sub(active_count) as usize;
    let to_dispatch: Vec<String> = ready.into_iter().take(dispatch_slots).collect();

    // Track the dispatch attempt up-front so any partial persistence on error records
    // the same dispatch sequence number as a fully successful run.
    let last = frontmatter
        .get("last")
        .and_then(YamlValue::as_u64)
        .unwrap_or(0);
    frontmatter.insert("last".to_string(), serde_yaml::to_value(last + 1)?);

    // Best-effort dispatch: only include nodes activated in this call, but persist
    // progress after each node so retries observe already-activated sessions.
    let mut dispatched = Vec::with_capacity(to_dispatch.len());
    for node_id in &to_dispatch {
        let node_session_id = node_states
            .get(node_id)
            .ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{node_id}'"
                ))
            })?
            .session_id
            .clone();

        let node = definition
            .nodes
            .iter()
            .find(|n| n.id() == *node_id)
            .ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline definition missing node '{node_id}'"
                ))
            })?;

        let dispatched_node = {
            let state = node_states.get_mut(node_id).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline session missing node state for '{node_id}'"
                ))
            })?;

            match node {
                PipelineNode::Workflow { .. } => {
                    // Activate the workflow session: atomically update status to active and render first step.
                    let activated = activate_workflow_node_session(
                        context,
                        workflow_id_for_node(node)?,
                        &node_session_id,
                    )?;
                    state.status = NodeStatus::Active;
                    activated
                        .then(|| build_dispatched_node_response(node, &node_session_id))
                        .transpose()?
                }
                PipelineNode::Prompt { .. }
                | PipelineNode::SequentialThinking { .. }
                | PipelineNode::Tool { .. } => {
                    let activated = activate_non_workflow_node_session(context, &node_session_id)?;
                    state.status = NodeStatus::Active;
                    activated
                        .then(|| build_dispatched_node_response(node, &node_session_id))
                        .transpose()?
                }
            }
        };

        persist_pipeline_state(
            &mut frontmatter,
            &node_states,
            &uri,
            &pipeline_content,
            context,
        )?;

        if let Some(dispatched_node) = dispatched_node {
            dispatched.push(dispatched_node);
        }
    }

    persist_pipeline_state(
        &mut frontmatter,
        &node_states,
        &uri,
        &pipeline_content,
        context,
    )?;

    // Build response with current pipeline state.
    let active_nodes = node_ids_with_status(&definition, &node_states, NodeStatus::Active)?;
    let completed_nodes = node_ids_with_status(&definition, &node_states, NodeStatus::Completed)?;
    let remaining = node_states
        .values()
        .filter(|state| {
            matches!(
                state.status,
                NodeStatus::Pending | NodeStatus::Active | NodeStatus::Ready
            )
        })
        .count();

    let result = json!({
        "dispatched": dispatched,
        "active": active_nodes,
        "completed": completed_nodes,
        "remaining": remaining,
    });

    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

/// Activate a workflow node session with a single write that updates its frontmatter
/// status to `active` and renders the first workflow step into the session body.
///
/// Returns `Ok(true)` when activation performed a write, or `Ok(false)` when the
/// session was already active and should be skipped.
pub(super) fn activate_workflow_node_session(
    context: &ToolContext,
    workflow_id: &str,
    session_id: &str,
) -> Result<bool> {
    let workflow_uri = workflow_session_uri(session_id);
    let (session_frontmatter, existing_content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut session_frontmatter = session_frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Workflow session {session_id} has invalid format"
        ))
    })?;

    if session_frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        == Some("active")
    {
        return Ok(false);
    }

    let workflow_value = read_workflow_definition(&context.config, workflow_id)?;
    let steps = workflow_steps(&workflow_value, workflow_id)?;

    // Transition status from pending to active and render the first step into the
    // session content before performing a single write.
    session_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    let updated_content = if let Some(first_step) = steps.first() {
        let heading = format!("Step 1/{}", steps.len());
        let rendered_step = format_workflow_step(first_step)?;
        update_activation_section_frontmatter(
            &mut session_frontmatter,
            &existing_content,
            &heading,
            &rendered_step,
        );
        append_step_section(&existing_content, &heading, &rendered_step)
    } else {
        existing_content
    };

    write_markdown(
        &workflow_uri,
        Some(&session_frontmatter),
        &updated_content,
        &context.config,
    )?;

    Ok(true)
}

fn activate_non_workflow_node_session(context: &ToolContext, session_id: &str) -> Result<bool> {
    let workflow_uri = workflow_session_uri(session_id);
    let (session_frontmatter, existing_content, _) = read_markdown(&workflow_uri, &context.config)?;
    let mut session_frontmatter = session_frontmatter.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "ERROR: Workflow session {session_id} has invalid format"
        ))
    })?;

    if session_frontmatter
        .get("status")
        .and_then(YamlValue::as_str)
        == Some("active")
    {
        return Ok(false);
    }

    session_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    write_markdown(
        &workflow_uri,
        Some(&session_frontmatter),
        &existing_content,
        &context.config,
    )?;

    Ok(true)
}

fn build_dispatched_node_response(
    node: &PipelineNode,
    session_id: &str,
) -> Result<serde_json::Value> {
    match node {
        PipelineNode::Workflow { .. } => Ok(json!({
            "nodeId": node.id(),
            "sessionId": session_id,
            "workflowId": workflow_id_for_node(node)?,
        })),
        PipelineNode::Prompt { .. } | PipelineNode::SequentialThinking { .. } => {
            let agent = node.acp_client().ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline node '{}' of type '{}' does not have an agent",
                    node.id(),
                    node.node_type_name()
                ))
            })?;
            Ok(json!({
                "nodeId": node.id(),
                "sessionId": session_id,
                "type": node.node_type_name(),
                "agent": agent,
            }))
        }
        PipelineNode::Tool {
            tool_name,
            tool_args,
            ..
        } => {
            let agent = node.acp_client().ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline node '{}' of type '{}' does not have an agent",
                    node.id(),
                    node.node_type_name()
                ))
            })?;
            Ok(json!({
                "nodeId": node.id(),
                "sessionId": session_id,
                "type": node.node_type_name(),
                "agent": agent,
                "toolName": tool_name,
                "toolArgs": tool_args,
            }))
        }
    }
}

fn append_step_section(existing_content: &str, heading: &str, content: &str) -> String {
    let mut new_content = existing_content.to_string();
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&format!("\n## {heading}\n\n{content}\n"));
    new_content
}

fn update_activation_section_frontmatter(
    frontmatter: &mut Frontmatter,
    existing_content: &str,
    heading: &str,
    content: &str,
) {
    let section_summary: String = format!("{heading}: {content}").chars().take(256).collect();
    if existing_content.trim().is_empty() && !frontmatter.contains_key("first") {
        frontmatter.insert(
            "first".to_string(),
            YamlValue::String(section_summary.clone()),
        );
    }
    frontmatter.insert("last".to_string(), YamlValue::String(section_summary));
}

#[cfg(test)]
mod tests {
    use super::super::handler::pipeline;
    use super::test_support::*;
    use super::*;
    use crate::markdown::{Frontmatter, read_markdown, write_markdown};
    use crate::sessions::{pipeline_session_uri, workflow_session_uri};
    use crate::util::json::OutputFormat;
    use crate::workflow::pipeline::{EdgeType, NodeState};
    use serde_json::{Value, json};
    use serde_yaml::Value as YamlValue;
    use serial_test::serial;
    use std::collections::{BTreeMap, HashMap};

    fn node_status_string(status: &NodeStatus) -> &'static str {
        match status {
            NodeStatus::Pending => "pending",
            NodeStatus::Active => "active",
            NodeStatus::Completed => "completed",
            NodeStatus::Abandoned => "abandoned",
            NodeStatus::Ready => "ready",
            NodeStatus::Stuck => "stuck",
        }
    }

    fn seed_pipeline_session_with_node_sessions(
        context: &ToolContext,
        pipeline_id: &str,
        definition: PipelineDefinition,
        node_states: HashMap<String, NodeState>,
        max_concurrency: u64,
    ) {
        let node_states_yaml: BTreeMap<String, YamlValue> = node_states
            .iter()
            .map(|(key, value)| (key.clone(), serde_yaml::to_value(value).expect("yaml")))
            .collect();
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
        frontmatter.insert(
            "definition".to_string(),
            serde_yaml::to_value(&definition).expect("serialize definition"),
        );
        frontmatter.insert(
            "nodeStates".to_string(),
            YamlValue::Mapping(
                node_states_yaml
                    .iter()
                    .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                    .collect(),
            ),
        );
        frontmatter.insert(
            "maxConcurrency".to_string(),
            serde_yaml::to_value(max_concurrency).expect("yaml"),
        );

        write_markdown(
            &pipeline_session_uri(pipeline_id),
            Some(&frontmatter),
            "# Pipeline Session\n",
            &context.config,
        )
        .expect("write pipeline session");

        for node in &definition.nodes {
            let state = node_states.get(node.id()).expect("node state");
            let mut session_frontmatter = Frontmatter::new();
            session_frontmatter.insert(
                "title".to_string(),
                YamlValue::String(format!("Workflow Session {}", state.session_id)),
            );
            session_frontmatter.insert(
                "permalink".to_string(),
                YamlValue::String(state.session_id.clone()),
            );
            session_frontmatter.insert(
                "type".to_string(),
                YamlValue::String("workflow".to_string()),
            );
            session_frontmatter.insert(
                "status".to_string(),
                YamlValue::String(node_status_string(&state.status).to_string()),
            );
            session_frontmatter.insert(
                "pipelineId".to_string(),
                YamlValue::String(pipeline_id.to_string()),
            );
            session_frontmatter.insert(
                "nodeId".to_string(),
                YamlValue::String(node.id().to_string()),
            );
            if let Some(workflow_id) = node.workflow_id() {
                session_frontmatter.insert(
                    "workflowId".to_string(),
                    YamlValue::String(workflow_id.to_string()),
                );
            }

            write_markdown(
                &workflow_session_uri(&state.session_id),
                Some(&session_frontmatter),
                "# Workflow Session\n",
                &context.config,
            )
            .expect("write node session");
        }
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_transitions_ready_nodes_to_active() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600000";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600000-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        // Only root should be dispatched (child depends on root).
        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0]["nodeId"], "root");
        assert_eq!(dispatched[0]["sessionId"], "workflow-1735689600000-root");
        assert_eq!(dispatched[0]["workflowId"], "test-workflow-a");

        // Active should include root.
        let active = data
            .get("active")
            .and_then(Value::as_array)
            .expect("active array");
        assert!(active.contains(&Value::String("root".to_string())));

        // Completed should be empty.
        let completed = data
            .get("completed")
            .and_then(Value::as_array)
            .expect("completed array");
        assert!(completed.is_empty());

        // Remaining = 2 (root=active, child=pending — both non-completed).
        assert_eq!(data.get("remaining").and_then(Value::as_u64), Some(2));

        // Verify pipeline session was updated: root now active.
        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);
        assert_eq!(updated_states["child"].status, NodeStatus::Pending);

        // Verify `last` field was incremented.
        assert_eq!(pipeline_fm.get("last").and_then(YamlValue::as_u64), Some(1));

        // Verify workflow session was updated: status=active and first step rendered.
        let (wf_fm, wf_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600000-root"),
            &context.config,
        )
        .expect("read workflow session");
        let wf_fm = wf_fm.expect("workflow frontmatter");
        assert_eq!(
            wf_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert!(
            wf_content.contains("## Step 1/1"),
            "first step should be rendered in session body, got: {wf_content}"
        );
        assert!(
            wf_content.contains("step-1"),
            "rendered step should contain step ID, got: {wf_content}"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_respects_max_concurrency() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600100";

        // Create a 3-node pipeline (root→child, plus a sibling with no deps).
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                crate::workflow::pipeline::PipelineNode::Workflow {
                    id: "sibling".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                crate::workflow::pipeline::PipelineNode::Workflow {
                    id: "child".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![crate::workflow::pipeline::PipelineEdge {
                source: "root".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Success,
            }],
        };

        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600100-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "sibling".to_string(),
                NodeState {
                    session_id: "workflow-1735689600100-sibling".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600100-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);

        // Write custom pipeline session with maxConcurrency=1.
        let node_states_yaml: BTreeMap<String, YamlValue> = node_states
            .iter()
            .map(|(key, value)| (key.clone(), serde_yaml::to_value(value).expect("yaml")))
            .collect();
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
        frontmatter.insert(
            "definition".to_string(),
            serde_yaml::to_value(&definition).expect("yaml"),
        );
        frontmatter.insert(
            "nodeStates".to_string(),
            YamlValue::Mapping(
                node_states_yaml
                    .iter()
                    .map(|(key, value)| (YamlValue::String(key.clone()), value.clone()))
                    .collect(),
            ),
        );
        frontmatter.insert(
            "maxConcurrency".to_string(),
            serde_yaml::to_value(1u64).expect("yaml"),
        );

        write_markdown(
            &pipeline_session_uri(pipeline_id),
            Some(&frontmatter),
            "# Pipeline Session\n",
            &context.config,
        )
        .expect("write pipeline session");

        // Write workflow sessions for all nodes.
        for node in &definition.nodes {
            let state = node_states.get(node.id()).expect("state");
            let uri = workflow_session_uri(&state.session_id);
            let workflow_id = workflow_id_for_node(node).expect("workflow node");
            let mut wf_fm = crate::workflow::initial_workflow_frontmatter(
                &state.session_id,
                std::slice::from_ref(&workflow_id.to_string()),
                "pending",
            );
            wf_fm.insert(
                "pipelineId".to_string(),
                YamlValue::String(pipeline_id.to_string()),
            );
            wf_fm.insert(
                "nodeId".to_string(),
                YamlValue::String(node.id().to_string()),
            );
            wf_fm.insert(
                "workflowId".to_string(),
                YamlValue::String(workflow_id.to_string()),
            );
            write_markdown(&uri, Some(&wf_fm), "# Workflow Session\n", &context.config)
                .expect("write workflow session");
        }

        // Both root and sibling are ready (no deps), but maxConcurrency=1.
        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert_eq!(
            dispatched.len(),
            1,
            "maxConcurrency=1 should dispatch only 1 node"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_only_reports_nodes_transitioned_in_this_call() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600150";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                crate::workflow::pipeline::PipelineNode::Workflow {
                    id: "sibling".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![],
        };
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600150-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "sibling".to_string(),
                NodeState {
                    session_id: "workflow-1735689600150-sibling".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

        activate_workflow_node_session(&context, "test-workflow-a", "workflow-1735689600150-root")
            .expect("activate root workflow session");

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        assert_eq!(
            data.get("dispatched").and_then(Value::as_array),
            Some(&vec![json!({
                "nodeId": "sibling",
                "sessionId": "workflow-1735689600150-sibling",
                "workflowId": "test-workflow-b",
            })])
        );
        assert_eq!(
            data.get("active").and_then(Value::as_array),
            Some(&vec![
                Value::String("root".to_string()),
                Value::String("sibling".to_string()),
            ])
        );

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);
        assert_eq!(updated_states["sibling"].status, NodeStatus::Active);

        let (_, root_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600150-root"),
            &context.config,
        )
        .expect("read root workflow");
        assert_eq!(root_content.matches("## Step 1/1").count(), 1);
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_returns_empty_when_no_nodes_ready() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600200";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert!(
            dispatched.is_empty(),
            "no nodes should be dispatched when none are ready"
        );

        // Active list should still contain root.
        let active = data
            .get("active")
            .and_then(Value::as_array)
            .expect("active array");
        assert!(active.contains(&Value::String("root".to_string())));
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_returns_empty_when_at_max_concurrency() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600300";
        // Root is already active; child is pending but blocked by root (success edge).
        // maxConcurrency=1, 1 already active → 0 dispatch slots.
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600300-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600300-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 1);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert!(
            dispatched.is_empty(),
            "should dispatch nothing when at max concurrency"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_renders_first_step_into_session_body() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600400";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600400-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600400-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");

        // Verify the dispatched node's workflow session has the first step rendered.
        let (wf_fm, wf_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600400-root"),
            &context.config,
        )
        .expect("read workflow session");
        let wf_fm = wf_fm.expect("workflow frontmatter");

        // Status should be active.
        assert_eq!(
            wf_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );

        // Body should contain rendered step with H2 heading and step content.
        assert!(
            wf_content.contains("## Step 1/1"),
            "body must contain step heading, got: {wf_content}"
        );
        assert!(
            wf_content.contains("\"id\": \"step-1\"") || wf_content.contains("step-1"),
            "body must contain step-1 from workflow JSON, got: {wf_content}"
        );
        assert!(
            wf_content.contains("Run [[test-workflow-a]]"),
            "body must contain step description, got: {wf_content}"
        );

        // Un-dispatched child should still be pending with no step content.
        let (child_fm, child_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600400-child"),
            &context.config,
        )
        .expect("read child workflow session");
        let child_fm = child_fm.expect("child frontmatter");
        assert_eq!(
            child_fm.get("status").and_then(YamlValue::as_str),
            Some("pending")
        );
        assert!(
            !child_content.contains("## Step"),
            "child should not have steps rendered"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_persists_partial_progress_when_activation_fails() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600460";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                crate::workflow::pipeline::PipelineNode::Workflow {
                    id: "broken".to_string(),
                    workflow_id: "missing-workflow".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![],
        };
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600460-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "broken".to_string(),
                NodeState {
                    session_id: "workflow-1735689600460-broken".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

        let error = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect_err("dispatch should fail when a workflow asset is missing");
        assert!(
            error.to_string().contains("missing-workflow"),
            "unexpected error: {error}"
        );

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);
        assert_eq!(updated_states["broken"].status, NodeStatus::Pending);
        assert_eq!(pipeline_fm.get("last").and_then(YamlValue::as_u64), Some(1));

        let (root_fm, root_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600460-root"),
            &context.config,
        )
        .expect("read root workflow");
        let root_fm = root_fm.expect("root frontmatter");
        assert_eq!(
            root_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert_eq!(root_content.matches("## Step 1/1").count(), 1);
    }

    #[test]
    #[serial(config)]
    fn activate_workflow_node_session_skips_active_session_without_duplication() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600450";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600450-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600450-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let first_activation = activate_workflow_node_session(
            &context,
            "test-workflow-a",
            "workflow-1735689600450-root",
        )
        .expect("activate workflow session");
        assert!(first_activation, "pending session should activate");

        let second_activation = activate_workflow_node_session(
            &context,
            "test-workflow-a",
            "workflow-1735689600450-root",
        )
        .expect("skip already active session");
        assert!(
            !second_activation,
            "already active session should be skipped without rewriting"
        );

        let (wf_fm, wf_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600450-root"),
            &context.config,
        )
        .expect("read workflow session");
        let wf_fm = wf_fm.expect("workflow frontmatter");
        assert_eq!(
            wf_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert_eq!(
            wf_content.matches("## Step 1/1").count(),
            1,
            "step heading must only be rendered once"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_skips_already_active_workflow_sessions() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600475";
        let node_states = HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600475-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600475-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    output: None,
                },
            ),
        ]);
        seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

        let activated = activate_workflow_node_session(
            &context,
            "test-workflow-a",
            "workflow-1735689600475-root",
        )
        .expect("activate workflow session");
        assert!(activated, "setup should pre-activate workflow session");

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");

        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");
        assert!(
            dispatched.is_empty(),
            "pre-activated sessions should be skipped from dispatched response"
        );

        let active = data
            .get("active")
            .and_then(Value::as_array)
            .expect("active array");
        assert_eq!(active, &vec![Value::String("root".to_string())]);

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["root"].status, NodeStatus::Active);

        let (_, wf_content, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600475-root"),
            &context.config,
        )
        .expect("read workflow session");
        assert_eq!(
            wf_content.matches("## Step 1/1").count(),
            1,
            "dispatch retry must not append duplicate first steps"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_returns_prompt_node_metadata_without_workflow_activation() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600480";
        let definition = PipelineDefinition {
            nodes: vec![crate::workflow::pipeline::PipelineNode::Prompt {
                id: "prompt-node".to_string(),
                prompt: "Summarize the dispatch result".to_string(),
                agent: "prompt-agent".to_string(),
                tool_mode: "default".to_string(),
            }],
            edges: vec![],
        };
        let node_states = HashMap::from([(
            "prompt-node".to_string(),
            NodeState {
                session_id: "prompt-prompt-node-1735689600480".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                output: None,
            },
        )]);
        seed_pipeline_session_with_node_sessions(&context, pipeline_id, definition, node_states, 8);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");

        assert_eq!(
            dispatched,
            &vec![json!({
                "nodeId": "prompt-node",
                "sessionId": "prompt-prompt-node-1735689600480",
                "type": "prompt",
                "agent": "prompt-agent",
            })]
        );
        assert!(
            dispatched[0].get("workflowId").is_none(),
            "prompt nodes should not expose workflow metadata"
        );

        let (pipeline_fm, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
        let updated_states: HashMap<String, NodeState> =
            frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
        assert_eq!(updated_states["prompt-node"].status, NodeStatus::Active);

        let (session_fm, session_content, _) = read_markdown(
            &workflow_session_uri("prompt-prompt-node-1735689600480"),
            &context.config,
        )
        .expect("read prompt session");
        let session_fm = session_fm.expect("prompt frontmatter");
        assert_eq!(
            session_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert!(
            !session_content.contains("## Step"),
            "prompt nodes should not render workflow steps"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_returns_sequential_thinking_metadata_without_workflow_activation() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600490";
        let definition = PipelineDefinition {
            nodes: vec![
                crate::workflow::pipeline::PipelineNode::SequentialThinking {
                    id: "thought-node".to_string(),
                    topic: "Dispatch branching".to_string(),
                    total_thoughts: 3,
                    analysis_type: Some("architecture".to_string()),
                    agent: "thinking-agent".to_string(),
                },
            ],
            edges: vec![],
        };
        let node_states = HashMap::from([(
            "thought-node".to_string(),
            NodeState {
                session_id: "sequentialThinking-thought-node-1735689600490".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                output: None,
            },
        )]);
        seed_pipeline_session_with_node_sessions(&context, pipeline_id, definition, node_states, 8);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");

        assert_eq!(
            dispatched,
            &vec![json!({
                "nodeId": "thought-node",
                "sessionId": "sequentialThinking-thought-node-1735689600490",
                "type": "sequentialThinking",
                "agent": "thinking-agent",
            })]
        );
        assert!(
            dispatched[0].get("workflowId").is_none(),
            "sequential-thinking nodes should not expose workflow metadata"
        );

        let (session_fm, session_content, _) = read_markdown(
            &workflow_session_uri("sequentialThinking-thought-node-1735689600490"),
            &context.config,
        )
        .expect("read sequential-thinking session");
        let session_fm = session_fm.expect("sequential-thinking frontmatter");
        assert_eq!(
            session_fm.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert!(
            !session_content.contains("## Step"),
            "sequential-thinking nodes should not render workflow steps"
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_dispatch_returns_tool_metadata_without_workflow_activation() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let pipeline_id = "pipeline-1735689600495";
        let definition = PipelineDefinition {
            nodes: vec![crate::workflow::pipeline::PipelineNode::Tool {
                id: "tool-node".to_string(),
                tool_name: "ReadMemory".to_string(),
                tool_args: Some(json!({"identifier": "memory://build/test"})),
                agent: "prompt-agent".to_string(),
            }],
            edges: vec![],
        };
        let node_states = HashMap::from([(
            "tool-node".to_string(),
            NodeState {
                session_id: "tool-tool-node-1735689600495".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                output: None,
            },
        )]);
        seed_pipeline_session_with_node_sessions(&context, pipeline_id, definition, node_states, 8);

        let output = pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "dispatch": true }),
        )
        .expect("dispatch pipeline");
        let response: Value = serde_json::from_str(&output).expect("parse response");
        let data = response.get("data").expect("response data");
        let dispatched = data
            .get("dispatched")
            .and_then(Value::as_array)
            .expect("dispatched array");

        assert_eq!(
            dispatched,
            &vec![json!({
                "nodeId": "tool-node",
                "sessionId": "tool-tool-node-1735689600495",
                "type": "tool",
                "agent": "prompt-agent",
                "toolName": "ReadMemory",
                "toolArgs": {"identifier": "memory://build/test"},
            })]
        );
        assert!(
            dispatched[0].get("workflowId").is_none(),
            "tool nodes should not expose workflow metadata"
        );
    }
}
