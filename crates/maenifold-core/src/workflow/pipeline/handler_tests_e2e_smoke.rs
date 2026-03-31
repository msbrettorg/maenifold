use super::handler::pipeline;
use super::handler_tests::set_pipeline_status;
use super::test_support::{
    pipeline_test_context, seed_pipeline_session_with_node_sessions,
    seed_pipeline_session_with_workflows,
};
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;

fn wf(id: &str, workflow_id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        workflow_id: workflow_id.to_string(),
        acp_client: None,
        prompt: None,
        timeout: None,
    }
}

fn prompt(id: &str) -> PipelineNode {
    PipelineNode::Prompt {
        id: id.to_string(),
        prompt: "Prompt [[pipeline]]".to_string(),
        agent: "agent".to_string(),
        tool_mode: "required".to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        timeout: None,
        agent_config: None,
    }
}

fn think(id: &str) -> PipelineNode {
    PipelineNode::SequentialThinking {
        id: id.to_string(),
        topic: "Topic [[pipeline]]".to_string(),
        total_thoughts: 2,
        analysis_type: None,
        agent: "agent".to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        timeout: None,
        agent_config: None,
    }
}

fn tool(id: &str) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        tool_name: "Pipeline".to_string(),
        tool_args: Some(json!({"learn": true})),
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        agent_config: None,
    }
}

fn edge(source: &str, target: &str, edge_type: EdgeType) -> PipelineEdge {
    PipelineEdge {
        source: source.to_string(),
        target: target.to_string(),
        edge_type,
        mapping: None,
    }
}

fn data(output: String) -> Value {
    serde_json::from_str::<Value>(&output).expect("parse")["data"].clone()
}

fn pipeline_id(output: &str) -> String {
    serde_json::from_str::<Value>(output).expect("parse")["data"]["pipelineId"]
        .as_str()
        .expect("id")
        .to_string()
}

#[test]
#[serial(config)]
fn e2e_01_create_dispatch_complete_all_delete() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let created = pipeline(&context, &json!({ "definition": { "nodes": [json!({"id":"a","workflowId":"test-workflow-a"}), json!({"id":"b","workflowId":"test-workflow-b"})], "edges": [json!({"source":"a","target":"b"})] } })).expect("create");
    let pipeline_id = pipeline_id(&created);
    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "dispatch": true }),
    )
    .expect("dispatch a");
    pipeline(&context, &json!({ "sessionId": pipeline_id, "nodeId": "a", "status": "completed", "conclusion": "a done [[pipeline]]" })).expect("complete a");
    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "dispatch": true }),
    )
    .expect("dispatch b");
    pipeline(&context, &json!({ "sessionId": pipeline_id, "nodeId": "b", "status": "completed", "conclusion": "b done [[pipeline]]" })).expect("complete b");
    assert!(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "delete": true })
        )
        .is_ok()
    );
}

#[test]
#[serial(config)]
fn e2e_02_create_dispatch_abandon_restart_complete_delete() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let created = pipeline(&context, &json!({ "definition": { "nodes": [json!({"id":"a","workflowId":"test-workflow-a"})], "edges": [] } })).expect("create");
    let pipeline_id = pipeline_id(&created);
    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "dispatch": true }),
    )
    .expect("dispatch");
    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "nodeId": "a", "status": "abandoned" }),
    )
    .expect("abandon");
    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "nodeId": "a", "restart": true }),
    )
    .expect("restart");
    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "dispatch": true }),
    )
    .expect("dispatch again");
    pipeline(&context, &json!({ "sessionId": pipeline_id, "nodeId": "a", "status": "completed", "conclusion": "done [[pipeline]]" })).expect("complete");
    assert!(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "delete": true })
        )
        .is_ok()
    );
}

#[test]
#[ignore]
#[serial(config)]
fn e2e_03_create_run_autonomous_execution_pipeline_completes() {
    // NEEDS-SUPERVISOR: requires running supervisor mock
}

#[test]
#[serial(config)]
fn e2e_04_diamond_dag_with_all_4_node_types_end_to_end() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![
            wf("root", "test-workflow-a"),
            prompt("p"),
            think("t"),
            tool("u"),
        ],
        edges: vec![
            edge("root", "p", EdgeType::Success),
            edge("root", "t", EdgeType::Success),
            edge("p", "u", EdgeType::Success),
            edge("t", "u", EdgeType::Always),
        ],
    };
    seed_pipeline_session_with_node_sessions(
        &context,
        "pipeline-e2e-04",
        definition,
        HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-e2e-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "p".to_string(),
                NodeState {
                    session_id: "workflow-e2e-p".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "t".to_string(),
                NodeState {
                    session_id: "workflow-e2e-t".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "u".to_string(),
                NodeState {
                    session_id: "workflow-e2e-u".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        4,
    );
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-04", "dispatch": true }),
    )
    .expect("dispatch root");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-04", "nodeId": "root", "status": "completed", "conclusion": "root [[pipeline]]" })).expect("complete root");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-04", "dispatch": true }),
    )
    .expect("dispatch branch");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-04", "nodeId": "p", "status": "completed", "conclusion": "p [[pipeline]]" })).expect("complete p");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-04", "nodeId": "t", "status": "completed", "conclusion": "t [[pipeline]]" })).expect("complete t");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-04", "dispatch": true }),
    )
    .expect("dispatch tool");
    let view = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-e2e-04", "view": true }),
        )
        .expect("view"),
    );
    assert_eq!(view["active"], json!(["u"]));
}

#[test]
#[ignore]
// BUG: delete-from-completed-state — has_remaining_work() guard blocks terminal pipeline deletion
#[serial(config)]
fn e2e_05_complete_graft_dispatch_complete_fork_rewind_delete() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-e2e-05",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([(
            "a".to_string(),
            NodeState {
                session_id: "workflow-e2e-05-a".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("done".to_string()),
            },
        )]),
        1,
    );
    set_pipeline_status(&context, "pipeline-e2e-05", "completed");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-05", "graft": { "nodes": [json!({"id":"b","workflowId":"test-workflow-b"})], "edges": [json!({"source":"a","target":"b"})] } })).expect("graft");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-05", "dispatch": true }),
    )
    .expect("dispatch b");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-05", "nodeId": "b", "status": "completed", "conclusion": "b [[pipeline]]" })).expect("complete b");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-05", "fork": { "fromNodeId": "b", "newSessionId": "pipeline-e2e-05-fork" } })).expect("fork");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-05", "rewind": { "nodeIds": ["a"] } }),
    )
    .expect("rewind");
    assert!(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-e2e-05-fork", "delete": true })
        )
        .is_ok()
    );
}

#[test]
#[serial(config)]
fn e2e_06_complete_two_pipelines_join_delete_merged() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-e2e-06-a",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([(
            "a".to_string(),
            NodeState {
                session_id: "workflow-e2e-06-a".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("a".to_string()),
            },
        )]),
        1,
    );
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-e2e-06-b",
        PipelineDefinition {
            nodes: vec![wf("b", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([(
            "b".to_string(),
            NodeState {
                session_id: "workflow-e2e-06-b".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("b".to_string()),
            },
        )]),
        1,
    );
    set_pipeline_status(&context, "pipeline-e2e-06-a", "completed");
    set_pipeline_status(&context, "pipeline-e2e-06-b", "completed");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-06-a", "join": { "sourceSessionId": "pipeline-e2e-06-b", "edges": [json!({"source":"a","target":"b"})] } })).expect("join");
    let view_result = pipeline(
        &context,
        &json!({
            "sessionId": "pipeline-e2e-06-a", "view": true
        }),
    )
    .expect("view merged");
    let view_data: Value = serde_json::from_str(&view_result).expect("parse");
    let nodes = view_data["data"]["nodes"].as_array().expect("nodes array");
    assert!(
        nodes.len() > 1,
        "join should merge nodes from source pipeline"
    );
    assert!(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-e2e-06-a", "delete": true })
        )
        .is_ok()
    );
}

#[test]
#[serial(config)]
fn e2e_07_linear_chain_with_always_edges_first_node_abandoned() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let created = pipeline(&context, &json!({ "definition": { "nodes": [json!({"id":"a","workflowId":"test-workflow-a"}), json!({"id":"b","workflowId":"test-workflow-b"})], "edges": [json!({"source":"a","target":"b","type":"always"})] } })).expect("create");
    let pipeline_id = pipeline_id(&created);
    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "dispatch": true }),
    )
    .expect("dispatch a");
    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "nodeId": "a", "status": "abandoned" }),
    )
    .expect("abandon a");
    let data =
        data(pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true })).expect("view"));
    assert_eq!(data["ready"], json!(["b"]));
}

#[test]
#[serial(config)]
fn e2e_08_node_exhausts_retries_stuck_rewind_complete() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-e2e-08",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([(
            "a".to_string(),
            NodeState {
                session_id: "workflow-e2e-08-a".to_string(),
                status: NodeStatus::Abandoned,
                attempts: 3,
                thinking_session_id: None,
                output: None,
            },
        )]),
        1,
    );
    assert!(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-e2e-08", "nodeId": "a", "restart": true })
        )
        .is_err()
    );
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-08", "rewind": { "nodeIds": ["a"] } }),
    )
    .expect("rewind");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-08", "dispatch": true }),
    )
    .expect("dispatch");
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-e2e-08", "nodeId": "a", "status": "completed", "conclusion": "done [[pipeline]]" })).is_ok());
}

#[test]
#[ignore]
// BUG: delete-from-completed-state — has_remaining_work() guard blocks terminal pipeline deletion
#[serial(config)]
fn e2e_09_graft_join_fork_rewind_delete_chain() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-e2e-09-a",
        PipelineDefinition {
            nodes: vec![wf("a", "test-workflow-a")],
            edges: vec![],
        },
        HashMap::from([(
            "a".to_string(),
            NodeState {
                session_id: "workflow-e2e-09-a".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("a".to_string()),
            },
        )]),
        1,
    );
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-e2e-09-b",
        PipelineDefinition {
            nodes: vec![wf("b", "test-workflow-b")],
            edges: vec![],
        },
        HashMap::from([(
            "b".to_string(),
            NodeState {
                session_id: "workflow-e2e-09-b".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("b".to_string()),
            },
        )]),
        1,
    );
    set_pipeline_status(&context, "pipeline-e2e-09-a", "completed");
    set_pipeline_status(&context, "pipeline-e2e-09-b", "completed");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-09-a", "graft": { "nodes": [json!({"id":"c","workflowId":"test-workflow-a"})], "edges": [json!({"source":"a","target":"c"})] } })).expect("graft");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-09-a", "join": { "sourceSessionId": "pipeline-e2e-09-b", "edges": [json!({"source":"c","target":"b"})] } })).expect("join");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-09-a", "fork": { "fromNodeId": "c", "newSessionId": "pipeline-e2e-09-fork" } })).expect("fork");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-09-a", "rewind": { "nodeIds": ["a"] } }),
    )
    .expect("rewind");
    assert!(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-e2e-09-fork", "delete": true })
        )
        .is_ok()
    );
}

#[test]
#[ignore]
#[serial(config)]
fn e2e_10_manual_dispatch_some_run_the_rest() {
    // NEEDS-SUPERVISOR: requires running supervisor mock
}

#[test]
#[ignore]
#[serial(config)]
fn e2e_11_pipeline_with_10_plus_nodes_mixed_types_and_edges() {
    // NEEDS-SUPERVISOR: requires larger execution harness
}

#[test]
#[serial(config)]
fn e2e_12_multiple_nodes_fail_restart_each_pipeline_completes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![wf("a", "test-workflow-a"), wf("b", "test-workflow-b")],
        edges: vec![],
    };
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-e2e-12",
        definition,
        HashMap::from([
            (
                "a".to_string(),
                NodeState {
                    session_id: "workflow-e2e-12-a".to_string(),
                    status: NodeStatus::Abandoned,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "b".to_string(),
                NodeState {
                    session_id: "workflow-e2e-12-b".to_string(),
                    status: NodeStatus::Abandoned,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        2,
    );
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-12", "nodeId": "a", "restart": true }),
    )
    .expect("restart a");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-12", "nodeId": "b", "restart": true }),
    )
    .expect("restart b");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-e2e-12", "dispatch": true }),
    )
    .expect("dispatch");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-12", "nodeId": "a", "status": "completed", "conclusion": "a [[pipeline]]" })).expect("complete a");
    pipeline(&context, &json!({ "sessionId": "pipeline-e2e-12", "nodeId": "b", "status": "completed", "conclusion": "b [[pipeline]]" })).expect("complete b");
    let view = data(
        pipeline(
            &context,
            &json!({ "sessionId": "pipeline-e2e-12", "view": true }),
        )
        .expect("view"),
    );
    assert_eq!(view["status"], "completed");
}
