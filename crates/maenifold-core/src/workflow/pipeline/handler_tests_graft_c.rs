use super::*;

#[test]
#[serial(config)]
fn pipeline_graft_completed_dependency_is_immediately_ready() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600054";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            
                timeout: None}],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600054-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "completed");

    let output = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "downstream", "workflowId": "test-workflow-b" }
                ],
                "edges": [
                    { "source": "root", "target": "downstream" }
                ]
            }
        }),
    )
    .expect("graft succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("ready").and_then(Value::as_array),
        Some(&vec![Value::String("downstream".to_string())])
    );
}

#[test]
#[serial(config)]
fn pipeline_graft_rejects_running_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600055";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            
                timeout: None}],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600055-root".to_string(),
                status: NodeStatus::Active,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "active");

    let error = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "child", "workflowId": "test-workflow-b" }
                ],
                "edges": [
                    { "source": "root", "target": "child" }
                ]
            }
        }),
    )
    .expect_err("running pipeline should fail");

    assert!(error.to_string().contains("running pipelines"));

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter, "definition").expect("pipeline definition");
    assert_eq!(definition.nodes.len(), 1);
}

#[test]
#[serial(config)]
fn pipeline_graft_paused_pipeline_is_allowed_and_preserves_status() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600056";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            
                timeout: None}],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600056-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "paused");

    pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "child", "workflowId": "test-workflow-b" }
                ],
                "edges": [
                    { "source": "root", "target": "child" }
                ]
            }
        }),
    )
    .expect("paused graft succeeds");

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("paused")
    );
}

#[test]
#[serial(config)]
fn pipeline_graft_stuck_pipeline_is_allowed() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600057";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            
                timeout: None}],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600057-root".to_string(),
                status: NodeStatus::Stuck,
                attempts: 2,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "stuck");

    let output = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "recovery", "workflowId": "test-workflow-b" }
                ],
                "edges": [
                    { "source": "root", "target": "recovery", "type": "always" }
                ]
            }
        }),
    )
    .expect("stuck graft succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let nodes = response
        .get("data")
        .and_then(|data| data.get("nodes"))
        .and_then(Value::as_array)
        .expect("nodes array");

    assert_eq!(nodes.len(), 2);
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("recovery".to_string()))
            && node.get("status") == Some(&Value::String("pending".to_string()))
    }));
}
