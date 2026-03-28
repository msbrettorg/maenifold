use crate::markdown::read_markdown;
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

use super::supervisor::{
    mark_pipeline_stuck, supervisor_cancel, supervisor_detect_orphans, supervisor_is_paused,
    supervisor_is_running, supervisor_pause, supervisor_resume,
    supervisor_spawn_test_task, supervisor_status,
};
use super::supervisor_test_support::{
    pending_node_state, reset_supervisor, single_workflow_definition, wait_for,
};
use super::test_support::{
    pipeline_test_context, seed_pipeline_session_with_workflows,
};
use super::{NodeState, NodeStatus};

#[test]
#[serial(config)]
fn supervisor_cancel_triggers_clean_shutdown() {
    let (_tmp, _context, _config_reset) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let pipeline_id = "pipeline-supervisor-cancel";

    supervisor_spawn_test_task(pipeline_id).expect("spawn tracked task");
    supervisor_cancel(pipeline_id).expect("cancel pipeline");

    assert!(!supervisor_is_running(pipeline_id));
    assert!(supervisor_status(pipeline_id).is_none());
}

#[test]
#[serial(config)]
fn supervisor_pause_and_resume_toggle_flag_and_gate_dispatch() {
    let (_tmp, _context, _config_reset) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let pipeline_id = "pipeline-supervisor-pause";

    supervisor_spawn_test_task(pipeline_id).expect("spawn tracked task");
    supervisor_pause(pipeline_id).expect("pause pipeline");
    wait_for(|| supervisor_is_paused(pipeline_id) == Some(true));

    supervisor_resume(pipeline_id).expect("resume pipeline");
    wait_for(|| supervisor_is_paused(pipeline_id) == Some(false));
    assert!(supervisor_is_running(pipeline_id));
    supervisor_cancel(pipeline_id).expect("cleanup running pipeline");
}

#[test]
#[serial(config)]
fn supervisor_marks_pipeline_stuck_when_retries_exhausted() {
    let (_tmp, context, _config_reset) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let pipeline_id = "pipeline-1735689600999";
    let workflow_id = "workflow-1735689600999-root";
    let node_states = HashMap::from([(
        "root".to_string(),
        NodeState {
            session_id: workflow_id.to_string(),
            status: NodeStatus::Active,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )]);
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        single_workflow_definition("test-workflow-a", None),
        node_states,
        1,
    );

    mark_pipeline_stuck(context.config.maenifold_root.as_path(), pipeline_id);

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("stuck")
    );

    let (workflow_frontmatter, _, _) =
        read_markdown(&workflow_session_uri(workflow_id), &context.config).expect("read workflow");
    let workflow_frontmatter = workflow_frontmatter.expect("workflow frontmatter");
    assert_eq!(
        workflow_frontmatter
            .get("status")
            .and_then(YamlValue::as_str),
        Some("stuck")
    );
}

#[test]
#[serial(config)]
fn pipeline_detect_orphans_marks_stuck() {
    let (_tmp, context, _config_reset) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let pipeline_id = "pipeline-1735689601000";
    let workflow_id = "workflow-1735689601000-root";
    let node_states = HashMap::from([(
        "root".to_string(),
        NodeState {
            session_id: workflow_id.to_string(),
            status: NodeStatus::Active,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )]);
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        single_workflow_definition("test-workflow-a", None),
        node_states,
        1,
    );

    assert_eq!(
        supervisor_detect_orphans(context.config.maenifold_root.as_path()),
        1
    );

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("stuck")
    );

    let (workflow_frontmatter, _, _) =
        read_markdown(&workflow_session_uri(workflow_id), &context.config).expect("read workflow");
    let workflow_frontmatter = workflow_frontmatter.expect("workflow frontmatter");
    assert_eq!(
        workflow_frontmatter
            .get("status")
            .and_then(YamlValue::as_str),
        Some("stuck")
    );
}
