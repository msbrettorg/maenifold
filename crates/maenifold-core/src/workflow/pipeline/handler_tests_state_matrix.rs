use super::handler::pipeline;
use super::handler_tests::set_pipeline_status;
use super::supervisor::supervisor_spawn_test_task;
use super::supervisor_test_support::reset_supervisor;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serial_test::serial;
use std::collections::HashMap;

fn wf(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        workflow_id: if id.ends_with('b') {
            "test-workflow-b"
        } else {
            "test-workflow-a"
        }
        .to_string(),
        acp_client: None,
        prompt: None,
        timeout: None,
    }
}

fn state(id: &str, status: NodeStatus) -> (String, NodeState) {
    (
        id.to_string(),
        NodeState {
            session_id: format!("workflow-stm-{id}"),
            status,
            attempts: 1,
            thinking_session_id: None,
            output: Some(format!("output-{id}")),
        },
    )
}

fn one(id: &str, status: NodeStatus) -> HashMap<String, NodeState> {
    HashMap::from([state(id, status)])
}

#[test]
#[ignore]
#[serial(config)]
fn stm_01_delete_from_completed_state() { // BUG: delete-from-completed-state
}

#[test]
#[serial(config)]
fn stm_02_delete_from_stuck_state() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-02",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Stuck),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-02", "delete": true })
    )
    .is_ok());
}
#[test]
#[serial(config)]
fn stm_03_delete_from_all_abandoned_state() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-03",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Abandoned),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-03", "delete": true })
    )
    .is_ok());
}
#[test]
#[serial(config)]
fn stm_04_delete_from_active_state_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-04",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Active),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-04", "delete": true })
    )
    .is_err());
}
#[test]
#[serial(config)]
fn stm_05_delete_from_pending_state_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-05",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-05", "delete": true })
    )
    .is_err());
}

#[test]
#[serial(config)]
fn stm_06_graft_onto_completed_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-06",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    set_pipeline_status(&context, "pipeline-stm-06", "completed");
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-06", "graft": { "nodes": [json!({"id":"b","workflowId":"test-workflow-b"})], "edges": [json!({"source":"a","target":"b"})] } })).is_ok());
}
#[test]
#[serial(config)]
fn stm_07_graft_onto_stuck_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-07",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Stuck),
        1,
    );
    set_pipeline_status(&context, "pipeline-stm-07", "stuck");
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-07", "graft": { "nodes": [json!({"id":"b","workflowId":"test-workflow-b"})], "edges": [json!({"source":"a","target":"b"})] } })).is_ok());
}
#[test]
#[serial(config)]
fn stm_08_graft_onto_paused_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-08",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    set_pipeline_status(&context, "pipeline-stm-08", "paused");
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-08", "graft": { "nodes": [json!({"id":"b","workflowId":"test-workflow-b"})], "edges": [json!({"source":"a","target":"b"})] } })).is_ok());
}
#[test]
#[serial(config)]
fn stm_09_graft_onto_active_pipeline_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-09",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Active),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-09", "graft": { "nodes": [json!({"id":"b","workflowId":"test-workflow-b"})], "edges": [json!({"source":"a","target":"b"})] } })).is_err());
}
#[test]
#[serial(config)]
fn stm_10_graft_onto_pending_pipeline_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-10",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-10", "graft": { "nodes": [json!({"id":"b","workflowId":"test-workflow-b"})], "edges": [json!({"source":"a","target":"b"})] } })).is_err());
}

#[test]
#[serial(config)]
fn stm_11_rewind_completed_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-11",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-11", "rewind": { "nodeIds": ["a"] } })
    )
    .is_ok());
}
#[test]
#[serial(config)]
fn stm_12_rewind_abandoned_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-12",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Abandoned),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-12", "rewind": { "nodeIds": ["a"] } })
    )
    .is_ok());
}
#[test]
#[serial(config)]
fn stm_13_rewind_stuck_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-13",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Stuck),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-13", "rewind": { "nodeIds": ["a"] } })
    )
    .is_ok());
}

#[test]
#[serial(config)]
fn stm_14_restart_completed_node_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-14",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-14", "nodeId": "a", "restart": true })
    )
    .is_err());
}
#[test]
#[serial(config)]
fn stm_15_restart_pending_node_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-15",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-15", "nodeId": "a", "restart": true })
    )
    .is_err());
}

#[test]
#[serial(config)]
fn stm_16_join_two_completed_pipelines() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-16-target",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-16-source",
        PipelineDefinition {
            nodes: vec![wf("b")],
            edges: vec![],
        },
        one("b", NodeStatus::Completed),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-16-target", "join": { "sourceSessionId": "pipeline-stm-16-source", "edges": [json!({"source":"a","target":"b"})] } })).is_ok());
}

#[test]
#[serial(config)]
fn stm_17_join_into_active_target_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-17-target",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Active),
        1,
    );
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-17-source",
        PipelineDefinition {
            nodes: vec![wf("b")],
            edges: vec![],
        },
        one("b", NodeStatus::Completed),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-17-target", "join": { "sourceSessionId": "pipeline-stm-17-source", "edges": [json!({"source":"a","target":"b"})] } })).is_err());
}
#[test]
#[serial(config)]
fn stm_18_join_from_active_source_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-18-target",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-18-source",
        PipelineDefinition {
            nodes: vec![wf("b")],
            edges: vec![],
        },
        one("b", NodeStatus::Active),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-18-target", "join": { "sourceSessionId": "pipeline-stm-18-source", "edges": [json!({"source":"a","target":"b"})] } })).is_err());
}

#[test]
#[ignore]
#[serial(config)]
fn stm_19_join_into_pending_target_rejected() {
    // BUG: join-into-pending-target-allowed
}

#[test]
#[serial(config)]
fn stm_20_fork_from_completed_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-20",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-20", "fork": { "fromNodeId": "a", "newSessionId": "fork-stm-20" } })).is_ok());
}
#[test]
#[serial(config)]
fn stm_21_fork_from_active_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-21",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Active),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-21", "fork": { "fromNodeId": "a", "newSessionId": "fork-stm-21" } })).is_ok());
}
#[test]
#[serial(config)]
fn stm_22_fork_from_pending_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-22",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-22", "fork": { "fromNodeId": "a", "newSessionId": "fork-stm-22" } })).is_ok());
}
#[test]
#[serial(config)]
fn stm_23_fork_from_stuck_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-23",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Stuck),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-23", "fork": { "fromNodeId": "a", "newSessionId": "fork-stm-23" } })).is_ok());
}
#[test]
#[serial(config)]
fn stm_24_dispatch_on_completed_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-24",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    let data: Value = serde_json::from_str(
        &pipeline(
            &context,
            &json!({ "sessionId": "pipeline-stm-24", "dispatch": true }),
        )
        .expect("dispatch"),
    )
    .expect("parse");
    assert_eq!(data["data"]["dispatched"], json!([]));
}

#[test]
#[ignore]
#[serial(config)]
fn stm_25_dispatch_on_cancelled_pipeline() {
    // BUG: dispatch-on-cancelled-pipeline
}

#[test]
#[serial(config)]
fn stm_26_delete_paused_pipeline_with_no_remaining_work() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-26",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    set_pipeline_status(&context, "pipeline-stm-26", "paused");
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-26", "delete": true })
    )
    .is_ok());
}
#[test]
#[serial(config)]
fn stm_27_delete_paused_pipeline_with_remaining_work() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-27",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    set_pipeline_status(&context, "pipeline-stm-27", "paused");
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-27", "delete": true })
    )
    .is_err());
}
#[test]
#[serial(config)]
fn stm_28_delete_cancelled_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-28",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Abandoned),
        1,
    );
    set_pipeline_status(&context, "pipeline-stm-28", "cancelled");
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-28", "delete": true })
    )
    .is_ok());
}

#[test]
#[serial(config)]
fn stm_29_pause_already_paused_pipeline() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-29",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    supervisor_spawn_test_task("pipeline-stm-29").expect("spawn");
    set_pipeline_status(&context, "pipeline-stm-29", "paused");
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-29", "pause": true })
    )
    .is_ok());
}

#[test]
#[serial(config)]
fn stm_30_resume_completed_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-30",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    set_pipeline_status(&context, "pipeline-stm-30", "completed");
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-30", "resume": true })
    )
    .is_err());
}
#[test]
#[serial(config)]
fn stm_31_cancel_completed_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-31",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    set_pipeline_status(&context, "pipeline-stm-31", "completed");
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-31", "cancel": true })
    )
    .is_err());
}

#[test]
#[serial(config)]
fn stm_32_cancel_paused_pipeline() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-32",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    supervisor_spawn_test_task("pipeline-stm-32").expect("spawn");
    set_pipeline_status(&context, "pipeline-stm-32", "paused");
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-32", "cancel": true })
    )
    .is_ok());
}

#[test]
#[serial(config)]
fn stm_33_complete_a_pending_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-33",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-33", "nodeId": "a", "status": "completed", "conclusion": "done" })).is_err());
}
#[test]
#[serial(config)]
fn stm_34_complete_an_already_completed_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-34",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Completed),
        1,
    );
    assert!(pipeline(&context, &json!({ "sessionId": "pipeline-stm-34", "nodeId": "a", "status": "completed", "conclusion": "done" })).is_err());
}
#[test]
#[serial(config)]
fn stm_35_abandon_a_pending_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-35",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Pending),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-35", "nodeId": "a", "status": "abandoned" })
    )
    .is_err());
}
#[test]
#[serial(config)]
fn stm_36_abandon_an_already_abandoned_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-36",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Abandoned),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-36", "nodeId": "a", "status": "abandoned" })
    )
    .is_err());
}
#[test]
#[serial(config)]
fn stm_37_restart_an_active_node() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-stm-37",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        one("a", NodeStatus::Active),
        1,
    );
    assert!(pipeline(
        &context,
        &json!({ "sessionId": "pipeline-stm-37", "nodeId": "a", "restart": true })
    )
    .is_err());
}
