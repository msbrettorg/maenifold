use super::handler::pipeline;
use super::session_io::frontmatter_value;
use super::test_support::*;
use super::update::restart_pipeline_node;
use super::{NodeState, NodeStatus};
use crate::markdown::{file_exists, read_markdown, write_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

#[test]
#[serial(config)]
fn pipeline_restart_recreates_abandoned_node_session() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600900";
    let original_child_session = "workflow-1735689600900-child";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600900-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: original_child_session.to_string(),
                status: NodeStatus::Abandoned,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

    let root_uri = workflow_session_uri("workflow-1735689600900-root");
    let (root_frontmatter, root_content, _) =
        read_markdown(&root_uri, &context.config).expect("read root");
    let mut root_frontmatter = root_frontmatter.expect("root frontmatter");
    root_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    root_frontmatter.insert(
        "last".to_string(),
        YamlValue::String("Root finished [[handoff]]".to_string()),
    );
    write_markdown(
        &root_uri,
        Some(&root_frontmatter),
        &root_content,
        &context.config,
    )
    .expect("write root workflow");

    let child_uri = workflow_session_uri(original_child_session);
    let (child_frontmatter, child_content, _) =
        read_markdown(&child_uri, &context.config).expect("read child");
    let mut child_frontmatter = child_frontmatter.expect("child frontmatter");
    child_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("abandoned".to_string()),
    );
    write_markdown(
        &child_uri,
        Some(&child_frontmatter),
        &child_content,
        &context.config,
    )
    .expect("write child workflow");

    let output = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "nodeId": "child",
            "restart": true
        }),
    )
    .expect("restart node");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    let child = nodes
        .iter()
        .find(|node| node.get("nodeId") == Some(&Value::String("child".to_string())))
        .expect("child node");
    let restarted_session = child
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("new session id");
    assert_ne!(restarted_session, original_child_session);
    assert_eq!(child.get("status").and_then(Value::as_str), Some("pending"));
    assert_eq!(child.get("attempts").and_then(Value::as_u64), Some(2));
    assert_eq!(
        data.get("ready").and_then(Value::as_array),
        Some(&vec![Value::String("child".to_string())])
    );

    let (pipeline_frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
    let updated_states: HashMap<String, NodeState> =
        frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");
    assert_eq!(updated_states["child"].session_id, restarted_session);
    assert_eq!(updated_states["child"].status, NodeStatus::Pending);
    assert_eq!(updated_states["child"].attempts, 2);

    assert!(file_exists(&child_uri, &context.config).expect("old session exists"));
    let (new_child_frontmatter, new_child_content, _) =
        read_markdown(&workflow_session_uri(restarted_session), &context.config)
            .expect("read restarted workflow");
    let new_child_frontmatter = new_child_frontmatter.expect("new child frontmatter");
    assert_eq!(
        new_child_frontmatter
            .get("status")
            .and_then(YamlValue::as_str),
        Some("pending")
    );
    assert!(new_child_content.contains("## Predecessor Context"));
    assert!(new_child_content.contains("Root finished [[handoff]]"));
}

#[test]
#[serial(config)]
fn pipeline_restart_recreates_stuck_node_session() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600950";
    let original_child_session = "workflow-1735689600950-child";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600950-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: original_child_session.to_string(),
                status: NodeStatus::Stuck,
                attempts: 2,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

    let output = restart_pipeline_node(&context, pipeline_id, "child").expect("restart node");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    let child = nodes
        .iter()
        .find(|node| node.get("nodeId") == Some(&Value::String("child".to_string())))
        .expect("child node");
    let restarted_session = child
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("new session id");

    assert_ne!(restarted_session, original_child_session);
    assert_eq!(child.get("status").and_then(Value::as_str), Some("pending"));
    assert_eq!(child.get("attempts").and_then(Value::as_u64), Some(3));

    let (pipeline_frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
    let updated_states: HashMap<String, NodeState> =
        frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");
    assert_eq!(updated_states["child"].status, NodeStatus::Pending);
    assert_eq!(updated_states["child"].attempts, 3);
    assert_eq!(updated_states["child"].session_id, restarted_session);
}

#[test]
#[serial(config)]
fn pipeline_restart_rejects_when_max_attempts_reached() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689601000";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689601000-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689601000-child".to_string(),
                status: NodeStatus::Abandoned,
                attempts: 3,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
    super::session_io::update_workflow_node_frontmatter(
        &context,
        "workflow-1735689601000-child",
        "abandoned",
        None,
    )
    .expect("mark child abandoned");

    let error = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "nodeId": "child",
            "restart": true
        }),
    )
    .expect_err("restart must fail at max attempts");

    assert_eq!(error.to_string(), "Node child has reached max attempts (3)");

    let (pipeline_frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
    let updated_states: HashMap<String, NodeState> =
        frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");
    assert_eq!(updated_states["child"].status, NodeStatus::Stuck);
    assert_eq!(
        updated_states["child"].session_id,
        "workflow-1735689601000-child"
    );
    assert_eq!(updated_states["child"].attempts, 3);
}

#[test]
#[serial(config)]
fn pipeline_restart_clears_thinking_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689601050";
    let original_child_session = "workflow-1735689601050-child";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689601050-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: original_child_session.to_string(),
                status: NodeStatus::Abandoned,
                attempts: 1,
                thinking_session_id: Some("thinking-stale-xyz".to_string()),
                output: Some("stale output".to_string()),
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

    let output = restart_pipeline_node(&context, pipeline_id, "child").expect("restart succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let _ = response; // result shape verified in other tests

    let (pipeline_frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
    let updated_states: HashMap<String, NodeState> =
        frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");

    assert!(
        updated_states["child"].thinking_session_id.is_none(),
        "thinking_session_id must be cleared on restart"
    );
    assert!(
        updated_states["child"].output.is_none(),
        "output must be cleared on restart"
    );
    assert_eq!(updated_states["child"].status, NodeStatus::Pending);
    assert_eq!(updated_states["child"].attempts, 2);
}
