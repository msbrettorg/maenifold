use super::handler::pipeline;
use super::handler_tests::set_pipeline_status;
use super::session_io::frontmatter_value;
use super::test_support::*;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use crate::markdown::{file_exists, read_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
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
fn state(
    pipeline_id: &str,
    node_id: &str,
    status: NodeStatus,
    output: Option<&str>,
) -> (String, NodeState) {
    (
        node_id.into(),
        NodeState {
            session_id: format!("workflow-{pipeline_id}-{node_id}"),
            status,
            attempts: 0,
            thinking_session_id: None,
            output: output.map(str::to_string),
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
fn def(context: &ToolContext, pipeline_id: &str) -> PipelineDefinition {
    let (fm, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read");
    frontmatter_value(&fm.expect("frontmatter"), "definition").expect("definition")
}

#[test]
#[serial(config)]
fn j_01_join_two_completed_simple_pipelines() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-01-t",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("j-01-t", "a", NodeStatus::Completed, None)]),
        "completed",
    );
    seed(
        &c,
        "j-01-s",
        PipelineDefinition {
            nodes: vec![wf("b", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([state("j-01-s", "b", NodeStatus::Completed, None)]),
        "completed",
    );
    assert_eq!(data(&c, json!({"sessionId":"j-01-t","join":{"sourceSessionId":"j-01-s","edges":[{"source":"a","target":"b"}]}}))["nodes"].as_array().expect("nodes").len(), 2);
}
#[test]
#[serial(config)]
fn j_02_join_preserves_node_states_from_both_pipelines() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-02-t",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("j-02-t", "a", NodeStatus::Abandoned, None)]),
        "completed",
    );
    seed(
        &c,
        "j-02-s",
        PipelineDefinition {
            nodes: vec![wf("b", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([state("j-02-s", "b", NodeStatus::Stuck, None)]),
        "stuck",
    );
    let d = data(
        &c,
        json!({"sessionId":"j-02-t","join":{"sourceSessionId":"j-02-s","edges":[]}}),
    );
    assert!(d["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .any(|n| n["nodeId"] == "a" && n["status"] == "abandoned"));
    assert!(d["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .any(|n| n["nodeId"] == "b" && n["status"] == "stuck"));
}
#[test]
#[serial(config)]
fn j_03_join_with_multiple_bridge_edges() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-03-t",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a"), wf("b", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([
            state("j-03-t", "a", NodeStatus::Completed, None),
            state("j-03-t", "b", NodeStatus::Completed, None),
        ]),
        "completed",
    );
    seed(
        &c,
        "j-03-s",
        PipelineDefinition {
            nodes: vec![wf("c", "test-workflow-a"), wf("d", "test-workflow-b")],
            edges: vec![edge("c", "d", EdgeType::Success)],
        },
        HashMap::from([
            state("j-03-s", "c", NodeStatus::Completed, None),
            state("j-03-s", "d", NodeStatus::Completed, None),
        ]),
        "completed",
    );
    assert_eq!(
        data(
            &c,
            json!({"sessionId":"j-03-t","join":{"sourceSessionId":"j-03-s","edges":[{"source":"a","target":"c"},{"source":"b","target":"d"}]}})
        )["ready"],
        json!([])
    );
}
#[test]
#[serial(config)]
fn j_04_join_rejected_when_target_pipeline_running() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-04-t",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        // Ready is a valid running status (treated as "running" in handler_surgery_join.rs)
        // — used here to verify join rejects any non-terminal target, not just Active
        HashMap::from([state("j-04-t", "a", NodeStatus::Ready, None)]),
        "active",
    );
    seed(
        &c,
        "j-04-s",
        PipelineDefinition {
            nodes: vec![wf("b", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([state("j-04-s", "b", NodeStatus::Completed, None)]),
        "completed",
    );
    assert!(pipeline(
        &c,
        &json!({"sessionId":"j-04-t","join":{"sourceSessionId":"j-04-s","edges":[]}})
    )
    .expect_err("err")
    .to_string()
    .contains("target pipeline"));
}
#[test]
#[serial(config)]
fn j_05_join_rejected_when_source_pipeline_running() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-05-t",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("j-05-t", "a", NodeStatus::Completed, None)]),
        "completed",
    );
    seed(
        &c,
        "j-05-s",
        PipelineDefinition {
            nodes: vec![wf("b", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([state("j-05-s", "b", NodeStatus::Active, None)]),
        "active",
    );
    assert!(pipeline(
        &c,
        &json!({"sessionId":"j-05-t","join":{"sourceSessionId":"j-05-s","edges":[]}})
    )
    .expect_err("err")
    .to_string()
    .contains("source pipeline"));
}
#[test]
#[serial(config)]
fn j_06_join_rejected_on_conflicting_node_ids() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    for id in ["j-06-t", "j-06-s"] {
        seed(
            &c,
            id,
            PipelineDefinition {
                nodes: vec![wf("dup", "test-workflow-a")],
                edges: vec![],
            },
            HashMap::from([state(id, "dup", NodeStatus::Completed, None)]),
            "completed",
        );
    }
    assert!(pipeline(
        &c,
        &json!({"sessionId":"j-06-t","join":{"sourceSessionId":"j-06-s","edges":[]}})
    )
    .expect_err("err")
    .to_string()
    .contains("conflicting node IDs"));
}
#[test]
#[serial(config)]
fn j_07_join_creating_cycle_rejected() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-07-t",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a"), wf("b", "test-workflow-b")],
            edges: vec![edge("a", "b", EdgeType::Success)],
        },
        HashMap::from([
            state("j-07-t", "a", NodeStatus::Completed, None),
            state("j-07-t", "b", NodeStatus::Completed, None),
        ]),
        "completed",
    );
    seed(
        &c,
        "j-07-s",
        PipelineDefinition {
            nodes: vec![wf("c", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("j-07-s", "c", NodeStatus::Completed, None)]),
        "completed",
    );
    assert!(pipeline(&c, &json!({"sessionId":"j-07-t","join":{"sourceSessionId":"j-07-s","edges":[{"source":"b","target":"c"},{"source":"c","target":"a"}]}})).expect_err("err").to_string().contains("cycle"));
}
#[test]
#[serial(config)]
fn j_08_join_with_duplicate_bridge_edge_rejected() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-08-t",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("j-08-t", "a", NodeStatus::Completed, None)]),
        "completed",
    );
    seed(
        &c,
        "j-08-s",
        PipelineDefinition {
            nodes: vec![wf("b", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([state("j-08-s", "b", NodeStatus::Completed, None)]),
        "completed",
    );
    assert!(pipeline(&c, &json!({"sessionId":"j-08-t","join":{"sourceSessionId":"j-08-s","edges":[{"source":"a","target":"b"},{"source":"a","target":"b"}]}})).expect_err("err").to_string().contains("duplicate edge"));
}
#[test]
#[serial(config)]
fn j_09_join_nonexistent_source_pipeline() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-09-t",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("j-09-t", "a", NodeStatus::Completed, None)]),
        "completed",
    );
    assert!(pipeline(
        &c,
        &json!({"sessionId":"j-09-t","join":{"sourceSessionId":"missing","edges":[]}})
    )
    .is_err());
}
#[test]
#[serial(config)]
fn j_10_join_pipeline_with_itself_rejected() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "j-10",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("j-10", "a", NodeStatus::Completed, None)]),
        "completed",
    );
    assert!(pipeline(
        &c,
        &json!({"sessionId":"j-10","join":{"sourceSessionId":"j-10","edges":[]}})
    )
    .expect_err("err")
    .to_string()
    .contains("conflicting node IDs"));
}
#[test]
#[serial(config)]
fn j_11_sequential_join_of_three_pipelines() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    for (pid, nid, wf_id) in [
        ("j-11-a", "a", "test-workflow-a"),
        ("j-11-b", "b", "test-workflow-b"),
        ("j-11-c", "c", "test-workflow-a"),
    ] {
        seed(
            &c,
            pid,
            PipelineDefinition {
                nodes: vec![wf(nid, wf_id)],
                edges: vec![],
            },
            HashMap::from([state(pid, nid, NodeStatus::Completed, None)]),
            "completed",
        );
    }
    data(
        &c,
        json!({"sessionId":"j-11-a","join":{"sourceSessionId":"j-11-b","edges":[{"source":"a","target":"b"}]}}),
    );
    data(
        &c,
        json!({"sessionId":"j-11-a","join":{"sourceSessionId":"j-11-c","edges":[{"source":"b","target":"c"}]}}),
    );
    assert_eq!(def(&c, "j-11-a").nodes.len(), 3);
}
#[test]
#[serial(config)]
fn f_01_fork_from_middle_node_includes_all_downstream() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-01",
        PipelineDefinition {
            nodes: vec![
                wf("root", "test-workflow-a"),
                wf("mid", "test-workflow-b"),
                wf("l", "test-workflow-a"),
                wf("r", "test-workflow-b"),
            ],
            edges: vec![
                edge("root", "mid", EdgeType::Success),
                edge("mid", "l", EdgeType::Success),
                edge("mid", "r", EdgeType::Always),
            ],
        },
        HashMap::from([
            state("f-01", "root", NodeStatus::Completed, None),
            state("f-01", "mid", NodeStatus::Active, None),
            state("f-01", "l", NodeStatus::Completed, None),
            state("f-01", "r", NodeStatus::Stuck, None),
        ]),
        "active",
    );
    let d = data(
        &c,
        json!({"sessionId":"f-01","fork":{"fromNodeId":"mid","newSessionId":"f-01-new"}}),
    );
    assert_eq!(d["nodes"].as_array().expect("nodes").len(), 3);
    assert_eq!(d["ready"], json!(["fork-f-01-new-mid"]));
}
#[test]
#[serial(config)]
fn f_02_fork_from_leaf_creates_single_node_pipeline() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-02",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a"), wf("leaf", "test-workflow-b")],
            edges: vec![edge("a", "leaf", EdgeType::Success)],
        },
        HashMap::from([
            state("f-02", "a", NodeStatus::Completed, None),
            state("f-02", "leaf", NodeStatus::Completed, None),
        ]),
        "completed",
    );
    assert_eq!(
        data(
            &c,
            json!({"sessionId":"f-02","fork":{"fromNodeId":"leaf","newSessionId":"f-02-new"}})
        )["nodes"]
            .as_array()
            .expect("nodes")
            .len(),
        1
    );
}
#[test]
#[serial(config)]
fn f_03_fork_prefixes_node_ids_and_creates_sessions() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-03",
        PipelineDefinition {
            nodes: vec![wf("solo", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("f-03", "solo", NodeStatus::Completed, None)]),
        "completed",
    );
    let d = data(
        &c,
        json!({"sessionId":"f-03","fork":{"fromNodeId":"solo","newSessionId":"f-03-new"}}),
    );
    let sid = d["nodes"][0]["sessionId"].as_str().expect("sid");
    assert_eq!(d["nodes"][0]["nodeId"].as_str(), Some("fork-f-03-new-solo"));
    assert!(file_exists(&workflow_session_uri(sid), &c.config).expect("exists"));
}
#[test]
#[serial(config)]
fn f_04_fork_inherits_predecessor_context() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-04",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a"), wf("b", "test-workflow-b")],
            edges: vec![edge("a", "b", EdgeType::Success)],
        },
        HashMap::from([
            state("f-04", "a", NodeStatus::Completed, Some("A output")),
            state("f-04", "b", NodeStatus::Completed, None),
        ]),
        "completed",
    );
    let _ = data(
        &c,
        json!({"sessionId":"f-04","fork":{"fromNodeId":"b","newSessionId":"f-04-new"}}),
    );
    let (fm, _, _) = read_markdown(&pipeline_session_uri("f-04-new"), &c.config).expect("read");
    let fm = fm.expect("frontmatter");
    let states: HashMap<String, NodeState> = frontmatter_value(&fm, "nodeStates").expect("states");
    let sid = states
        .get("fork-f-04-new-b")
        .expect("fork root")
        .session_id
        .clone();
    let (_, body, _) =
        read_markdown(&workflow_session_uri(&sid), &c.config).expect("read workflow");
    assert!(serde_yaml::to_string(&fm)
        .expect("yaml")
        .contains("predecessorContext"));
    assert!(body.contains("## Predecessor Context"));
}
#[test]
#[serial(config)]
fn f_05_fork_with_custom_new_session_id() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-05",
        PipelineDefinition {
            nodes: vec![wf("solo", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("f-05", "solo", NodeStatus::Completed, None)]),
        "completed",
    );
    assert_eq!(
        data(
            &c,
            json!({"sessionId":"f-05","fork":{"fromNodeId":"solo","newSessionId":"custom-fork"}})
        )["pipelineId"]
            .as_str(),
        Some("custom-fork")
    );
}
#[test]
#[serial(config)]
fn f_06_fork_all_nodes_start_pending() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-06",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a"), wf("b", "test-workflow-b")],
            edges: vec![edge("a", "b", EdgeType::Success)],
        },
        HashMap::from([
            state("f-06", "a", NodeStatus::Completed, None),
            state("f-06", "b", NodeStatus::Stuck, None),
        ]),
        "stuck",
    );
    assert!(data(
        &c,
        json!({"sessionId":"f-06","fork":{"fromNodeId":"a","newSessionId":"f-06-new"}})
    )["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .all(|n| n["status"] == "pending"));
}
#[test]
#[serial(config)]
fn f_07_fork_with_existing_new_session_id_rejected() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-07-src",
        PipelineDefinition {
            nodes: vec![wf("solo", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("f-07-src", "solo", NodeStatus::Completed, None)]),
        "completed",
    );
    seed(
        &c,
        "f-07-existing",
        PipelineDefinition {
            nodes: vec![wf("x", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([state("f-07-existing", "x", NodeStatus::Completed, None)]),
        "completed",
    );
    assert!(pipeline(
        &c,
        &json!({"sessionId":"f-07-src","fork":{"fromNodeId":"solo","newSessionId":"f-07-existing"}})
    )
    .expect_err("err")
    .to_string()
    .contains("already exists"));
}
#[test]
#[serial(config)]
fn f_08_fork_from_nonexistent_node_rejected() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-08",
        PipelineDefinition {
            nodes: vec![wf("solo", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("f-08", "solo", NodeStatus::Completed, None)]),
        "completed",
    );
    assert!(pipeline(
        &c,
        &json!({"sessionId":"f-08","fork":{"fromNodeId":"missing","newSessionId":"f-08-new"}})
    )
    .expect_err("err")
    .to_string()
    .contains("doesn't contain"));
}
#[test]
#[serial(config)]
fn f_09_fork_from_root_copies_entire_pipeline() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-09",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a"), wf("b", "test-workflow-b")],
            edges: vec![edge("a", "b", EdgeType::Success)],
        },
        HashMap::from([
            state("f-09", "a", NodeStatus::Completed, None),
            state("f-09", "b", NodeStatus::Completed, None),
        ]),
        "completed",
    );
    let d = data(
        &c,
        json!({"sessionId":"f-09","fork":{"fromNodeId":"a","newSessionId":"f-09-new"}}),
    );
    assert_eq!(d["nodes"].as_array().expect("nodes").len(), 2);
    assert_eq!(d["ready"], json!(["fork-f-09-new-a"]));
}
#[test]
#[serial(config)]
fn f_10_fork_pipeline_with_no_edges_creates_single_root_pipeline() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-10",
        PipelineDefinition {
            nodes: vec![wf("solo", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("f-10", "solo", NodeStatus::Completed, None)]),
        "completed",
    );
    assert_eq!(
        data(
            &c,
            json!({"sessionId":"f-10","fork":{"fromNodeId":"solo","newSessionId":"f-10-new"}})
        )["ready"],
        json!(["fork-f-10-new-solo"])
    );
}
#[test]
#[serial(config)]
fn f_11_double_fork_from_same_node() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-11",
        PipelineDefinition {
            nodes: vec![wf("solo", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([state("f-11", "solo", NodeStatus::Completed, None)]),
        "completed",
    );
    let _ = data(
        &c,
        json!({"sessionId":"f-11","fork":{"fromNodeId":"solo","newSessionId":"f-11-a"}}),
    );
    let _ = data(
        &c,
        json!({"sessionId":"f-11","fork":{"fromNodeId":"solo","newSessionId":"f-11-b"}}),
    );
    assert!(file_exists(&pipeline_session_uri("f-11-a"), &c.config).expect("a"));
    assert!(file_exists(&pipeline_session_uri("f-11-b"), &c.config).expect("b"));
}
#[test]
#[serial(config)]
fn f_12_fork_diamond_dag_from_join_point() {
    let (_t, c, _r) = pipeline_test_context(OutputFormat::Json);
    seed(
        &c,
        "f-12",
        PipelineDefinition {
            nodes: vec![
                wf("root", "test-workflow-a"),
                wf("l", "test-workflow-b"),
                wf("r", "test-workflow-a"),
                wf("join", "test-workflow-b"),
                wf("tail", "test-workflow-a"),
            ],
            edges: vec![
                edge("root", "l", EdgeType::Success),
                edge("root", "r", EdgeType::Success),
                edge("l", "join", EdgeType::Success),
                edge("r", "join", EdgeType::Success),
                edge("join", "tail", EdgeType::Success),
            ],
        },
        HashMap::from([
            state("f-12", "root", NodeStatus::Completed, None),
            state("f-12", "l", NodeStatus::Completed, None),
            state("f-12", "r", NodeStatus::Completed, None),
            state("f-12", "join", NodeStatus::Completed, None),
            state("f-12", "tail", NodeStatus::Pending, None),
        ]),
        "active",
    );
    let ids = data(
        &c,
        json!({"sessionId":"f-12","fork":{"fromNodeId":"join","newSessionId":"f-12-new"}}),
    )["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|n| n["nodeId"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    assert!(ids.iter().any(|id| id.ends_with("join")));
    assert!(ids.iter().any(|id| id.ends_with("tail")));
}
