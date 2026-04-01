use super::handler::pipeline;
use super::session_io::frontmatter_value;
use super::test_support::*;
use super::update::update_pipeline_node_status;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use crate::markdown::{read_markdown, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

#[test]
#[serial(config)]
fn pipeline_status_completed_updates_node_and_returns_pipeline_view() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600500";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600500-root".to_string(),
                status: NodeStatus::Active,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600500-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

    let root_uri = workflow_session_uri("workflow-1735689600500-root");
    let (root_fm, root_content, _) = read_markdown(&root_uri, &context.config).expect("read root");
    let mut root_fm = root_fm.expect("root frontmatter");
    root_fm.insert(
        "last".to_string(),
        YamlValue::String("Step 1 Response: Root [[context]] summary".to_string()),
    );
    write_markdown(&root_uri, Some(&root_fm), &root_content, &context.config)
        .expect("update root session");

    let output = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "nodeId": "root",
            "status": "completed",
            "conclusion": "Root [[context]] conclusion"
        }),
    )
    .expect("complete pipeline node");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("pipelineId").and_then(Value::as_str),
        Some(pipeline_id)
    );
    assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
    assert_eq!(
        data.get("ready").and_then(Value::as_array),
        Some(&vec![Value::String("child".to_string())])
    );
    assert_eq!(
        data.get("completed").and_then(Value::as_array),
        Some(&vec![Value::String("root".to_string())])
    );
    assert_eq!(
        data.get("active").and_then(Value::as_array),
        Some(&Vec::<Value>::new())
    );

    let (pipeline_fm, _, _) = read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
        .expect("read pipeline session");
    let pipeline_fm = pipeline_fm.expect("pipeline frontmatter");
    let updated_states: HashMap<String, NodeState> =
        frontmatter_value(&pipeline_fm, "nodeStates").expect("node states");
    assert_eq!(updated_states["root"].status, NodeStatus::Completed);
    assert_eq!(updated_states["child"].status, NodeStatus::Pending);

    let (root_fm, _, _) = read_markdown(&root_uri, &context.config).expect("read root session");
    let root_fm = root_fm.expect("root frontmatter");
    assert_eq!(
        root_fm.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
    assert_eq!(
        root_fm.get("conclusion").and_then(YamlValue::as_str),
        Some("Root [[context]] conclusion")
    );
}

#[test]
#[serial(config)]
fn pipeline_status_rejects_unsupported_status_values() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600550";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600550-root".to_string(),
                status: NodeStatus::Active,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600550-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

    let error =
        update_pipeline_node_status(&context, pipeline_id, "root", "invalid_status", None, None)
            .expect_err("unsupported status should fail");

    assert!(
        error
            .to_string()
            .contains("Unsupported pipeline node status"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_status_abandoned_enables_always_edge_ready_set() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600600";
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("workflow-agent".to_string()),
                prompt: None,

                timeout: None,
            },
            PipelineNode::Workflow {
                id: "cleanup".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: Some("workflow-agent".to_string()),
                prompt: None,

                timeout: None,
            },
        ],
        edges: vec![PipelineEdge {
            source: "root".to_string(),
            target: "cleanup".to_string(),
            edge_type: EdgeType::Always,
            mapping: None,
        }],
    };
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600600-root".to_string(),
                status: NodeStatus::Active,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "cleanup".to_string(),
            NodeState {
                session_id: "workflow-1735689600600-cleanup".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

    let output = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "nodeId": "root",
            "status": "abandoned"
        }),
    )
    .expect("abandon pipeline node");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("ready").and_then(Value::as_array),
        Some(&vec![Value::String("cleanup".to_string())])
    );
    assert_eq!(
        data.get("completed").and_then(Value::as_array),
        Some(&Vec::<Value>::new())
    );
    let cleanup_nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    assert_eq!(cleanup_nodes[0]["status"], "abandoned");
    assert_eq!(cleanup_nodes[1]["status"], "pending");

    let (root_fm, _, _) = read_markdown(
        &workflow_session_uri("workflow-1735689600600-root"),
        &context.config,
    )
    .expect("read root session");
    let root_fm = root_fm.expect("root frontmatter");
    assert_eq!(
        root_fm.get("status").and_then(YamlValue::as_str),
        Some("abandoned")
    );
}

#[test]
#[serial(config)]
fn pipeline_status_rejects_non_active_transitions() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600700";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600700-root".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600700-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

    let error = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "nodeId": "root",
            "status": "completed",
            "conclusion": "Should fail [[context]]"
        }),
    )
    .expect_err("non-active transition should fail");

    assert!(
        error
            .to_string()
            .contains("must be active to transition to completed"),
        "unexpected error: {error}"
    );
}
