use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::workflow::pipeline::test_support::{
    seed_dispatch_pipeline, seed_pipeline_session_with_workflows,
};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

use super::super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
};
use super::support::{dispatch_data, pipeline_context, read_pipeline_states};

#[test]
#[serial(config)]
fn pipeline_dispatch_transitions_ready_nodes_to_active() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600000";
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600000-root", NodeStatus::Pending),
        node_state("child", "workflow-1735689600000-child", NodeStatus::Pending),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
    let data = dispatch_data(&context, pipeline_id);
    assert_eq!(data["dispatched"][0]["nodeId"], "root");
    assert_eq!(
        data["dispatched"][0]["sessionId"],
        "workflow-1735689600000-root"
    );
    assert_eq!(data["dispatched"][0]["workflowId"], "test-workflow-a");
    assert!(
        data["active"]
            .as_array()
            .expect("active")
            .contains(&Value::String("root".into()))
    );
    assert!(data["completed"].as_array().expect("completed").is_empty());
    assert_eq!(data.get("remaining").and_then(Value::as_u64), Some(2));
    let updated_states = read_pipeline_states(&context, pipeline_id);
    assert_eq!(updated_states["root"].status, NodeStatus::Active);
    assert_eq!(updated_states["child"].status, NodeStatus::Pending);
    let (pipeline_fm, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    assert_eq!(
        pipeline_fm
            .expect("pipeline frontmatter")
            .get("last")
            .and_then(YamlValue::as_u64),
        Some(1)
    );
}

#[test]
#[serial(config)]
fn pipeline_dispatch_respects_max_concurrency() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600100";
    let definition = PipelineDefinition {
        nodes: vec![
            workflow_node("root", "test-workflow-a"),
            workflow_node("sibling", "test-workflow-b"),
            workflow_node("child", "test-workflow-b"),
        ],
        edges: vec![PipelineEdge {
            source: "root".into(),
            target: "child".into(),
            edge_type: EdgeType::Success,
            mapping: None,
        }],
    };
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600100-root", NodeStatus::Pending),
        node_state(
            "sibling",
            "workflow-1735689600100-sibling",
            NodeStatus::Pending,
        ),
        node_state("child", "workflow-1735689600100-child", NodeStatus::Pending),
    ]);
    seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 1);
    assert_eq!(
        dispatch_data(&context, pipeline_id)["dispatched"]
            .as_array()
            .expect("dispatched")
            .len(),
        1
    );
}

#[test]
#[serial(config)]
fn pipeline_dispatch_only_reports_nodes_transitioned_in_this_call() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600150";
    let definition = PipelineDefinition {
        nodes: vec![
            workflow_node("root", "test-workflow-a"),
            workflow_node("sibling", "test-workflow-b"),
        ],
        edges: vec![],
    };
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600150-root", NodeStatus::Pending),
        node_state(
            "sibling",
            "workflow-1735689600150-sibling",
            NodeStatus::Pending,
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);
    super::super::dispatch_workflow::activate_workflow_node_session(
        &context,
        "test-workflow-a",
        "workflow-1735689600150-root",
    )
    .expect("activate root");
    let data = dispatch_data(&context, pipeline_id);
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
            Value::String("root".into()),
            Value::String("sibling".into())
        ])
    );
}

#[test]
#[serial(config)]
fn pipeline_dispatch_returns_empty_when_no_nodes_ready() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600200";
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600200-root", NodeStatus::Active),
        node_state("child", "workflow-1735689600200-child", NodeStatus::Pending),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
    let data = dispatch_data(&context, pipeline_id);
    assert!(
        data["dispatched"]
            .as_array()
            .expect("dispatched")
            .is_empty()
    );
    assert!(
        data["active"]
            .as_array()
            .expect("active")
            .contains(&Value::String("root".into()))
    );
}

#[test]
#[serial(config)]
fn pipeline_dispatch_returns_empty_when_at_max_concurrency() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600300";
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600300-root", NodeStatus::Active),
        node_state("child", "workflow-1735689600300-child", NodeStatus::Pending),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 1);
    assert!(
        dispatch_data(&context, pipeline_id)["dispatched"]
            .as_array()
            .expect("dispatched")
            .is_empty()
    );
}

#[test]
#[serial(config)]
fn pipeline_dispatch_persists_partial_progress_when_activation_fails() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600460";
    let definition = PipelineDefinition {
        nodes: vec![
            workflow_node("root", "test-workflow-a"),
            workflow_node("broken", "missing-workflow"),
        ],
        edges: vec![],
    };
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600460-root", NodeStatus::Pending),
        node_state(
            "broken",
            "workflow-1735689600460-broken",
            NodeStatus::Pending,
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);
    let error = super::super::handler::pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "dispatch": true }),
    )
    .expect_err("dispatch should fail");
    assert!(
        error.to_string().contains("missing-workflow"),
        "unexpected error: {error}"
    );
    let updated_states = read_pipeline_states(&context, pipeline_id);
    assert_eq!(updated_states["root"].status, NodeStatus::Active);
    assert_eq!(updated_states["broken"].status, NodeStatus::Pending);
}

fn workflow_node(id: &str, workflow_id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.into(),
        workflow_id: workflow_id.into(),
        acp_client: None,
        prompt: None,
    }
}

fn node_state(id: &str, session_id: &str, status: NodeStatus) -> (String, NodeState) {
    (
        id.into(),
        NodeState {
            session_id: session_id.into(),
            status,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )
}
