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
        assert!(
            text.contains("isn't running"),
            "unexpected pause error: {text}"
        );
        sleep(Duration::from_millis(50));
    }
    panic!("pipeline never became pausable");
}

#[path = "pipeline_mcp_config_extra/mod.rs"]
mod pipeline_mcp_config_b;

// Spec TC-8.1 + TC-8.2.
#[test]
fn tc_8_01_restart_abandoned_node() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    p.dispatch(&id);
    let before = node(&p.view(&id), "A")["sessionId"].clone();
    p.abandon(&id, "A");
    let restarted = p.restart(&id, "A");
    let after = node(&restarted, "A");
    assert_eq!(after["status"], "pending");
    assert_eq!(after["attempts"], 1);
    assert_ne!(
        after["sessionId"], before,
        "restart must allocate a fresh sessionId"
    );
}

// Spec TC-8.4.
#[test]
fn tc_8_02_restart_error_on_pending_node() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    let error = p.restart_err(&id, "A");
    assert!(error.contains("Node A is pending"));
    assert!(error.contains("only abandoned or stuck nodes can be restarted"));
}

// Spec TC-8.5.
#[test]
fn tc_8_03_restart_error_on_active_node() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    p.dispatch(&id);
    let error = p.restart_err(&id, "A");
    assert!(error.contains("Node A is active"));
    assert!(error.contains("only abandoned or stuck nodes can be restarted"));
}

// Spec TC-8.3.
#[test]
fn tc_8_04_restart_error_on_completed_node() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    p.dispatch(&id);
    p.complete(&id, "A", "done");
    let error = p.restart_err(&id, "A");
    assert!(error.contains("Node A is completed"));
    assert!(error.contains("only abandoned or stuck nodes can be restarted"));
}

// Spec TC-8.1 dedicated attempts assertion.
#[test]
fn tc_8_05_restart_increments_attempts_counter() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    p.dispatch(&id);
    p.abandon(&id, "A");
    assert_eq!(node(&p.restart(&id, "A"), "A")["attempts"], 1);
}

// Spec TC-8.6. Ignored until MCP behavior matches the spec's third-restart rejection.
#[test]
#[ignore = "BUG: max attempts not implemented"]
fn spec_tc_8_06_max_attempts_becomes_stuck() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    for _ in 0..2 {
        p.dispatch(&id);
        p.abandon(&id, "A");
        p.restart(&id, "A");
    }
    p.dispatch(&id);
    p.abandon(&id, "A");
    let error = p.restart_err(&id, "A");
    assert!(error.contains("max attempts"));
    assert_eq!(node(&p.view(&id), "A")["status"], "stuck");
}

// Spec TC-8.7. Ignored until MCP behavior exposes the spec's Stuck transition path.
#[test]
#[ignore = "BUG: max attempts not implemented"]
fn spec_tc_8_07_stuck_node_visible_in_view() {
    let (_tmp, mut p, id) = setup(single(PipelineHarness::tool_node("A")));
    for _ in 0..2 {
        p.dispatch(&id);
        p.abandon(&id, "A");
        p.restart(&id, "A");
    }
    p.dispatch(&id);
    p.abandon(&id, "A");
    let _ = p.pipeline(json!({ "sessionId": &id, "nodeId": "A", "restart": true }));
    assert_eq!(node(&p.view(&id), "A")["status"], "stuck");
}

// Non-spec regression: verify view exposes type-specific fields across node kinds.
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

// Spec TC-8.8.
#[test]
fn tc_8_07_view_shows_agent_config_fields() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1",
        "type": "prompt",
        "prompt": "Test",
        "agent": "test-agent",
        "agentConfig": {
            "model": "gpt-4",
            "mode": "interactive",
            "thoughtLevel": "high",
            "configOptions": { "sandbox": "workspace-write", "temperature": 0 }
        }
    })));
    assert_eq!(
        node(&p.view(&id), "p1")["agentConfig"],
        json!({
            "model": "gpt-4",
            "mode": "interactive",
            "thoughtLevel": "high",
            "configOptions": { "sandbox": "workspace-write", "temperature": 0 }
        })
    );
}

// Spec TC-8.9.
#[test]
fn tc_8_10_sequentialthinking_node_with_all_optional_fields() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "s1",
        "type": "sequentialThinking",
        "topic": "Trace restart behavior",
        "agent": "test-agent",
        "model": "claude-sonnet-4.6",
        "analysisType": "diagnostic",
        "thoughtLevel": "deep",
        "agentConfig": { "model": "claude-sonnet-4.5", "mode": "batch", "thoughtLevel": "high" }
    })));
    let view = p.view(&id);
    let n = node(&view, "s1");
    assert_eq!(n["model"], "claude-sonnet-4.6");
    assert_eq!(n["analysisType"], "diagnostic");
    assert_eq!(n["thoughtLevel"], "deep");
    assert_eq!(
        n["agentConfig"],
        json!({ "model": "claude-sonnet-4.5", "mode": "batch", "thoughtLevel": "high" })
    );
}

// Spec TC-8.10.
#[test]
fn tc_8_09_prompt_node_with_all_optional_fields() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "p1",
        "type": "prompt",
        "prompt": "Test",
        "agent": "test-agent",
        "model": "node-level-model",
        "thoughtLevel": "high",
        "maxTurns": 7,
        "toolMode": "auto",
        "agentConfig": { "model": "config-level-model", "mode": "interactive" }
    })));
    let view = p.view(&id);
    let n = node(&view, "p1");
    assert_eq!(n["model"], "node-level-model");
    assert_eq!(n["thoughtLevel"], "high");
    assert_eq!(n["maxTurns"], 7);
    assert_eq!(n["toolMode"], "auto");
    assert_eq!(
        n["agentConfig"],
        json!({ "model": "config-level-model", "mode": "interactive" })
    );
}

// Spec TC-8.11.
#[test]
fn tc_8_11_prompt_tool_mode_variations() {
    let def = json!({
        "nodes": [
            { "id": "p_auto", "type": "prompt", "prompt": "A", "agent": "test-agent", "toolMode": "auto" },
            { "id": "p_default", "type": "prompt", "prompt": "B", "agent": "test-agent" }
        ],
        "edges": []
    });
    let (_tmp, mut p, id) = setup(def);
    let view = p.view(&id);
    assert_eq!(node(&view, "p_auto")["toolMode"], "auto");
    assert_eq!(node(&view, "p_default")["toolMode"], "disabled");
}

// Spec TC-8.12.
#[test]
fn tc_8_12_sequentialthinking_defaults() {
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "s1", "type": "sequentialThinking", "topic": "Trace defaults", "agent": "test-agent"
    })));
    assert_eq!(node(&p.view(&id), "s1")["totalThoughts"], 5);
}

// Spec TC-8.13.
#[test]
fn tc_8_13_workflow_node_prompt_optional() {
    let def = json!({
        "nodes": [
            { "id": "w1", "workflowId": "workflow-dispatch", "prompt": "with prompt" },
            { "id": "w2", "workflowId": "workflow-dispatch" }
        ],
        "edges": []
    });
    let (_tmp, mut p, id) = setup(def);
    let view = p.view(&id);
    assert_eq!(node(&view, "w1")["prompt"], "with prompt");
    assert_eq!(node(&view, "w2")["prompt"], Value::Null);
}

// Spec TC-8.14.
#[test]
fn tc_8_14_tool_node_with_toolargs_displayed_in_view() {
    let args = json!({ "query": "pipeline testing", "mode": "Hybrid", "pageSize": 5 });
    let (_tmp, mut p, id) = setup(single(json!({
        "id": "t1", "type": "tool", "toolName": "SearchMemories", "toolArgs": args.clone()
    })));
    assert_eq!(node(&p.view(&id), "t1")["toolArgs"], args);
}
