mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::{Value, json};
use std::{thread::sleep, time::Duration};
use tempfile::TempDir;

fn setup(definition: Value) -> (TempDir, PipelineHarness, String) {
    let (tmp, mut p) = PipelineHarness::new();
    let id = PipelineHarness::id(&p.create(definition));
    (tmp, p, id)
}

fn single(node: Value) -> Value {
    json!({ "nodes": [node], "edges": [] })
}

fn node<'a>(view: &'a Value, id: &str) -> &'a Value {
    view["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["nodeId"].as_str() == Some(id) || n["id"].as_str() == Some(id))
        .expect("node")
}

fn pause_after_run(p: &mut PipelineHarness, id: &str) -> Value {
    p.pipeline_ok(json!({ "sessionId": id, "run": true, "prompt": "ambient prompt" }));
    for _ in 0..20 {
        let resp = p.pipeline(json!({ "sessionId": id, "pause": true }));
        let result = &resp["result"];
        if result["isError"] != true {
            return PipelineHarness::parse_data(result);
        }
        let text = PipelineHarness::extract_text(result);
        assert!(text.contains("isn't running"), "unexpected pause error: {text}");
        sleep(Duration::from_millis(50));
    }
    panic!("pipeline never became pausable");
}

#[test]
fn tc_8_01_restart_abandoned_node() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    let before = node(&p.view(&id), "A")["sessionId"].clone();
    p.dispatch(&id);
    p.abandon(&id, "A");
    let restarted = p.restart(&id, "A");
    let after = node(&restarted, "A");
    assert_eq!(after["status"], "pending");
    assert_ne!(after["sessionId"], before);
}

#[test]
fn tc_8_02_restart_error_on_pending_node() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    let error = p.restart_err(&id, "A");
    assert!(error.contains("Node A is pending"));
    assert!(error.contains("only abandoned or stuck nodes can be restarted"));
}

#[test]
fn tc_8_03_restart_error_on_active_node() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    p.dispatch(&id);
    let error = p.restart_err(&id, "A");
    assert!(error.contains("Node A is active"));
    assert!(error.contains("only abandoned or stuck nodes can be restarted"));
}

#[test]
fn tc_8_04_restart_error_on_completed_node() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    p.dispatch(&id);
    p.complete(&id, "A", "done");
    let error = p.restart_err(&id, "A");
    assert!(error.contains("Node A is completed"));
    assert!(error.contains("only abandoned or stuck nodes can be restarted"));
}

#[test]
fn tc_8_05_restart_increments_attempts_counter() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    p.dispatch(&id);
    p.abandon(&id, "A");
    assert_eq!(node(&p.restart(&id, "A"), "A")["attempts"], 1);
}

#[test]
fn tc_8_06_view_shows_node_specific_fields() {
    let def = json!({
        "nodes": [
            { "id": "w1", "workflowId": "workflow-dispatch", "prompt": "wf prompt", "timeout": 30 },
            { "id": "p1", "type": "prompt", "prompt": "prompt text", "agent": "test-agent" },
            { "id": "s1", "type": "sequentialThinking", "topic": "topic", "agent": "test-agent" },
            { "id": "t1", "type": "tool", "toolName": "MemoryStatus" }
        ],
        "edges": [
            PipelineHarness::edge("w1", "p1"),
            PipelineHarness::edge("p1", "s1"),
            PipelineHarness::edge("s1", "t1")
        ]
    });
    let (_tmp, mut p, id) = setup(def);
    let view = p.view(&id);
    assert_eq!(node(&view, "w1")["workflowId"], "workflow-dispatch");
    assert_eq!(node(&view, "w1")["prompt"], "wf prompt");
    assert_eq!(node(&view, "p1")["prompt"], "prompt text");
    assert_eq!(node(&view, "s1")["topic"], "topic");
    assert_eq!(node(&view, "t1")["toolName"], "MemoryStatus");
}

#[test]
fn tc_8_07_view_shows_agent_config_fields() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1",
        "type": "prompt",
        "prompt": "Test",
        "agent": "test-agent",
        "agentConfig": { "model": "gpt-4", "mode": "interactive", "thoughtLevel": "high" }
    })));
    assert_eq!(
        node(&p.view(&id), "p1")["agentConfig"],
        json!({ "model": "gpt-4", "mode": "interactive", "thoughtLevel": "high" })
    );
}

#[test]
fn tc_8_08_view_shows_config_options() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1",
        "type": "prompt",
        "prompt": "Test",
        "agent": "test-agent",
        "agentConfig": {
            "model": "gpt-4",
            "configOptions": { "sandbox": "workspace-write", "temperature": 0 }
        }
    })));
    assert_eq!(
        node(&p.view(&id), "p1")["agentConfig"]["configOptions"],
        json!({ "sandbox": "workspace-write", "temperature": 0 })
    );
}

#[test]
fn tc_8_09_prompt_node_with_all_optional_fields() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1",
        "type": "prompt",
        "prompt": "Test",
        "agent": "test-agent",
        "model": "gpt-5.4-mini",
        "thoughtLevel": "high",
        "maxTurns": 7,
        "toolMode": "auto"
    })));
    let view = p.view(&id);
    let n = node(&view, "p1");
    assert_eq!(n["model"], "gpt-5.4-mini");
    assert_eq!(n["thoughtLevel"], "high");
    assert_eq!(n["maxTurns"], 7);
    assert_eq!(n["toolMode"], "auto");
}

#[test]
fn tc_8_10_sequentialthinking_node_with_all_optional_fields() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "s1",
        "type": "sequentialThinking",
        "topic": "Trace restart behavior",
        "agent": "test-agent",
        "model": "claude-sonnet-4.6",
        "analysisType": "diagnostic",
        "thoughtLevel": "deep"
    })));
    let view = p.view(&id);
    let n = node(&view, "s1");
    assert_eq!(n["model"], "claude-sonnet-4.6");
    assert_eq!(n["analysisType"], "diagnostic");
    assert_eq!(n["thoughtLevel"], "deep");
}

#[test]
fn tc_8_11_tool_node_with_toolargs_displayed_in_view() {
    let args = json!({ "query": "pipeline testing", "mode": "Hybrid", "pageSize": 5 });
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "t1",
        "type": "tool",
        "toolName": "SearchMemories",
        "toolArgs": args.clone()
    })));
    assert_eq!(node(&p.view(&id), "t1")["toolArgs"], args);
}

#[test]
#[ignore = "BUG: pipeline view does not expose the stored ambient prompt directly"]
// BUG: ambient prompt is persisted by run(), but view() omits it as a first-class field.
fn tc_8_12_ambient_prompt_stored_and_visible() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1", "type": "prompt", "prompt": "Test", "agent": "test-agent"
    })));
    p.pipeline_ok(json!({ "sessionId": &id, "run": true, "prompt": "ambient prompt" }));
    assert_eq!(p.view(&id)["prompt"], "ambient prompt");
}

#[test]
fn tc_8_13_pause_pipeline() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1", "type": "prompt", "prompt": "Test", "agent": "test-agent"
    })));
    assert_eq!(pause_after_run(&mut p, &id)["status"], "paused");
}

#[test]
#[ignore = "BUG: MCP resume can fail after pause with invalid session file format"]
// BUG: resuming a paused pipeline via the MCP harness currently errors with
// "Invalid session file format" instead of returning a resumed response.
fn tc_8_14_resume_paused_pipeline() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1", "type": "prompt", "prompt": "Test", "agent": "test-agent"
    })));
    pause_after_run(&mut p, &id);
    assert_eq!(
        p.pipeline_ok(json!({ "sessionId": &id, "resume": true }))["status"],
        "resumed"
    );
}
