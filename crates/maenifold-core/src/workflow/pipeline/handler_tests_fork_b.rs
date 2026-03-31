use super::*;

#[test]
#[serial(config)]
fn pipeline_join_fork_fork_from_leaf_creates_single_node_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let source_id = "pipeline-1735689600800";
    let new_id = "pipeline-1735689600801";

    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("workflow-agent".to_string()),
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "leaf".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: Some("workflow-agent".to_string()),
                    prompt: None,

                    timeout: None,
                },
            ],
            edges: vec![super::super::PipelineEdge {
                source: "root".to_string(),
                target: "leaf".to_string(),
                edge_type: super::super::EdgeType::Success,
                mapping: None,
            }],
        },
        HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600800-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "leaf".to_string(),
                NodeState {
                    session_id: "workflow-1735689600800-leaf".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        4,
    );

    let output = pipeline(
        &context,
        &json!({
            "sessionId": source_id,
            "fork": {
                "fromNodeId": "leaf",
                "newSessionId": new_id
            }
        }),
    )
    .expect("fork succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    let edges = data
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges array");

    assert_eq!(nodes.len(), 1);
    assert_eq!(edges.len(), 0);
    assert_eq!(
        nodes[0].get("nodeId").and_then(Value::as_str),
        Some(format!("fork-{new_id}-leaf").as_str())
    );
}

#[test]
#[serial(config)]
fn pipeline_join_fork_fork_prefixes_ids_and_creates_workflow_sessions() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let source_id = "pipeline-1735689600900";
    let new_id = "pipeline-1735689600901";

    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "solo".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("workflow-agent".to_string()),
                prompt: None,

                timeout: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "solo".to_string(),
            NodeState {
                session_id: "workflow-1735689600900-solo".to_string(),
                status: NodeStatus::Active,
                attempts: 9,
                thinking_session_id: None,
                output: None,
            },
        )]),
        1,
    );

    let output = pipeline(
        &context,
        &json!({
            "sessionId": source_id,
            "fork": {
                "fromNodeId": "solo",
                "newSessionId": new_id
            }
        }),
    )
    .expect("fork succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let node = data
        .get("nodes")
        .and_then(Value::as_array)
        .and_then(|nodes| nodes.first())
        .expect("node");
    let node_id = node.get("nodeId").and_then(Value::as_str).expect("node id");
    let workflow_session_id = node
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("workflow session id");

    assert_eq!(node_id, format!("fork-{new_id}-solo"));
    assert_eq!(node.get("status").and_then(Value::as_str), Some("pending"));
    assert_eq!(node.get("attempts").and_then(Value::as_u64), Some(0));
    assert!(
        file_exists(&workflow_session_uri(workflow_session_id), &context.config)
            .expect("workflow file exists")
    );

    let (workflow_frontmatter, _, _) =
        read_markdown(&workflow_session_uri(workflow_session_id), &context.config)
            .expect("read workflow");
    let workflow_frontmatter = workflow_frontmatter.expect("frontmatter");
    assert_eq!(
        workflow_frontmatter
            .get("pipelineId")
            .and_then(YamlValue::as_str),
        Some(new_id)
    );
    assert_eq!(
        workflow_frontmatter
            .get("nodeId")
            .and_then(YamlValue::as_str),
        Some(node_id)
    );
    assert_eq!(
        workflow_frontmatter
            .get("status")
            .and_then(YamlValue::as_str),
        Some("pending")
    );
}
