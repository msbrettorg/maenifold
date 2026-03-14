// T-12.7: RTM NFR-1
#[path = "workflow_supervisor/support.rs"]
mod support;

use serde_json::json;
use support::*;

// T-W1-7: RTM FR-10.1
#[test]
fn register_nonexistent_submachine_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let fake_session_id = "session-9999999999999-00001";
    let phase_before = yaml_string(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "phase",
    );
    assert_eq!(phase_before, "running");

    let result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering with [[hierarchical-state-machines]] context.",
            "submachineSessionId": fake_session_id,
        }),
    );

    assert!(result.contains("ERROR"));
    assert!(result.contains(fake_session_id));
    assert_eq!(
        yaml_string(
            &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
            "phase"
        ),
        phase_before,
    );
}

// T-W1-7: RTM FR-10.3
#[test]
fn register_submachine_appends_note_to_session_content() {
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
            "response": "Registering submachine with [[workflow]] tracking.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
    assert!(content.contains("Submachine Registered"));
    assert!(content.contains(&thinking_session_id));
    assert!(content.contains("sequential"));
}

// T-W1-7: RTM FR-10.3, FR-10.4
#[test]
fn response_persisted_during_register() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    let response =
        "Analysis complete for [[hierarchical-state-machines]] and [[workflow]] at step boundary.";
    let step_before = yaml_usize(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "currentStep",
    );

    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": response,
            "submachineSessionId": thinking_session_id,
        }),
    );

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
    assert!(content.contains("Analysis complete for"));
    assert!(content.contains("[[hierarchical-state-machines]]"));
    assert_eq!(yaml_usize(&frontmatter, "currentStep"), step_before);
    assert_eq!(yaml_string(&frontmatter, "phase"), "waiting");
}

// T-W1-7: RTM FR-10.3
#[test]
fn register_path_enforces_wikilink_validation_on_response() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let thinking_session_id = create_thinking_session(&env.cfg, "active");
    let phase_before = yaml_string(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "phase",
    );
    assert_eq!(phase_before, "running");

    let result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "No double bracket concepts in this response at all.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    assert!(result.contains("ERROR"));
    assert!(result.contains(WIKILINK_ERROR_HINT));
    assert_eq!(
        yaml_string(
            &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
            "phase"
        ),
        phase_before,
    );
}

// T-W1-7: RTM FR-10.4
#[test]
fn double_registration_while_waiting_is_blocked() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));
    let first_session_id = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Registering first [[hierarchical-state-machines]] submachine.",
            "submachineSessionId": first_session_id,
        }),
    );

    let after_first = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&after_first, "phase"), "waiting");
    assert_eq!(
        yaml_string(&after_first, "activeSubmachineSessionId"),
        first_session_id
    );

    let second_session_id = create_thinking_session(&env.cfg, "active");
    let second_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Attempting to register second [[sequential-thinking]] submachine while waiting.",
            "submachineSessionId": second_session_id,
        }),
    );

    assert!(second_result.contains("waiting on submachine"));
    let after_second = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&after_second, "phase"), "waiting");
    assert_eq!(
        yaml_string(&after_second, "activeSubmachineSessionId"),
        first_session_id
    );
}

// T-W1-7: RTM FR-10.4
#[test]
fn status_parameter_while_waiting_is_blocked() {
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
            "response": "Registering [[hierarchical-state-machines]] submachine before attempting completion.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    let after_register = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&after_register, "phase"), "waiting");

    let complete_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Trying to complete [[workflow]] while submachine is active.",
            "status": "completed",
            "conclusion": "Attempting completion with [[hierarchical-state-machines]] conclusion.",
        }),
    );
    assert!(complete_result.contains("waiting on submachine"));

    let cancel_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Trying to cancel [[workflow]] while submachine is active.",
            "status": "cancelled",
        }),
    );
    assert!(cancel_result.contains("waiting on submachine"));

    let after_attempts = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&after_attempts, "status"), "active");
    assert_eq!(yaml_string(&after_attempts, "phase"), "waiting");
}
