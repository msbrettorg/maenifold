// T-12.7: RTM NFR-1
#[path = "workflow_supervisor/support.rs"]
mod support;

use std::fs;

use serde_json::json;
use support::*;

// T-W1-7: RTM FR-10.3
#[test]
fn unblock_after_completion_appends_resume_note_to_content() {
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
            "response": "Registering with [[state-machine]] tracking.",
            "submachineSessionId": thinking_session_id,
        }),
    );
    update_thinking_session_status(&env.cfg, &thinking_session_id, "completed");
    // Complements lifecycle::continue_after_submachine_completed_clears_supervisor_fields_and_resumes;
    // focuses on the body annotation write.
    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming after submachine with [[hierarchical-state-machines]] done.",
        }),
    );

    assert!(!resume_result.contains("waiting on submachine"));
    assert!(!resume_result.contains("ERROR"));
    assert!(resume_result.contains("Step 2/"));

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter, "phase"), "running");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");

    let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
    assert!(content.contains("Submachine Resumed"));
    assert!(content.contains(&format!("Submachine {} completed.", thinking_session_id)));
}

// T-W1-7: RTM FR-10.1
#[test]
fn abandoned_submachine_unblocks_workflow() {
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
            "response": "Registering submachine for [[hierarchical-state-machines]] analysis.",
            "submachineSessionId": thinking_session_id,
        }),
    );
    update_thinking_session_status(&env.cfg, &thinking_session_id, "abandoned");

    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming after [[sequential-thinking]] submachine was abandoned.",
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
fn missing_submachine_file_unblocks_workflow() {
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
            "response": "Registering submachine with [[state-machine]] and [[workflow]] tracking.",
            "submachineSessionId": thinking_session_id,
        }),
    );

    let waiting_frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&waiting_frontmatter, "phase"), "waiting");

    let submachine_path = path_for_uri(&env.cfg, &sequential_uri(&thinking_session_id));
    assert!(submachine_path.exists());
    fs::remove_file(&submachine_path).expect("delete submachine file");

    let resume_result = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming after [[hierarchical-state-machines]] submachine was deleted.",
        }),
    );

    assert!(!resume_result.contains("ERROR"));
    assert!(!resume_result.contains("waiting on submachine"));
    assert!(resume_result.contains("Step 2/"));

    let frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    let content = read_body(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter, "phase"), "running");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
    assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");
    assert!(content.contains("Submachine Cleared"));
    assert!(content.contains(&thinking_session_id));
    assert!(content.contains("deleted or expired"));
}

// T-W1-7: RTM FR-10.1
#[test]
fn re_registration_cycle_works_across_multiple_submachines() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let workflow_session_id = extract_workflow_session_id(&invoke_workflow(
        &env.cfg,
        json!({"workflowId": "wsup-alpha"}),
    ));

    let thinking_session_id_a = create_thinking_session(&env.cfg, "active");
    let step_before_a = yaml_usize(
        &read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id)),
        "currentStep",
    );
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Starting [[hierarchical-state-machines]] analysis with submachine A.",
            "submachineSessionId": thinking_session_id_a,
        }),
    );

    let frontmatter_after_reg_a = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter_after_reg_a, "phase"), "waiting");
    assert_eq!(
        yaml_usize(&frontmatter_after_reg_a, "currentStep"),
        step_before_a
    );

    update_thinking_session_status(&env.cfg, &thinking_session_id_a, "completed");
    let resume_result_a = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming [[workflow]] after submachine A completed — advancing step.",
        }),
    );
    assert!(!resume_result_a.contains("waiting on submachine"));
    assert!(resume_result_a.contains("Step 2/"));

    let frontmatter_after_resume_a =
        read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter_after_resume_a, "phase"), "running");
    assert_eq!(
        yaml_string(&frontmatter_after_resume_a, "activeSubmachineType"),
        ""
    );
    assert_eq!(
        yaml_string(&frontmatter_after_resume_a, "activeSubmachineSessionId"),
        ""
    );
    let step_after_resume_a = yaml_usize(&frontmatter_after_resume_a, "currentStep");
    assert!(
        step_after_resume_a > step_before_a,
        "step should advance after A-cycle resume"
    );

    let thinking_session_id_b = create_thinking_session(&env.cfg, "active");
    invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Starting [[sequential-thinking]] submachine B on step 2.",
            "submachineSessionId": thinking_session_id_b,
        }),
    );

    let frontmatter_after_reg_b = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter_after_reg_b, "phase"), "waiting");
    assert_eq!(
        yaml_string(&frontmatter_after_reg_b, "activeSubmachineSessionId"),
        thinking_session_id_b
    );
    assert_eq!(
        yaml_usize(&frontmatter_after_reg_b, "currentStep"),
        step_after_resume_a
    );

    update_thinking_session_status(&env.cfg, &thinking_session_id_b, "completed");
    let resume_result_b = invoke_workflow(
        &env.cfg,
        json!({
            "sessionId": workflow_session_id,
            "response": "Resuming [[hierarchical-state-machines]] after submachine B completed — advancing step.",
        }),
    );
    assert!(!resume_result_b.contains("waiting on submachine"));
    assert!(resume_result_b.contains("Step 3/"));

    let frontmatter_after_resume_b =
        read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
    assert_eq!(yaml_string(&frontmatter_after_resume_b, "phase"), "running");
    assert_eq!(
        yaml_string(&frontmatter_after_resume_b, "activeSubmachineSessionId"),
        ""
    );
    assert_eq!(
        yaml_string(&frontmatter_after_resume_b, "activeSubmachineType"),
        ""
    );
}
