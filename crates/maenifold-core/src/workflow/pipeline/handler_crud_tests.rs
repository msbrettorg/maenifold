use super::*;
use crate::markdown::delete_file;
use crate::sessions::workflow_session_uri;
use crate::util::json::OutputFormat;
use serial_test::serial;
use std::collections::HashMap;
use std::io::Write as _;

use super::super::PipelineNode;
use super::super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::super::view::view_pipeline_session;
use std::collections::BTreeSet;

pub(super) const TS: i64 = 1_735_689_600_000;

pub(super) fn workflow_node(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        display_label: None,
        workflow_id: "test-workflow-a".to_string(),
        acp_client: Some(format!("agent-{id}")),
        prompt: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
        join_policy: None,
    }
}

pub(super) fn tool_node(id: &str) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        display_label: None,
        tool_name: "GetConfig".to_string(),
        tool_args: None,
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        retry_policy: None,
        join_policy: None,
    }
}

pub(super) fn states(specs: &[(&str, NodeStatus, u32)]) -> HashMap<String, NodeState> {
    specs
        .iter()
        .map(|(node_id, status, attempts)| {
            (
                (*node_id).to_string(),
                NodeState {
                    session_id: format!("workflow-{TS}-{node_id}"),
                    status: status.clone(),
                    attempts: *attempts,
                    started_at: None,
                    completed_at: None,
                    acp_session_id: None,
                    thinking_session_id: None,
                    tool_provenance: None,
                    input_tokens: None,
                    output_tokens: None,
                    model: None,
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
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Completed, 1)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states.clone(), 1);

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

    let error =
        delete_pipeline(&context, session_id).expect_err("delete must refuse running pipeline");
    assert!(
        error
            .to_string()
            .contains("is running; cancel it before deleting"),
        "expected running-pipeline refusal, got: {error}"
    );
}

#[test]
#[serial]
fn create_pipeline_persists_effective_inputs_on_definition_and_session() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let response = create_pipeline(
        &context,
        &json!({
            "definition": {
                "nodes": [
                    { "id": "tool", "type": "tool", "toolName": "GetConfig" }
                ],
                "edges": [],
                "inputs": { "existing": 1 }
            },
            "inputs": {
                "existing": 2,
                "runtime": "value"
            }
        }),
    )
    .expect("create pipeline");
    let payload: Value = serde_json::from_str(&response).expect("json response");
    let pipeline_id = payload["data"]["pipelineId"].as_str().expect("pipeline id");
    let (_, frontmatter, _) = read_pipeline_session(&context, pipeline_id).expect("read pipeline");
    let definition: PipelineDefinition =
        super::super::session_io::pipeline_definition_from_frontmatter(&frontmatter)
            .expect("definition");
    let inputs: JsonMap<String, Value> = frontmatter_value(&frontmatter, "inputs").expect("inputs");

    assert_eq!(definition.nodes, vec![tool_node("tool")]);
    assert_eq!(
        definition.inputs,
        Some(serde_json::Map::from_iter([
            ("existing".to_string(), json!(2)),
            ("runtime".to_string(), json!("value")),
        ]))
    );
    assert_eq!(
        inputs,
        serde_json::Map::from_iter([
            ("existing".to_string(), json!(2)),
            ("runtime".to_string(), json!("value")),
        ])
    );
}

#[test]
#[serial]
fn view_pipeline_summary_returns_compact_node_array_for_a_50_node_fixture() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "p";
    let ids = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWX"
        .chars()
        .map(|ch| ch.to_string())
        .collect::<Vec<_>>();
    let definition = PipelineDefinition {
        nodes: ids.iter().map(|id| tool_node(id)).collect(),
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let specs = ids
        .iter()
        .map(|id| (id.clone(), NodeStatus::Ready, 0))
        .collect::<Vec<_>>();
    let state_specs = specs
        .iter()
        .map(|(id, status, attempts)| (id.as_str(), status.clone(), *attempts))
        .collect::<Vec<_>>();
    let mut node_states = states(&state_specs);
    node_states
        .get_mut(&ids[7])
        .expect("node state")
        .acp_session_id = Some("acp-session-7".to_string());
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 8);

    let response = view_pipeline(&context, session_id, true).expect("summary view");
    let payload: Value = serde_json::from_str(&response).expect("json response");
    let data = &payload["data"];

    assert!(
        response.len() < 4096,
        "summary was {} bytes",
        response.len()
    );
    assert_eq!(data["pipelineId"], session_id);
    assert_eq!(data["status"], "running");
    assert_eq!(data["running"], 0);
    let nodes = data["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 50);
    assert_eq!(data["nodes"][7]["acpSessionId"], "acp-session-7");
    let actual_ids = nodes
        .iter()
        .map(|node| node["nodeId"].as_str().expect("node id").to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_ids, ids.into_iter().collect::<BTreeSet<_>>());
}

#[test]
#[serial]
fn view_pipeline_without_summary_matches_existing_full_view_bytes() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-full-view-unchanged";
    let definition = PipelineDefinition {
        nodes: vec![workflow_node("node")],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let node_states = states(&[("node", NodeStatus::Pending, 0)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);

    let existing = view_pipeline_session(&context, session_id).expect("existing view");
    let routed = view_pipeline(&context, session_id, false).expect("routed view");

    assert_eq!(routed, existing);
}

#[test]
#[serial]
fn view_pipeline_with_summary_returns_compact_shape() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-summary-shape";
    let definition = PipelineDefinition {
        nodes: vec![tool_node("tool")],
        edges: vec![],
        inputs: None,
        input_schema: None,
        template_id: None,
    };
    let node_states = states(&[("tool", NodeStatus::Active, 2)]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 1);

    let response = view_pipeline(&context, session_id, true).expect("summary view");
    let payload: Value = serde_json::from_str(&response).expect("json response");
    let data = payload["data"].as_object().expect("data object");
    let node = data["nodes"].as_array().expect("nodes")[0]
        .as_object()
        .expect("node object");

    assert_eq!(data.len(), 4);
    assert_eq!(data["running"], 1);
    assert_eq!(node.len(), 5);
    assert_eq!(node["nodeId"], "tool");
    assert_eq!(node["type"], "tool");
    assert_eq!(node["status"], "active");
    assert_eq!(node["attempts"], 2);
    assert!(node["acpSessionId"].is_null());
}

#[path = "handler_crud_tests_extra.rs"]
mod extra;
#[path = "handler_crud_tests_more.rs"]
mod more;
#[path = "handler_crud_tests_runs.rs"]
mod runs;
