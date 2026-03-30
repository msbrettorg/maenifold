use crate::markdown::{read_markdown, Frontmatter};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

use super::handler::pipeline;
use super::session_io::frontmatter_value;
use super::test_support::{
    pipeline_test_context, seed_pipeline_session_with_node_sessions,
    seed_pipeline_session_with_workflows,
};
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};

#[test]
#[serial(config)]
fn pipeline_dispatch_smoke_parallel_roots_and_concurrency_limits() {
    // Covers D-01, D-02, D-12, CON-01, CON-02.
    for (pipeline_id, max_concurrency, expected_roots) in [
        ("pipeline-1735689600600", 2_u64, vec!["root-a", "root-b"]),
        ("pipeline-1735689600601", 1_u64, vec!["root-a"]),
        ("pipeline-1735689600602", 8_u64, vec!["root-a", "root-b"]),
    ] {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let definition = wf_def(
            &[
                ("root-a", "test-workflow-a"),
                ("root-b", "test-workflow-b"),
                ("leaf", "test-workflow-a"),
            ],
            &[("root-a", "leaf", EdgeType::Success)],
        );
        let states = states([
            (
                "root-a",
                &format!("{pipeline_id}-root-a"),
                NodeStatus::Pending,
            ),
            (
                "root-b",
                &format!("{pipeline_id}-root-b"),
                NodeStatus::Pending,
            ),
            ("leaf", &format!("{pipeline_id}-leaf"), NodeStatus::Pending),
        ]);
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            definition,
            states,
            max_concurrency,
        );

        // Order follows definition.nodes Vec — deterministic by construction
        assert_eq!(
            dispatched_ids(&dispatch_data(&context, pipeline_id)),
            expected_roots
        );
        assert_eq!(
            read_pipeline_states(&context, pipeline_id)
                .values()
                .filter(|state| state.status == NodeStatus::Active)
                .count(),
            expected_roots.len()
        );
    }
}

#[test]
#[serial(config)]
fn pipeline_dispatch_returns_empty_when_nothing_is_eligible() {
    // Covers D-03 and D-04.
    let scenarios = [
        (
            "pipeline-1735689600610",
            wf_def(
                &[("root", "test-workflow-a"), ("child", "test-workflow-b")],
                &[("root", "child", EdgeType::Success)],
            ),
            states([
                ("root", "workflow-1735689600610-root", NodeStatus::Active),
                ("child", "workflow-1735689600610-child", NodeStatus::Pending),
            ]),
            8_u64,
        ),
        (
            "pipeline-1735689600611",
            wf_def(
                &[("root", "test-workflow-a"), ("sibling", "test-workflow-b")],
                &[],
            ),
            states([
                ("root", "workflow-1735689600611-root", NodeStatus::Active),
                (
                    "sibling",
                    "workflow-1735689600611-sibling",
                    NodeStatus::Pending,
                ),
            ]),
            1_u64,
        ),
    ];

    for (pipeline_id, definition, states, max_concurrency) in scenarios {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            definition,
            states,
            max_concurrency,
        );

        assert!(dispatch_data(&context, pipeline_id)["dispatched"]
            .as_array()
            .expect("dispatched")
            .is_empty());
    }
}

#[test]
#[serial(config)]
fn pipeline_dispatch_smoke_non_workflow_metadata_variants() {
    // Covers D-05, D-06, and D-07.
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600620";
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Prompt {
                id: "prompt".into(),
                prompt: "Summarize dispatch".into(),
                agent: "prompt-agent".into(),
                tool_mode: "default".into(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                timeout: Some(30),
                agent_config: None,
            },
            PipelineNode::SequentialThinking {
                id: "think".into(),
                topic: "dispatch reasoning".into(),
                total_thoughts: 3,
                analysis_type: Some("architecture".into()),
                agent: "think-agent".into(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                timeout: Some(45),
                agent_config: None,
            },
            PipelineNode::Tool {
                id: "tool".into(),
                tool_name: "ReadMemory".into(),
                tool_args: Some(json!({ "identifier": "memory://build/test" })),
                agent: Some("tool-agent".into()),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                agent_config: None,
            },
        ],
        edges: vec![],
    };
    let states = states([
        (
            "prompt",
            "workflow-1735689600620-prompt",
            NodeStatus::Pending,
        ),
        ("think", "workflow-1735689600620-think", NodeStatus::Pending),
        ("tool", "workflow-1735689600620-tool", NodeStatus::Pending),
    ]);
    seed_pipeline_session_with_node_sessions(&context, pipeline_id, definition, states, 8);

    assert_eq!(
        dispatch_data(&context, pipeline_id)["dispatched"],
        json!([
            {
                "nodeId": "prompt",
                "sessionId": "workflow-1735689600620-prompt",
                "type": "prompt",
                "agent": "prompt-agent"
            },
            {
                "nodeId": "think",
                "sessionId": "workflow-1735689600620-think",
                "type": "sequentialThinking",
                "agent": "think-agent"
            },
            {
                "nodeId": "tool",
                "sessionId": "workflow-1735689600620-tool",
                "type": "tool",
                "toolName": "ReadMemory",
                "toolArgs": { "identifier": "memory://build/test" },
                "agent": "tool-agent"
            }
        ])
    );

    for session_id in [
        "workflow-1735689600620-prompt",
        "workflow-1735689600620-think",
        "workflow-1735689600620-tool",
    ] {
        let (frontmatter, content) = read_session(&context, session_id);
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("active")
        );
        assert!(!content.contains("## Step"));
    }
}

#[test]
#[serial(config)]
fn pipeline_dispatch_is_idempotent_and_noops_after_completion() {
    // Covers D-08, D-09, D-11.
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600630";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        wf_def(&[("root", "test-workflow-a")], &[]),
        states([("root", "workflow-1735689600630-root", NodeStatus::Pending)]),
        8,
    );

    assert_eq!(
        dispatch_data(&context, pipeline_id)["dispatched"][0]["workflowId"],
        "test-workflow-a"
    );

    let (_, content) = read_session(&context, "workflow-1735689600630-root");
    assert_eq!(content.matches("## Step 1/1").count(), 1);

    assert!(dispatch_data(&context, pipeline_id)["dispatched"]
        .as_array()
        .expect("dispatched")
        .is_empty());

    pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "nodeId": "root",
            "status": "completed",
            "conclusion": "workflow done"
        }),
    )
    .expect("complete node");

    let completed_dispatch = dispatch_data(&context, pipeline_id);
    assert!(completed_dispatch["dispatched"]
        .as_array()
        .expect("dispatched")
        .is_empty());
    assert_eq!(completed_dispatch["remaining"], Value::from(0));
    assert_eq!(completed_dispatch["completed"], json!(["root"]));
}

#[test]
#[serial(config)]
fn pipeline_dispatch_releases_successive_waves_after_predecessors_complete() {
    // Covers D-10 and CON-03.
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600640";
    let definition = wf_def(
        &[
            ("root-a", "test-workflow-a"),
            ("root-b", "test-workflow-b"),
            ("leaf-a", "test-workflow-a"),
            ("leaf-b", "test-workflow-b"),
        ],
        &[
            ("root-a", "leaf-a", EdgeType::Success),
            ("root-b", "leaf-b", EdgeType::Success),
        ],
    );
    let states = states([
        (
            "root-a",
            "workflow-1735689600640-root-a",
            NodeStatus::Pending,
        ),
        (
            "root-b",
            "workflow-1735689600640-root-b",
            NodeStatus::Pending,
        ),
        (
            "leaf-a",
            "workflow-1735689600640-leaf-a",
            NodeStatus::Pending,
        ),
        (
            "leaf-b",
            "workflow-1735689600640-leaf-b",
            NodeStatus::Pending,
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, pipeline_id, definition, states, 2);

    assert_eq!(
        dispatch_data(&context, pipeline_id)["dispatched"]
            .as_array()
            .expect("dispatched")
            .len(),
        2
    );

    for node_id in ["root-a", "root-b"] {
        pipeline(
            &context,
            &json!({
                "sessionId": pipeline_id,
                "nodeId": node_id,
                "status": "completed",
                "conclusion": format!("{node_id} finished")
            }),
        )
        .expect("complete root");
    }

    // Order follows definition.nodes Vec — deterministic by construction
    assert_eq!(
        dispatched_ids(&dispatch_data(&context, pipeline_id)),
        vec!["leaf-a", "leaf-b"]
    );
}

fn dispatch_data(context: &crate::tools::ToolContext, pipeline_id: &str) -> Value {
    let output = pipeline(
        context,
        &json!({ "sessionId": pipeline_id, "dispatch": true }),
    )
    .expect("dispatch pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    response.get("data").cloned().expect("response data")
}

fn read_pipeline_states(
    context: &crate::tools::ToolContext,
    pipeline_id: &str,
) -> HashMap<String, NodeState> {
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    frontmatter_value(&frontmatter.expect("pipeline frontmatter"), "nodeStates")
        .expect("node states")
}

fn read_session(context: &crate::tools::ToolContext, session_id: &str) -> (Frontmatter, String) {
    let (frontmatter, content, _) =
        read_markdown(&workflow_session_uri(session_id), &context.config).expect("read session");
    (frontmatter.expect("session frontmatter"), content)
}

fn dispatched_ids(data: &Value) -> Vec<&str> {
    data["dispatched"]
        .as_array()
        .expect("dispatched")
        .iter()
        .map(|entry| entry["nodeId"].as_str().expect("nodeId"))
        .collect()
}

fn wf_def(nodes: &[(&str, &str)], edges: &[(&str, &str, EdgeType)]) -> PipelineDefinition {
    PipelineDefinition {
        nodes: nodes
            .iter()
            .map(|(id, workflow_id)| workflow_node(id, workflow_id))
            .collect(),
        edges: edges
            .iter()
            .map(|(source, target, edge_type)| edge(source, target, edge_type.clone()))
            .collect(),
    }
}

fn workflow_node(id: &str, workflow_id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.into(),
        workflow_id: workflow_id.into(),
        acp_client: None,
        prompt: None,
        timeout: None,
    }
}

fn edge(source: &str, target: &str, edge_type: EdgeType) -> PipelineEdge {
    PipelineEdge {
        source: source.into(),
        target: target.into(),
        edge_type,
        mapping: None,
    }
}

fn states<const N: usize>(entries: [(&str, &str, NodeStatus); N]) -> HashMap<String, NodeState> {
    entries
        .into_iter()
        .map(|(id, session_id, status)| (id.to_string(), node_state(session_id, status)))
        .collect()
}

fn node_state(session_id: &str, status: NodeStatus) -> NodeState {
    NodeState {
        session_id: session_id.into(),
        status,
        attempts: 0,
        thinking_session_id: None,
        output: None,
    }
}
