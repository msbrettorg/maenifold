use super::handler::pipeline;
use super::handler_tests::set_pipeline_status;
use super::session_io::frontmatter_value;
use super::test_support::*;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

fn wf(id: &str, workflow_id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.into(),
        workflow_id: workflow_id.into(),
        acp_client: None,
        prompt: None,
        timeout: None,
    }
}
fn edge(source: &str, target: &str, edge_type: EdgeType) -> PipelineEdge {
    PipelineEdge {
        source: source.into(),
        target: target.into(),
        edge_type,
        mapping: None,
    }
}
fn state(pipeline_id: &str, node_id: &str, status: NodeStatus) -> (String, NodeState) {
    (
        node_id.into(),
        NodeState {
            session_id: format!("workflow-{pipeline_id}-{node_id}"),
            status,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )
}
fn seed(
    context: &ToolContext,
    pipeline_id: &str,
    definition: PipelineDefinition,
    states: HashMap<String, NodeState>,
    status: &str,
) {
    seed_pipeline_session_with_workflows(context, pipeline_id, definition, states, 2);
    set_pipeline_status(context, pipeline_id, status);
}
fn data(context: &ToolContext, payload: serde_json::Value) -> Value {
    serde_json::from_str::<Value>(&pipeline(context, &payload).expect("pipeline")).expect("json")
        ["data"]
        .clone()
}
fn read(
    context: &ToolContext,
    pipeline_id: &str,
) -> (PipelineDefinition, HashMap<String, NodeState>, String) {
    let (fm, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read");
    let fm = fm.expect("frontmatter");
    (
        frontmatter_value(&fm, "definition").expect("definition"),
        frontmatter_value(&fm, "nodeStates").expect("states"),
        fm.get("status")
            .and_then(YamlValue::as_str)
            .unwrap_or("")
            .into(),
    )
}
#[test]
#[serial(config)]
fn g_01_graft_single_node_onto_completed_pipeline() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-01";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    let d = data(
        &c,
        json!({"sessionId": id, "graft": {"nodes":[{"id":"child","workflowId":"test-workflow-b"}],"edges":[{"source":"root","target":"child"}]}}),
    );
    assert_eq!(d["ready"], json!(["child"]));
    assert_eq!(read(&c, id).0.nodes.len(), 2);
}
#[test]
#[serial(config)]
fn g_02_graft_multiple_nodes_with_edges() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-02";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    let d = data(
        &c,
        json!({"sessionId": id, "graft": {"nodes":[{"id":"a","workflowId":"test-workflow-b"},{"id":"b","workflowId":"test-workflow-a"}],"edges":[{"source":"root","target":"a"},{"source":"a","target":"b"}]}}),
    );
    assert_eq!(d["ready"], json!(["a"]));
    assert_eq!(read(&c, id).0.edges.len(), 2);
}
#[test]
#[serial(config)]
fn g_03_graft_onto_paused_pipeline_preserves_paused_status() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-03";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "paused",
    );
    data(
        &c,
        json!({"sessionId": id, "graft": {"nodes":[{"id":"child","workflowId":"test-workflow-b"}],"edges":[{"source":"root","target":"child"}]}}),
    );
    assert_eq!(read(&c, id).2, "paused");
}
#[test]
#[serial(config)]
fn g_04_graft_onto_stuck_pipeline_is_allowed() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-04";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Stuck)]),
        "stuck",
    );
    let d = data(
        &c,
        json!({"sessionId": id, "graft": {"nodes":[{"id":"recovery","workflowId":"test-workflow-b"}],"edges":[{"source":"root","target":"recovery","type":"always"}]}}),
    );
    assert_eq!(d["nodes"].as_array().expect("nodes").len(), 2);
}
#[test]
#[serial(config)]
fn g_05_graft_all_four_node_types() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-05";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    let d = data(
        &c,
        json!({"sessionId": id, "graft": {"nodes":[{"id":"flow","workflowId":"test-workflow-b"},{"id":"prompt","type":"prompt","prompt":"Summarize","agent":"agent-a"},{"id":"think","type":"sequentialThinking","topic":"Review","totalThoughts":2,"agent":"agent-b"},{"id":"tool","type":"tool","toolName":"GetConfig","agent":"agent-c"}],"edges":[{"source":"root","target":"flow"},{"source":"flow","target":"prompt"},{"source":"prompt","target":"think"},{"source":"think","target":"tool"}]}}),
    );
    assert_eq!(d["nodes"].as_array().expect("nodes").len(), 5);
}
#[test]
#[serial(config)]
fn g_06_graft_completed_dependency_is_immediately_ready() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-06";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    assert_eq!(
        data(
            &c,
            json!({"sessionId": id, "graft": {"nodes":[{"id":"downstream","workflowId":"test-workflow-b"}],"edges":[{"source":"root","target":"downstream"}]}})
        )["ready"],
        json!(["downstream"])
    );
}
#[test]
#[serial(config)]
fn g_07_graft_rejected_on_running_pipeline() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-07";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Active)]),
        "active",
    );
    assert!(pipeline(&c, &json!({"sessionId": id, "graft": {"nodes":[{"id":"x","workflowId":"test-workflow-b"}],"edges":[]}})).expect_err("err").to_string().contains("running pipelines"));
}
#[test]
#[serial(config)]
fn g_08_graft_rejected_on_active_pipeline_with_pending_node() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-08";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Pending)]),
        "active",
    );
    assert!(pipeline(&c, &json!({"sessionId": id, "graft": {"nodes":[{"id":"x","workflowId":"test-workflow-b"}],"edges":[]}})).expect_err("err").to_string().contains("running pipelines"));
}
#[test]
#[serial(config)]
fn g_09_graft_with_duplicate_node_id_rejected() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-09";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    assert!(pipeline(&c, &json!({"sessionId": id, "graft": {"nodes":[{"id":"root","workflowId":"test-workflow-b"}],"edges":[]}})).expect_err("err").to_string().contains("duplicate node ID"));
}
#[test]
#[serial(config)]
fn g_10_graft_creating_cycle_rejected() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-10";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![
                wf("root", "test-workflow-a"),
                wf("child", "test-workflow-b"),
            ],
            edges: vec![edge("root", "child", EdgeType::Success)],
        },
        HashMap::from([
            state(id, "root", NodeStatus::Completed),
            state(id, "child", NodeStatus::Completed),
        ]),
        "completed",
    );
    assert!(pipeline(&c, &json!({"sessionId": id, "graft": {"nodes":[{"id":"loop","workflowId":"test-workflow-a"}],"edges":[{"source":"child","target":"loop"},{"source":"loop","target":"root"}]}})).expect_err("err").to_string().contains("cycle"));
}
#[test]
#[serial(config)]
fn g_11_graft_edge_referencing_unknown_node_rejected() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-11";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    assert!(pipeline(&c, &json!({"sessionId": id, "graft": {"nodes":[{"id":"child","workflowId":"test-workflow-b"}],"edges":[{"source":"ghost","target":"child"}]}})).expect_err("err").to_string().contains("unknown source node"));
}
#[test]
#[serial(config)]
fn g_12_graft_failure_is_atomic() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-12";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    let _ = pipeline(&c, &json!({"sessionId": id, "graft": {"nodes":[{"id":"fresh","workflowId":"test-workflow-b"},{"id":"root","workflowId":"test-workflow-a"}],"edges":[]}})).expect_err("err");
    let (_, states, _) = read(&c, id);
    assert_eq!(states.len(), 1);
    assert!(!states.contains_key("fresh"));
}
#[test]
#[serial(config)]
fn g_13_graft_with_zero_new_edges_allows_disconnected_node() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-13";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    assert_eq!(
        data(
            &c,
            json!({"sessionId": id, "graft": {"nodes":[{"id":"solo","workflowId":"test-workflow-b"}],"edges":[]}})
        )["ready"],
        json!(["solo"])
    );
}
#[test]
#[serial(config)]
fn g_14_graft_backward_edge_new_to_existing_is_allowed() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-14";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    let d = data(
        &c,
        json!({"sessionId": id, "graft": {"nodes":[{"id":"prep","workflowId":"test-workflow-b"}],"edges":[{"source":"prep","target":"root"}]}}),
    );
    assert_eq!(d["ready"], json!(["prep"]));
}
#[test]
#[serial(config)]
fn g_15_double_graft_succeeds_sequentially() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    let id = "g-15";
    seed(
        &c,
        id,
        PipelineDefinition {
            nodes: vec![wf("root", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state(id, "root", NodeStatus::Completed)]),
        "completed",
    );
    data(
        &c,
        json!({"sessionId": id, "graft": {"nodes":[{"id":"a","workflowId":"test-workflow-b"}],"edges":[{"source":"root","target":"a"}]}}),
    );
    data(&c, json!({"sessionId": id, "dispatch": true}));
    data(
        &c,
        json!({"sessionId": id, "nodeId": "a", "status": "completed", "conclusion": "a [[pipeline]]"}),
    );
    let d = data(
        &c,
        json!({"sessionId": id, "graft": {"nodes":[{"id":"b","workflowId":"test-workflow-a"}],"edges":[{"source":"a","target":"b"}]}}),
    );
    assert_eq!(d["ready"], json!(["b"]));
    assert_eq!(read(&c, id).0.nodes.len(), 3);
}
