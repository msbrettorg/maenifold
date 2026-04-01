use super::handler::pipeline;
use super::handler_tests::set_pipeline_status;
use super::supervisor::{supervisor_is_running, supervisor_spawn_test_task};
use super::supervisor_test_support::reset_supervisor;
use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;

fn wf(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        workflow_id: "test-workflow-a".to_string(),
        acp_client: Some("workflow-agent".to_string()),
        prompt: None,
        timeout: None,
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

fn state(id: &str, status: NodeStatus) -> (String, NodeState) {
    (
        id.to_string(),
        NodeState {
            session_id: format!("workflow-life-{id}"),
            status,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )
}

macro_rules! needs_supervisor {
    ($name:ident, $note:literal) => {
        #[test]
        #[ignore]
        #[serial(config)]
        fn $name() {
            // NEEDS-SUPERVISOR: requires running supervisor mock
            let _ = $note;
        }
    };
}

needs_supervisor!(l_01_run_starts_autonomous_execution, "L-01");
needs_supervisor!(l_02_run_with_ambient_prompt, "L-02");
needs_supervisor!(l_03_pause_stops_after_active_nodes_drain, "L-03");
needs_supervisor!(l_05_cancel_abandons_active_nodes, "L-05");
needs_supervisor!(l_12_run_pause_resume_completes, "L-12");
needs_supervisor!(l_13_run_pause_graft_resume, "L-13");
needs_supervisor!(l_14_run_pause_rewind_resume, "L-14");
needs_supervisor!(l_15_run_cancel_restart_abandoned_run_again, "L-15");
needs_supervisor!(l_17_run_complete_run_again, "L-17");
needs_supervisor!(l_22_pipeline_auto_completes_when_all_nodes_done, "L-22");
needs_supervisor!(l_23_pipeline_detects_stuck_during_run, "L-23");
needs_supervisor!(l_24_run_executes_nodes_in_dependency_order, "L-24");
needs_supervisor!(l_25_run_respects_max_concurrency, "L-25");
needs_supervisor!(l_26_run_passes_predecessor_context_downstream, "L-26");
needs_supervisor!(l_27_run_with_always_edges_continues_on_abandoned, "L-27");
needs_supervisor!(l_28_run_with_success_edge_stops_chain_on_abandoned, "L-28");

#[test]
#[serial(config)]
fn l_04_resume_restarts_paused_pipeline() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-04",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Pending)]),
        1,
    );
    supervisor_spawn_test_task("pipeline-l-04").expect("spawn");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-l-04", "pause": true }),
    )
    .expect("pause");
    let data: Value = serde_json::from_str(
        &pipeline(
            &context,
            &json!({ "sessionId": "pipeline-l-04", "resume": true }),
        )
        .expect("resume"),
    )
    .expect("parse");
    assert_eq!(data["data"]["status"], "resumed");
    assert!(supervisor_is_running("pipeline-l-04"));
}

#[test]
#[serial(config)]
fn l_06_pause_non_running_pipeline_returns_error() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-06",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Pending)]),
        1,
    );
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-l-06", "pause": true }),
    )
    .expect_err("pause should fail");
    assert!(error.to_string().contains("isn't running"));
}

#[test]
#[serial(config)]
fn l_07_resume_non_paused_pipeline_returns_error() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-07",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Pending)]),
        1,
    );
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-l-07", "resume": true }),
    )
    .expect_err("resume should fail");
    assert!(error.to_string().contains("is not paused"));
}

#[test]
#[serial(config)]
fn l_08_cancel_non_running_pipeline_returns_error() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-08",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Pending)]),
        1,
    );
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-l-08", "cancel": true }),
    )
    .expect_err("cancel should fail");
    assert!(error.to_string().contains("isn't running"));
}

#[test]
#[serial(config)]
fn l_09_run_already_running_pipeline_returns_error() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-09",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Pending)]),
        1,
    );
    supervisor_spawn_test_task("pipeline-l-09").expect("spawn");
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-l-09", "run": true }),
    )
    .expect_err("run should fail");
    assert!(error.to_string().contains("already running"));
}

#[test]
#[serial(config)]
fn l_10_run_with_active_nodes_already_present_returns_error() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-10",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Active)]),
        1,
    );
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-l-10", "run": true }),
    )
    .expect_err("run should fail");
    assert!(error.to_string().contains("already has active nodes"));
}

#[test]
#[serial(config)]
fn l_11_resume_non_paused_reports_status_message() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-11",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Completed)]),
        1,
    );
    set_pipeline_status(&context, "pipeline-l-11", "completed");
    let error = pipeline(
        &context,
        &json!({ "sessionId": "pipeline-l-11", "resume": true }),
    )
    .expect_err("resume should fail");
    assert!(error.to_string().contains("status: completed"));
}

#[test]
#[serial(config)]
fn l_16_run_pause_cancel() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-16",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Pending)]),
        1,
    );
    supervisor_spawn_test_task("pipeline-l-16").expect("spawn");
    pipeline(
        &context,
        &json!({ "sessionId": "pipeline-l-16", "pause": true }),
    )
    .expect("pause");
    let data: Value = serde_json::from_str(
        &pipeline(
            &context,
            &json!({ "sessionId": "pipeline-l-16", "cancel": true }),
        )
        .expect("cancel"),
    )
    .expect("parse");
    assert_eq!(data["data"]["status"], "cancelled");
}

#[test]
#[serial(config)]
fn l_18_pause_delete_with_no_remaining_work() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-18",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Completed)]),
        1,
    );
    set_pipeline_status(&context, "pipeline-l-18", "paused");
    let data: Value = serde_json::from_str(
        &pipeline(
            &context,
            &json!({ "sessionId": "pipeline-l-18", "delete": true }),
        )
        .expect("delete"),
    )
    .expect("parse");
    assert_eq!(data["data"]["deletedSessionId"], "pipeline-l-18");
}

#[test]
#[serial(config)]
fn l_19_view_while_running_shows_running_true() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-19",
        PipelineDefinition {
            nodes: vec![wf("a")],
            edges: vec![],
        },
        HashMap::from([state("a", NodeStatus::Active)]),
        1,
    );
    supervisor_spawn_test_task("pipeline-l-19").expect("spawn");
    let data: Value = serde_json::from_str(
        &pipeline(
            &context,
            &json!({ "sessionId": "pipeline-l-19", "view": true }),
        )
        .expect("view"),
    )
    .expect("parse");
    assert_eq!(data["data"]["running"], true);
}

#[test]
#[serial(config)]
fn l_20_status_update_while_running_works() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    seed_pipeline_session_with_workflows(
        &context,
        "pipeline-l-20",
        PipelineDefinition {
            nodes: vec![wf("a"), wf("b")],
            edges: vec![edge("a", "b", EdgeType::Success)],
        },
        HashMap::from([
            state("a", NodeStatus::Active),
            state("b", NodeStatus::Pending),
        ]),
        1,
    );
    supervisor_spawn_test_task("pipeline-l-20").expect("spawn");
    let data: Value = serde_json::from_str(&pipeline(&context, &json!({ "sessionId": "pipeline-l-20", "nodeId": "a", "status": "completed", "conclusion": "done [[pipeline]]" })).expect("complete")).expect("parse");
    assert_eq!(data["data"]["ready"], json!(["b"]));
}

