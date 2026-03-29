use super::*;

#[test]
#[serial(config)]
fn pipeline_rewind_resets_specified_node_to_pending() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600061";
    let root_session_id = "workflow-1735689600061-a".to_string();
    let middle_session_id = "workflow-1735689600061-b".to_string();
    let leaf_session_id = "workflow-1735689600061-c".to_string();
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "c".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
            ],
            edges: vec![
                super::super::PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "b".to_string(),
                    target: "c".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                },
            ],
        },
        HashMap::from([
            (
                "a".to_string(),
                NodeState {
                    session_id: root_session_id.clone(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "b".to_string(),
                NodeState {
                    session_id: middle_session_id.clone(),
                    status: NodeStatus::Completed,
                    attempts: 2,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "c".to_string(),
                NodeState {
                    session_id: leaf_session_id.clone(),
                    status: NodeStatus::Pending,
                    attempts: 3,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        2,
    );

    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": ["b"] } }),
    )
    .expect("rewind succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let nodes = response["data"]["nodes"].as_array().expect("nodes array");
    let find_node = |node_id: &str| {
        nodes
            .iter()
            .find(|node| node.get("nodeId") == Some(&Value::String(node_id.to_string())))
            .expect("node exists")
    };

    let a = find_node("a");
    assert_eq!(a["sessionId"], root_session_id);
    assert_eq!(a["status"], "completed");
    assert_eq!(a["attempts"], 1);

    let b = find_node("b");
    let b_session_id = b["sessionId"].as_str().expect("b session");
    assert_ne!(b_session_id, middle_session_id);
    assert_eq!(b["status"], "pending");
    assert_eq!(b["attempts"], 0);
    assert!(file_exists(&workflow_session_uri(b_session_id), &context.config).expect("b workflow"));

    let c = find_node("c");
    let c_session_id = c["sessionId"].as_str().expect("c session");
    assert_ne!(c_session_id, leaf_session_id);
    assert_eq!(c["status"], "pending");
    assert_eq!(c["attempts"], 0);
    assert!(file_exists(&workflow_session_uri(c_session_id), &context.config).expect("c workflow"));
}

#[test]
#[serial(config)]
fn pipeline_rewind_cascades_downstream() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600062";
    let old_session_ids = HashMap::from([
        ("a".to_string(), "workflow-1735689600062-a".to_string()),
        ("b".to_string(), "workflow-1735689600062-b".to_string()),
        ("c".to_string(), "workflow-1735689600062-c".to_string()),
    ]);
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "c".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
            ],
            edges: vec![
                super::super::PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "b".to_string(),
                    target: "c".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                },
            ],
        },
        HashMap::from([
            (
                "a".to_string(),
                NodeState {
                    session_id: old_session_ids["a"].clone(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "b".to_string(),
                NodeState {
                    session_id: old_session_ids["b"].clone(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "c".to_string(),
                NodeState {
                    session_id: old_session_ids["c"].clone(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "completed");

    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": ["a"] } }),
    )
    .expect("rewind succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");

    assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
    for node in nodes {
        let node_id = node["nodeId"].as_str().expect("node id");
        let session_id = node["sessionId"].as_str().expect("session id");
        assert_ne!(session_id, old_session_ids[node_id]);
        assert_eq!(node["status"], "pending");
        assert_eq!(node["attempts"], 0);
        assert!(
            file_exists(&workflow_session_uri(session_id), &context.config)
                .expect("rewound workflow exists")
        );
    }
}

#[test]
#[serial(config)]
fn pipeline_rewind_clears_thinking_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600070";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,

                    timeout: None,
                },
            ],
            edges: vec![super::super::PipelineEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: super::super::EdgeType::Success,
                mapping: None,
            }],
        },
        HashMap::from([
            (
                "a".to_string(),
                NodeState {
                    session_id: "workflow-1735689600070-a".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: Some("thinking-1735689600070-a".to_string()),
                    output: Some("prior output".to_string()),
                },
            ),
            (
                "b".to_string(),
                NodeState {
                    session_id: "workflow-1735689600070-b".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: Some("thinking-1735689600070-b".to_string()),
                    output: Some("prior output b".to_string()),
                },
            ),
        ]),
        2,
    );

    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": ["a"] } }),
    )
    .expect("rewind succeeds");

    let (pipeline_frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
    let updated_states: HashMap<String, NodeState> =
        frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");

    // Both a and b (downstream) are rewound — thinking_session_id must be cleared
    assert!(
        updated_states["a"].thinking_session_id.is_none(),
        "node a thinking_session_id should be cleared after rewind"
    );
    assert!(
        updated_states["b"].thinking_session_id.is_none(),
        "node b thinking_session_id should be cleared after rewind"
    );
    // output cleared too
    assert!(updated_states["a"].output.is_none());
    assert!(updated_states["b"].output.is_none());
}
