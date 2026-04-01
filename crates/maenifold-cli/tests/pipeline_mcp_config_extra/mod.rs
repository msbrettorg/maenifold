use super::*;

// Non-spec regression: ambient run prompt should remain visible in pipeline view.
#[test]
#[ignore = "BUG: pipeline view does not expose the stored ambient prompt directly"]
fn tc_8_15_ambient_prompt_stored_and_visible() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1", "type": "prompt", "prompt": "Test", "agent": "test-agent"
    })));
    p.pipeline_ok(json!({ "sessionId": &id, "run": true, "prompt": "ambient prompt" }));
    assert_eq!(p.view(&id)["prompt"], "ambient prompt");
}

// Non-spec regression: pause support should continue to work.
#[test]
fn tc_8_16_pause_pipeline() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1", "type": "prompt", "prompt": "Test", "agent": "test-agent"
    })));
    assert_eq!(pause_after_run(&mut p, &id)["status"], "paused");
}

// Non-spec regression: resume support should continue to work once MCP bug is fixed.
#[test]
#[ignore = "BUG: MCP resume can fail after pause with invalid session file format"]
fn tc_8_17_resume_paused_pipeline() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1", "type": "prompt", "prompt": "Test", "agent": "test-agent"
    })));
    pause_after_run(&mut p, &id);
    assert_eq!(
        p.pipeline_ok(json!({ "sessionId": &id, "resume": true }))["status"],
        "resumed"
    );
}
