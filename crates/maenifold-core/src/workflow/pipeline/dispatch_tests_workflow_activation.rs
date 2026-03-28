use crate::workflow::pipeline::test_support::seed_dispatch_pipeline;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

use super::super::dispatch_workflow::activate_workflow_node_session;
use super::super::{NodeState, NodeStatus};
use super::support::{dispatch_data, pipeline_context, read_session};

#[test]
#[serial(config)]
fn pipeline_dispatch_renders_first_step_into_session_body() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600400";
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600400-root", NodeStatus::Pending),
        node_state("child", "workflow-1735689600400-child", NodeStatus::Pending),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
    dispatch_data(&context, pipeline_id);
    let (root_fm, root_content) = read_session(&context, "workflow-1735689600400-root");
    assert_eq!(
        root_fm.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
    assert!(root_content.contains("## Step 1/1"));
    assert!(root_content.contains("\"id\": \"step-1\"") || root_content.contains("step-1"));
    assert!(root_content.contains("Run [[test-workflow-a]]"));
    let (child_fm, child_content) = read_session(&context, "workflow-1735689600400-child");
    assert_eq!(
        child_fm.get("status").and_then(YamlValue::as_str),
        Some("pending")
    );
    assert!(!child_content.contains("## Step"));
}

#[test]
#[serial(config)]
fn activate_workflow_node_session_skips_active_session_without_duplication() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600450";
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600450-root", NodeStatus::Pending),
        node_state("child", "workflow-1735689600450-child", NodeStatus::Pending),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
    assert!(
        activate_workflow_node_session(&context, "test-workflow-a", "workflow-1735689600450-root")
            .expect("first activation")
    );
    assert!(
        !activate_workflow_node_session(&context, "test-workflow-a", "workflow-1735689600450-root")
            .expect("skip active")
    );
    let (frontmatter, content) = read_session(&context, "workflow-1735689600450-root");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
    assert_eq!(content.matches("## Step 1/1").count(), 1);
}

#[test]
#[serial(config)]
fn pipeline_dispatch_skips_already_active_workflow_sessions() {
    let (_tmp, context, _reset) = pipeline_context();
    let pipeline_id = "pipeline-1735689600475";
    let node_states = HashMap::from([
        node_state("root", "workflow-1735689600475-root", NodeStatus::Pending),
        node_state("child", "workflow-1735689600475-child", NodeStatus::Pending),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);
    assert!(
        activate_workflow_node_session(&context, "test-workflow-a", "workflow-1735689600475-root")
            .expect("setup activation")
    );
    let data = dispatch_data(&context, pipeline_id);
    assert!(
        data["dispatched"]
            .as_array()
            .expect("dispatched")
            .is_empty()
    );
    assert_eq!(
        data["active"].as_array().expect("active"),
        &vec![json!("root")]
    );
    let (_, content) = read_session(&context, "workflow-1735689600475-root");
    assert_eq!(content.matches("## Step 1/1").count(), 1);
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
