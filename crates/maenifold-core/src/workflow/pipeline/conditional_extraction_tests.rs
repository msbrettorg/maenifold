use std::collections::HashMap;

use serial_test::serial;

use crate::util::json::OutputFormat;

use super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::update_status::update_pipeline_node_status;
use super::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    compute_pipeline_status_with_definition, compute_ready_set,
};

fn workflow_node(id: &str) -> PipelineNode {
    PipelineNode::Workflow {
        id: id.to_string(),
        display_label: None,
        workflow_id: "test-workflow-a".to_string(),
        acp_client: Some(format!("agent-{id}")),
        prompt: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
        join_policy: None,
        input_schema: None,
        output_schema: None,
    }
}

fn state(node_id: &str, status: NodeStatus, output: Option<&str>) -> NodeState {
    NodeState {
        session_id: format!("workflow-1735689604000-{node_id}"),
        status,
        attempts: 1,
        started_at: None,
        completed_at: None,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
        output: output.map(str::to_string),
        input_artifact: None,
        output_artifact: None,
        failure_reason: None,
        retry_after: None,
        retry_count: None,
    }
}

fn verdict_definition() -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![
            workflow_node("source"),
            workflow_node("pass"),
            workflow_node("fail"),
        ],
        edges: vec![
            PipelineEdge {
                source: "source".to_string(),
                target: "pass".to_string(),
                edge_type: EdgeType::Conditional {
                    path: "verdict".to_string(),
                    equals: serde_json::json!("PASS"),
                },
                mode: Default::default(),
                mapping: None,
            },
            PipelineEdge {
                source: "source".to_string(),
                target: "fail".to_string(),
                edge_type: EdgeType::Conditional {
                    path: "verdict".to_string(),
                    equals: serde_json::json!("FAIL"),
                },
                mode: Default::default(),
                mapping: None,
            },
        ],
        inputs: None,
        input_schema: None,
        template_id: None,
    }
}

fn ready_for_output(output: &str) -> Vec<String> {
    let states = HashMap::from([
        (
            "source".to_string(),
            state("source", NodeStatus::Completed, Some(output)),
        ),
        ("pass".to_string(), state("pass", NodeStatus::Pending, None)),
        ("fail".to_string(), state("fail", NodeStatus::Pending, None)),
    ]);
    compute_ready_set(&verdict_definition(), &states)
}

#[test]
fn prose_then_json_pass_routes_pass_branch() {
    assert_eq!(
        ready_for_output("some prose\n{\"verdict\":\"PASS\"}"),
        vec!["pass"]
    );
}

#[test]
fn prose_then_json_fail_routes_fail_branch() {
    assert_eq!(
        ready_for_output("some prose\n{\"verdict\":\"FAIL\"}"),
        vec!["fail"]
    );
}

#[test]
fn fenced_json_pass_routes_pass_branch() {
    assert_eq!(
        ready_for_output("```json\n{\"verdict\":\"PASS\"}\n```"),
        vec!["pass"]
    );
}

#[test]
fn multiple_json_objects_use_last_valid_object() {
    assert_eq!(
        ready_for_output("{\"verdict\":\"FAIL\"}\nfinal:\n{\"verdict\":\"PASS\"}"),
        vec!["pass"]
    );
}

#[test]
#[serial]
fn no_json_marks_source_and_downstream_with_parse_failure_reason() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689604000-conditional-no-json";
    let definition = verdict_definition();
    let states = HashMap::from([
        (
            "source".to_string(),
            state("source", NodeStatus::Active, None),
        ),
        ("pass".to_string(), state("pass", NodeStatus::Pending, None)),
        ("fail".to_string(), state("fail", NodeStatus::Pending, None)),
    ]);
    seed_pipeline_session_with_workflows(&context, session_id, definition.clone(), states, 3);

    update_pipeline_node_status(
        &context,
        session_id,
        "source",
        "completed",
        Some("plain prose with no structured verdict"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("complete source");

    let (_, frontmatter, _) =
        super::session_read::read_pipeline_session(&context, session_id).expect("read pipeline");
    let node_states: HashMap<String, NodeState> =
        super::session_io::frontmatter_value(&frontmatter, "nodeStates").expect("node states");
    let reason = "conditional predicate could not parse JSON from prompt output";

    assert_eq!(node_states["source"].status, NodeStatus::Completed);
    assert!(
        node_states["source"]
            .failure_reason
            .as_deref()
            .is_some_and(|value| value.contains(reason))
    );
    assert_eq!(node_states["pass"].status, NodeStatus::Abandoned);
    assert_eq!(node_states["fail"].status, NodeStatus::Abandoned);
    assert!(
        node_states["pass"]
            .failure_reason
            .as_deref()
            .is_some_and(|value| value.contains("(source node: source)"))
    );
    assert_eq!(
        compute_pipeline_status_with_definition(&definition, &node_states),
        "abandoned"
    );
}
