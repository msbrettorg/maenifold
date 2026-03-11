// T-12.7: RTM NFR-1
#[path = "workflow_supervisor/support.rs"]
mod support;

use serde_json::json;
use support::*;

// T-W1-7: RTM FR-10.3
#[test]
fn workflow_start_writes_supervisor_frontmatter_fields() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = invoke_workflow(&env.cfg, json!({"workflowId": "wsup-alpha"}));
    let session_id = extract_workflow_session_id(&result);
    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&session_id));

    assert_eq!(yaml_string(&frontmatter, "phase"), "running");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");
}

// T-W1-7: RTM FR-10.1, FR-10.3, FR-10.4
#[test]
fn register_submachine_enters_waiting_state_without_advancing_step() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_result = invoke_workflow(&env.cfg, json!({"workflowId": "wsup-alpha"}));
    let workflow_session_id = extract_workflow_session_id(&workflow_result);
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    let step_before = yaml_usize(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "currentStep",
    );

    let register_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Starting analysis with [[hierarchical-state-machines]] and [[sequential-thinking]].",
            "submachineSessionId": thinking_session_id,
        }),
    );

    assert!(register_result.contains("Registered submachine"));
    assert!(register_result.contains(&thinking_session_id));
    assert!(register_result.contains("waiting"));

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter, "phase"), "waiting");
    assert_eq!(
        yaml_string(&frontmatter, "activeSubmachineType"),
        "sequential"
    );
    assert_eq!(
        yaml_string(&frontmatter, "activeSubmachineSessionId"),
        thinking_session_id
    );
    assert_eq!(yaml_usize(&frontmatter, "currentStep"), step_before);
}

// T-W1-7: RTM FR-10.4
#[test]
fn continue_while_waiting_on_active_submachine_blocks_and_does_not_advance_step() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering submachine with [[state-machine]] analysis.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    let step_after_register = yaml_usize(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "currentStep",
    );
    let block_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Trying to advance but submachine is still running [[workflow]].",
        }),
    );

    assert!(block_result.contains("waiting on submachine"));
    assert!(block_result.contains(&thinking_session_id));
    assert!(!block_result.contains("Step 2/"));
    assert_eq!(
        yaml_usize(
            &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
            "currentStep"
        ),
        step_after_register,
    );
}

// T-W1-7: RTM FR-10.1, FR-10.3, FR-10.4
#[test]
fn continue_after_submachine_completed_clears_supervisor_fields_and_resumes() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering [[sequential-thinking]] submachine.",
            "submachineSessionId": thinking_session_id,
        }),
    );
    update_thinking_session_status(&env.cfg, &thinking_session_id, "completed");

    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming with [[hierarchical-state-machines]] after submachine completed.",
        }),
    );

    assert!(!resume_result.contains("waiting on submachine"));
    assert!(!resume_result.contains("ERROR"));
    assert!(resume_result.contains("Step 2/"));

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter, "phase"), "running");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");
}

// T-W1-7: RTM FR-10.1
#[test]
fn continue_after_submachine_cancelled_clears_supervisor_fields_and_resumes() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering submachine for [[state-machine]] analysis.",
            "submachineSessionId": thinking_session_id,
        }),
    );
    update_thinking_session_status(&env.cfg, &thinking_session_id, "cancelled");

    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming after [[sequential-thinking]] submachine was cancelled.",
        }),
    );

    assert!(!resume_result.contains("waiting on submachine"));
    assert!(resume_result.contains("Step 2/"));
    assert_eq!(
        yaml_string(
            &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
            "phase"
        ),
        "running",
    );
}

// T-W1-7: RTM FR-10.2
#[test]
fn backward_compatibility_old_session_without_phase_field_proceeds_normally() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let session_id = create_legacy_workflow_session(&env.cfg);
    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&session_id));
    assert!(!frontmatter.contains_key("phase"));

    let result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": session_id,
            "response": "Step 1 done with [[workflow]] and [[backward-compatibility]] verification.",
        }),
    );

    assert!(!result.contains("ERROR"));
    assert!(result.contains("Step 2/"));
}
