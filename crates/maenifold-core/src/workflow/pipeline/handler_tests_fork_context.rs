use super::*;

#[test]
#[serial(config)]
fn pipeline_fork_inherits_predecessor_context() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let source_id = "pipeline-1735689601000";
    let new_id = "pipeline-1735689601001";

    // Source pipeline: A → B → C
    // A is completed with output, B is active, C is pending.
    // Forking from B should give B' the output of A as predecessor context.
    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "A".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "B".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "C".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
            ],
            edges: vec![
                super::super::PipelineEdge {
                    source: "A".to_string(),
                    target: "B".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "B".to_string(),
                    target: "C".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                },
            ],
        },
        HashMap::from([
            (
                "A".to_string(),
                NodeState {
                    session_id: "workflow-1735689601000-A".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: Some("A produced this reasoning output".to_string()),
                },
            ),
            (
                "B".to_string(),
                NodeState {
                    session_id: "workflow-1735689601000-B".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "C".to_string(),
                NodeState {
                    session_id: "workflow-1735689601000-C".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        5,
    );

    let output = pipeline(
        &context,
        &json!({
            "sessionId": source_id,
            "fork": {
                "fromNodeId": "B",
                "newSessionId": new_id
            }
        }),
    )
    .expect("fork succeeds");

    // Parse the fork response — should contain B' and C' (not A).
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    assert_eq!(nodes.len(), 2, "forked subgraph should have B and C");

    // Read the forked pipeline session's YAML and verify predecessor context
    // was injected into B' (the fork root).
    let (frontmatter, _, _) = read_markdown(&pipeline_session_uri(new_id), &context.config)
        .expect("read forked pipeline");
    let frontmatter = frontmatter.expect("frontmatter");
    let node_states_yaml = frontmatter
        .get("nodeStates")
        .expect("nodeStates in frontmatter");

    // The fork root is B' (remapped as fork-{new_id}-B).
    let fork_root_id = format!("fork-{new_id}-B");
    let fork_root_state = node_states_yaml
        .get(YamlValue::String(fork_root_id.clone()))
        .unwrap_or_else(|| panic!("node state for {fork_root_id} should exist"));

    let pred_context = fork_root_state
        .get("predecessorContext")
        .expect("fork root should have predecessorContext");
    let pred_list = pred_context
        .as_sequence()
        .expect("predecessorContext should be a list");
    assert_eq!(pred_list.len(), 1, "B had one completed predecessor (A)");
    let pred = &pred_list[0];
    assert_eq!(
        pred.get("nodeId").and_then(YamlValue::as_str),
        Some("A"),
        "predecessor should reference source node A"
    );
    assert_eq!(
        pred.get("output").and_then(YamlValue::as_str),
        Some("A produced this reasoning output"),
        "predecessor should carry A's output"
    );

    let fork_root_session_id = fork_root_state
        .get("sessionId")
        .and_then(YamlValue::as_str)
        .expect("fork root session id");
    let (_, fork_root_content, _) =
        read_markdown(&workflow_session_uri(fork_root_session_id), &context.config)
            .expect("read fork root workflow");
    assert!(
        fork_root_content.contains("## Predecessor Context"),
        "fork root workflow session should materialize predecessor context"
    );
    assert!(
        fork_root_content.contains("### From A\n\nA produced this reasoning output"),
        "fork root workflow session should include inherited predecessor output"
    );

    // C' (not a fork root — has B' as predecessor in the forked subgraph)
    // should NOT have predecessorContext.
    let fork_child_id = format!("fork-{new_id}-C");
    let fork_child_state = node_states_yaml
        .get(YamlValue::String(fork_child_id.clone()))
        .unwrap_or_else(|| panic!("node state for {fork_child_id} should exist"));
    assert!(
        fork_child_state.get("predecessorContext").is_none(),
        "non-root node C' should not have predecessorContext"
    );
}
