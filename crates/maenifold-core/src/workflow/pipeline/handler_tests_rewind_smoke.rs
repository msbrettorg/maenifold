use super::handler::pipeline;
use super::session_io::frontmatter_value;
use super::supervisor::supervisor_spawn_test_task;
use super::supervisor_test_support::reset_supervisor;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use crate::markdown::{file_exists, read_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;

fn wf(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        workflow_id: if id.ends_with('b') || id.ends_with('d') {
            "test-workflow-b"
        } else {
            "test-workflow-a"
        }
        .to_string(),
        acp_client: Some("workflow-agent".to_string()),
        prompt: None,
        timeout: None,
    }
}

fn edge(source: &str, target: &str) -> PipelineEdge {
    PipelineEdge {
        source: source.to_string(),
        target: target.to_string(),
        edge_type: EdgeType::Success,
        mapping: None,
    }
}

fn state(id: &str, status: NodeStatus, attempts: u32) -> (String, NodeState) {
    (
        id.to_string(),
        NodeState {
            session_id: format!("workflow-smoke-{id}"),
            status,
            attempts,
            thinking_session_id: Some(format!("thinking-{id}")),
            output: Some(format!("output-{id}")),
        },
    )
}

fn data(output: String) -> Value {
    serde_json::from_str::<Value>(&output).expect("parse response")["data"].clone()
}

fn node<'a>(data: &'a Value, id: &str) -> &'a Value {
    data["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["nodeId"] == id)
        .expect("node")
}

#[test]
#[serial(config)]
fn rw_01_rewind_single_node_resets_to_pending() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-01",
        PipelineDefinition {
            nodes: vec![wf("a"), wf("b"), wf("c")],
            edges: vec![edge("a", "b"), edge("b", "c")],
        },
        HashMap::from([
            state("a", NodeStatus::Completed, 1),
            state("b", NodeStatus::Completed, 2),
            state("c", NodeStatus::Completed, 3),
        ]),
        2,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-01", "rewind": { "nodeIds": ["b"] } }),
        )
        .expect("rewind"),
    );
    assert_eq!(node(&data, "a")["status"], "completed");
    assert_eq!(node(&data, "b")["status"], "pending");
}

#[test]
#[serial(config)]
fn rw_02_rewind_cascades_to_all_downstream_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-02",
        PipelineDefinition {
            nodes: vec![wf("a"), wf("b"), wf("c")],
            edges: vec![edge("a", "b"), edge("b", "c")],
        },
        HashMap::from([
            state("a", NodeStatus::Completed, 1),
            state("b", NodeStatus::Completed, 2),
            state("c", NodeStatus::Completed, 3),
        ]),
        2,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-02", "rewind": { "nodeIds": ["b"] } }),
        )
        .expect("rewind"),
    );
    assert_eq!(node(&data, "c")["status"], "pending");
    assert_eq!(node(&data, "c")["attempts"], 0);
}

#[test]
#[serial(config)]
fn rw_03_rewind_clears_output_and_thinking_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-03",
        PipelineDefinition {
            nodes: vec![wf("a"), wf("b")],
            edges: vec![edge("a", "b")],
        },
        HashMap::from([
            state("a", NodeStatus::Completed, 1),
            state("b", NodeStatus::Completed, 2),
        ]),
        2,
    );
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-rw-03", "rewind": { "nodeIds": ["a"] } }),
    )
    .expect("rewind");
    let (fm, _, _) = read_markdown(&pipeline_session_uri("pipeline-rw-03"), &context.config)
        .expect("read pipeline");
    let states: HashMap<String, NodeState> =
        frontmatter_value(&fm.expect("fm"), "nodeStates").expect("states");
    assert!(states["a"].thinking_session_id.is_none() && states["a"].output.is_none());
    assert!(states["b"].thinking_session_id.is_none() && states["b"].output.is_none());
}

#[test]
#[serial(config)]
fn rw_04_rewind_resets_attempts_to_zero() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-04",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Completed, 9)]),
        1,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-04", "rewind": { "nodeIds": ["a"] } }),
        )
        .expect("rewind"),
    );
    assert_eq!(node(&data, "a")["attempts"], 0);
}

#[test]
#[serial(config)]
fn rw_05_rewind_allocates_new_workflow_session_ids() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-05",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Completed, 1)]),
        1,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-05", "rewind": { "nodeIds": ["a"] } }),
        )
        .expect("rewind"),
    );
    let session_id = node(&data, "a")["sessionId"].as_str().expect("session");
    assert_ne!(session_id, "workflow-smoke-a");
    assert!(file_exists(&workflow_session_uri(session_id), &context.config).expect("exists"));
}

#[test]
#[serial(config)]
fn rw_06_rewind_multiple_root_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-06",
        PipelineDefinition {
            nodes: vec![wf("a"), wf("b"), wf("c"), wf("d")],
            edges: vec![edge("a", "c"), edge("b", "d")],
        },
        HashMap::from([
            state("a", NodeStatus::Completed, 1),
            state("b", NodeStatus::Completed, 1),
            state("c", NodeStatus::Completed, 1),
            state("d", NodeStatus::Completed, 1),
        ]),
        4,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-06", "rewind": { "nodeIds": ["a", "b"] } }),
        )
        .expect("rewind"),
    );
    for id in ["a", "b", "c", "d"] {
        assert_eq!(node(&data, id)["status"], "pending");
    }
}

#[test]
#[serial(config)]
fn rw_07_rewind_while_supervisor_running_is_rejected() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-07",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Completed, 1)]),
        1,
    );
    supervisor_spawn_test_task("pipeline-rw-07").expect("spawn");
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-rw-07", "rewind": { "nodeIds": ["a"] } }),
    )
    .expect_err("rewind should fail");
    assert!(
        error
            .to_string()
            .contains("pause or cancel before rewinding")
    );
}

#[test]
#[serial(config)]
fn rw_08_rewind_nonexistent_node_is_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-08",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Completed, 1)]),
        1,
    );
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-rw-08", "rewind": { "nodeIds": ["missing"] } }),
    )
    .expect_err("missing node");
    assert!(error.to_string().contains("Unknown node ID: missing"));
}

#[test]
#[serial(config)]
fn rw_09_rewind_in_diamond_dag_cascades_correctly() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-09",
        PipelineDefinition {
            nodes: vec![wf("a"), wf("b"), wf("c"), wf("d")],
            edges: vec![
                edge("a", "b"),
                edge("a", "c"),
                edge("b", "d"),
                edge("c", "d"),
            ],
        },
        HashMap::from([
            state("a", NodeStatus::Completed, 1),
            state("b", NodeStatus::Completed, 1),
            state("c", NodeStatus::Completed, 1),
            state("d", NodeStatus::Completed, 1),
        ]),
        4,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-09", "rewind": { "nodeIds": ["b"] } }),
        )
        .expect("rewind"),
    );
    assert_eq!(node(&data, "a")["status"], "completed");
    assert_eq!(node(&data, "b")["status"], "pending");
    assert_eq!(node(&data, "c")["status"], "completed");
    assert_eq!(node(&data, "d")["status"], "pending");
}

#[test]
#[serial(config)]
fn rw_10_rewind_all_nodes_in_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-10",
        PipelineDefinition {
            nodes: vec![wf("a"), wf("b"), wf("c")],
            edges: vec![edge("a", "b"), edge("b", "c")],
        },
        HashMap::from([
            state("a", NodeStatus::Completed, 1),
            state("b", NodeStatus::Abandoned, 2),
            state("c", NodeStatus::Stuck, 3),
        ]),
        3,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-10", "rewind": { "nodeIds": ["a", "b", "c"] } }),
        )
        .expect("rewind"),
    );
    for id in ["a", "b", "c"] {
        assert_eq!(node(&data, id)["status"], "pending");
    }
}

#[test]
#[serial(config)]
fn rw_11_rewind_pending_node_recreates_pending_session() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-11",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Pending, 4)]),
        1,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-11", "rewind": { "nodeIds": ["a"] } }),
        )
        .expect("rewind"),
    );
    assert_eq!(node(&data, "a")["status"], "pending");
    assert_ne!(node(&data, "a")["sessionId"], "workflow-smoke-a");
}

#[test]
#[serial(config)]
fn rw_12_rewind_active_node_resets_it_to_pending() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-rw-12",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Active, 2)]),
        1,
    );
    let data = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-rw-12", "rewind": { "nodeIds": ["a"] } }),
        )
        .expect("rewind"),
    );
    assert_eq!(node(&data, "a")["status"], "pending");
    assert_eq!(node(&data, "a")["attempts"], 0);
}
