use super::handler::pipeline;
use super::supervisor_test_support::reset_supervisor;
use super::test_support::{
    pipeline_test_context, seed_dispatch_pipeline, seed_pipeline_session_with_workflows,
};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

// NOTE: duplicated across param_smoke files for test isolation
fn solo_definition() -> Value {
    json!({ "nodes": [{ "id": "solo", "workflowId": "test-workflow-a" }], "edges": [] })
}

// NOTE: duplicated across param_smoke files for test isolation
fn assert_invalid_combo(error: impl std::fmt::Display) {
    assert!(
        error.to_string().contains("Invalid parameter combination"),
        "unexpected error: {error}"
    );
}

// NOTE: duplicated across param_smoke files for test isolation
fn completed_states(prefix: &str) -> HashMap<String, NodeState> {
    HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-root"),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("done".to_string()),
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-child"),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("done".to_string()),
            },
        ),
    ])
}

// NOTE: duplicated across param_smoke files for test isolation
fn active_states(prefix: &str) -> HashMap<String, NodeState> {
    HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-root"),
                status: NodeStatus::Active,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: format!("workflow-{prefix}-child"),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ])
}

fn one_node_definition(id: &str, workflow_id: &str) -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![PipelineNode::Workflow {
            id: id.to_string(),
            workflow_id: workflow_id.to_string(),
            acp_client: None,
            prompt: None,
            timeout: None,
        }],
        edges: vec![],
    }
}

#[test]
#[serial(config)]
fn test_smoke_pi21_prompt_without_run() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600101";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600101"), 2);
    assert_invalid_combo(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "prompt": "ambient only" }),
        )
        .expect_err("prompt without run"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi22_prompt_as_whitespace_only_with_run() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _supervisor = reset_supervisor();
    let output = pipeline(&context, &json!({ "definition": solo_definition() })).expect("create");
    let pipeline_id = serde_json::from_str::<Value>(&output).expect("parse")["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id")
        .to_string();
    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "run": true, "prompt": "   " }),
    )
    .expect("run");
    assert_eq!(
        serde_json::from_str::<Value>(&output).expect("parse")["data"]["status"],
        "started"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi23_pause_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "pause": true })).expect_err("pause without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi24_resume_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "resume": true })).expect_err("resume without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi25_cancel_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "cancel": true })).expect_err("cancel without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi26_delete_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "delete": true })).expect_err("delete without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi27_learn_alone() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Markdown);
    let output = pipeline(&context, &json!({ "learn": true })).expect("learn");
    assert!(output.contains("# Pipeline"));
}

#[test]
#[serial(config)]
fn test_smoke_pi28_learn_with_session_id_extra_param() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Markdown);
    let output = pipeline(
        &context,
        &json!({ "learn": true, "sessionId": "pipeline-1735689600102" }),
    )
    .expect("learn");
    assert!(output.contains("# Pipeline"));
}

#[test]
#[serial(config)]
fn test_smoke_pi29_node_id_without_status_or_restart() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600103";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600103"), 2);
    assert_invalid_combo(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "nodeId": "root" }),
        )
        .expect_err("nodeId alone"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi30_node_id_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "nodeId": "root" })).expect_err("nodeId without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi31_status_without_node_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600104";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600104"), 2);
    assert_invalid_combo(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "status": "completed" }),
        )
        .expect_err("status without nodeId"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi32_status_as_null() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600105";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600105"), 2);
    assert_invalid_combo(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "nodeId": "root", "status": null }),
        )
        .expect_err("null status"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi33_conclusion_without_status() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600106";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600106"), 2);
    assert_invalid_combo(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "nodeId": "root", "conclusion": "done" }),
        )
        .expect_err("conclusion without status"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi34_conclusion_with_whitespace_only() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600107";
    seed_dispatch_pipeline(&context, pipeline_id, active_states("1735689600107"), 2);
    let error = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id, "nodeId": "root", "status": "completed", "conclusion": "   "
        }),
    )
    .expect_err("blank conclusion should fail");
    assert!(
        error.to_string().contains("Conclusion required"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi35_restart_without_node_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600108";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600108"), 2);
    assert_invalid_combo(
        pipeline(
            &context,
            &json!({ "sessionId": pipeline_id, "restart": true }),
        )
        .expect_err("restart without nodeId"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi36_restart_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "nodeId": "root", "restart": true }))
            .expect_err("restart without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi37_graft_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "graft": { "nodes": [], "edges": [] } }))
            .expect_err("graft without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi38_graft_with_empty_nodes_array() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600109";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600109"), 2);
    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "graft": { "nodes": [], "edges": [] } }),
    )
    .expect("empty graft should route");
    assert_eq!(
        serde_json::from_str::<Value>(&output).expect("parse")["data"]["nodes"]
            .as_array()
            .expect("nodes")
            .len(),
        2
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi39_graft_as_string() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600110";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600110"), 2);
    let error = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "graft": "nope" }),
    )
    .expect_err("string graft");
    assert!(
        error.to_string().contains("invalid type"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi40_join_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(
            &context,
            &json!({ "join": { "sourceSessionId": "pipeline-1735689600111", "edges": [] } }),
        )
        .expect_err("join without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi41_join_without_source_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600112";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600112"), 2);
    let error = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "join": { "edges": [] } }),
    )
    .expect_err("missing sourceSessionId");
    assert!(
        error.to_string().contains("sourceSessionId"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi42_join_with_empty_edges() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-1735689600113",
        one_node_definition("left", "test-workflow-a"),
        HashMap::from([(
            "left".to_string(),
            NodeState {
                session_id: "workflow-1735689600113-left".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("ok".to_string()),
            },
        )]),
        1,
    );
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-1735689600114",
        one_node_definition("right", "test-workflow-b"),
        HashMap::from([(
            "right".to_string(),
            NodeState {
                session_id: "workflow-1735689600114-right".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("ok".to_string()),
            },
        )]),
        1,
    );
    let output = pipeline(&context, &json!({ "sessionId": "pipeline-1735689600113", "join": { "sourceSessionId": "pipeline-1735689600114", "edges": [] } })).expect("join should succeed");
    assert_eq!(
        serde_json::from_str::<Value>(&output).expect("parse")["data"]["nodes"]
            .as_array()
            .expect("nodes")
            .len(),
        2
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi43_fork_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "fork": { "fromNodeId": "root" } }))
            .expect_err("fork without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi44_fork_without_from_node_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600115";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600115"), 2);
    let error = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "fork": { "newSessionId": "pipeline-forked" } }),
    )
    .expect_err("missing fromNodeId");
    assert!(
        error.to_string().contains("fromNodeId"),
        "unexpected error: {error}"
    );
}

// BUG: fork accepts an empty newSessionId instead of rejecting it.
#[test]
#[ignore]
#[serial(config)]
fn test_smoke_pi45_fork_with_empty_new_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600116";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600116"), 2);
    let error = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "fork": { "fromNodeId": "root", "newSessionId": "" } }),
    )
    .expect_err("empty newSessionId should fail");
    assert!(
        error.to_string().contains("newSessionId"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi46_rewind_without_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    assert_invalid_combo(
        pipeline(&context, &json!({ "rewind": { "nodeIds": ["root"] } }))
            .expect_err("rewind without session"),
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi47_rewind_with_empty_node_ids() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600117";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600117"), 2);
    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": [] } }),
    )
    .expect("empty rewind should route");
    assert_eq!(
        serde_json::from_str::<Value>(&output).expect("parse")["data"]["pipelineId"],
        pipeline_id
    );
}

#[test]
#[serial(config)]
fn test_smoke_pi48_rewind_as_string() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600118";
    seed_dispatch_pipeline(&context, pipeline_id, completed_states("1735689600118"), 2);
    let error = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "rewind": "nope" }),
    )
    .expect_err("string rewind");
    assert!(
        error.to_string().contains("invalid type"),
        "unexpected error: {error}"
    );
}
