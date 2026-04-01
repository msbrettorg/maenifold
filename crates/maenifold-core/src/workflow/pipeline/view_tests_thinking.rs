use super::*;
use crate::markdown::read_markdown;
use crate::thinking::sequential_session_uri;
use crate::thinking::sequential_thinking;
use crate::workflow::pipeline::test_support::seed_pipeline_session_with_node_sessions;
use serde_json::json;

#[test]
#[serial(config)]
fn pipeline_view_includes_thinking_trace_for_st_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let wrapper_sid = "workflow-1735689600000-think";
    let completed_sid = "session-1735689600000-think";
    let pending_sid = "st-1735689600000-pending";

    let definition = PipelineDefinition {
        nodes: vec![
            thinking_node(
                "think-done",
                "architecture review",
                3,
                Some("architecture"),
                "agent-think",
            ),
            thinking_node("think-pending", "pending analysis", 2, None, "agent-think"),
        ],
        edges: vec![edge("think-done", "think-pending", EdgeType::Success)],
    };
    let node_states = HashMap::from([
        (
            "think-done".to_string(),
            node_state_with_thinking_session(
                wrapper_sid,
                NodeStatus::Completed,
                1,
                Some(completed_sid),
                Some("conclusion text"),
            ),
        ),
        (
            "think-pending".to_string(),
            node_state(pending_sid, NodeStatus::Pending, 0, None),
        ),
    ]);

    let uri = sequential_session_uri(completed_sid);
    append_h2_section(
        &uri,
        "Thought 1/3 [agent-think]",
        "First thought about [[architecture]].\n\n*2025-01-01 00:00:01 UTC*\n",
        &context.config,
    )
    .expect("write thought 1");
    append_h2_section(
        &uri,
        "Thought 2/3 [agent-think] (Revises: 1)",
        "Revised first thought.\n\n*2025-01-01 00:00:02 UTC*\n",
        &context.config,
    )
    .expect("write thought 2");
    append_h2_section(
        &uri,
        "Thought 3/3 [agent-think] (Branch: alt-path)",
        "Branched thought.\n\n*2025-01-01 00:00:03 UTC*\n",
        &context.config,
    )
    .expect("write thought 3");
    append_h2_section(&uri, "Conclusion", "Final conclusion.", &context.config)
        .expect("write conclusion");

    let result = build_pipeline_view_result(
        &context,
        "pipeline-thinking-trace",
        &definition,
        &node_states,
        None,
    )
    .expect("build pipeline view");
    let nodes = result
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");

    let completed_node = &nodes[0];
    assert_eq!(completed_node["sessionId"], completed_sid);
    let trace = completed_node
        .get("thinking_trace")
        .expect("thinking_trace present");
    let trace_array = trace.as_array().expect("thinking_trace is array");
    assert_eq!(trace_array.len(), 3, "Conclusion section must be excluded");

    assert_eq!(trace_array[0]["thought_number"], 1);
    assert_eq!(trace_array[0]["total_thoughts"], 3);
    assert_eq!(trace_array[0]["agent"], "agent-think");
    assert_eq!(trace_array[0]["is_revision"], false);
    assert_eq!(trace_array[0]["revises_thought"], Value::Null);
    assert_eq!(trace_array[0]["branch_id"], Value::Null);
    assert!(
        trace_array[0]["text"]
            .as_str()
            .unwrap()
            .contains("architecture")
    );

    assert_eq!(trace_array[1]["thought_number"], 2);
    assert_eq!(trace_array[1]["is_revision"], true);
    assert_eq!(trace_array[1]["revises_thought"], 1);
    assert_eq!(trace_array[1]["branch_id"], Value::Null);

    assert_eq!(trace_array[2]["thought_number"], 3);
    assert_eq!(trace_array[2]["is_revision"], false);
    assert_eq!(trace_array[2]["revises_thought"], Value::Null);
    assert_eq!(trace_array[2]["branch_id"], "alt-path");

    let pending_node = &nodes[1];
    assert_eq!(
        pending_node.get("thinking_trace"),
        Some(&Value::Null),
        "pending node must have null thinking_trace"
    );
}

#[test]
#[serial(config)]
fn pipeline_view_resolves_active_thinking_trace_from_wrapper_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let wrapper_sid = "workflow-1735689601000-think";
    let pipeline_id = "pipeline-thinking-wrapper-trace";
    let definition = PipelineDefinition {
        nodes: vec![thinking_node(
            "think-active",
            "wrapper trace lookup",
            2,
            Some("architecture"),
            "agent-think",
        )],
        edges: vec![],
    };
    let node_states = HashMap::from([(
        "think-active".to_string(),
        node_state(wrapper_sid, NodeStatus::Active, 1, None),
    )]);
    seed_pipeline_session_with_node_sessions(
        &context,
        pipeline_id,
        definition.clone(),
        node_states.clone(),
        8,
    );

    let created = serde_json::from_str::<Value>(
        &sequential_thinking(
            &context,
            &json!({
                "thoughtNumber": 0,
                "totalThoughts": 2,
                "nextThoughtNeeded": true,
                "response": "Investigating [[pipeline]] wrapper trace lookup.",
                "parentWorkflowId": wrapper_sid,
            }),
        )
        .expect("create sequential thinking session"),
    )
    .expect("parse sequential thinking response");
    let actual_session_id = created["data"]["sessionId"]
        .as_str()
        .expect("session id")
        .to_string();

    let result = build_pipeline_view_result(&context, pipeline_id, &definition, &node_states, None)
        .expect("build pipeline view");
    let nodes = result["nodes"].as_array().expect("nodes array");
    let active_node = &nodes[0];
    let trace = active_node["thinking_trace"]
        .as_array()
        .expect("thinking trace array");

    assert_eq!(active_node["sessionId"], wrapper_sid);
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0]["thought_number"], 0);
    assert_eq!(trace[0]["total_thoughts"], 2);
    assert!(
        trace[0]["text"]
            .as_str()
            .expect("trace text")
            .contains("[[pipeline]]")
    );

    let uri = sequential_session_uri(&actual_session_id);
    let (frontmatter, _, _) = read_markdown(&uri, &context.config).expect("read thinking session");
    let expected_parent = format!("[[workflow/{wrapper_sid}]]");
    assert_eq!(
        frontmatter
            .expect("thinking frontmatter")
            .get("parent")
            .and_then(serde_yaml::Value::as_str),
        Some(expected_parent.as_str())
    );
}
