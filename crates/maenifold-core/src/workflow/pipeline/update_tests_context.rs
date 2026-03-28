use super::executor::predecessor_context_content;
use super::test_support::*;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use crate::markdown::{append_h2_section, read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::util::json::OutputFormat;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;

#[test]
#[serial(config)]
fn predecessor_context_replaces_existing_section_with_all_completed_upstreams() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600800";
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Workflow {
                id: "root-a".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            },
            PipelineNode::Workflow {
                id: "root-b".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: None,
                prompt: None,
            },
            PipelineNode::Workflow {
                id: "child".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: None,
                prompt: None,
            },
        ],
        edges: vec![
            PipelineEdge {
                source: "root-a".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "root-b".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
            },
        ],
    };
    let node_states = HashMap::from([
        (
            "root-a".to_string(),
            NodeState {
                session_id: "workflow-1735689600800-root-a".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "root-b".to_string(),
            NodeState {
                session_id: "workflow-1735689600800-root-b".to_string(),
                status: NodeStatus::Active,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600800-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 8);

    for (session_id, last_value) in [
        (
            "workflow-1735689600800-root-a",
            "Step 1 Response: Alpha [[context]]",
        ),
        (
            "workflow-1735689600800-root-b",
            "Step 1 Response: Beta [[context]]",
        ),
    ] {
        let uri = workflow_session_uri(session_id);
        let (frontmatter, content, _) =
            read_markdown(&uri, &context.config).expect("read upstream");
        let mut frontmatter = frontmatter.expect("upstream frontmatter");
        frontmatter.insert(
            "last".to_string(),
            YamlValue::String(last_value.to_string()),
        );
        write_markdown(&uri, Some(&frontmatter), &content, &context.config)
            .expect("write upstream last");
    }

    let child_uri = workflow_session_uri("workflow-1735689600800-child");
    append_h2_section(
        &child_uri,
        "Predecessor Context",
        "### From root-a\n\nOld [[context]] only",
        &context.config,
    )
    .expect("seed predecessor context");
    append_h2_section(
        &child_uri,
        "Step 1/1",
        "Downstream [[context]] step",
        &context.config,
    )
    .expect("seed downstream step");

    super::handler::pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "nodeId": "root-b",
            "status": "completed",
            "conclusion": "Beta [[context]] conclusion"
        }),
    )
    .expect("complete second upstream");

    let (_, child_content, _) = read_markdown(&child_uri, &context.config).expect("read child");
    assert_eq!(child_content.matches("## Predecessor Context").count(), 1);
    assert!(child_content.contains("### From root-a"));
    assert!(child_content.contains("Step 1 Response: Alpha [[context]]"));
    assert!(child_content.contains("### From root-b"));
    assert!(child_content.contains("Beta [[context]] conclusion"));
    assert!(
        !child_content.contains("Old [[context]] only"),
        "stale content should be gone"
    );
    assert!(child_content.contains("## Step 1/1"));
}

#[test]
#[serial(config)]
fn predecessor_context_prefers_node_output_before_workflow_last() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600840";
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Workflow {
                id: "root-a".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            },
            PipelineNode::Workflow {
                id: "root-b".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: None,
                prompt: None,
            },
            PipelineNode::Workflow {
                id: "child".to_string(),
                workflow_id: "test-workflow-c".to_string(),
                acp_client: None,
                prompt: None,
            },
        ],
        edges: vec![
            PipelineEdge {
                source: "root-a".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "root-b".to_string(),
                target: "child".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
            },
        ],
    };
    let node_states = HashMap::from([
        (
            "root-a".to_string(),
            NodeState {
                session_id: "workflow-1735689600840-root-a".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: Some("Alpha [[context]] conclusion".to_string()),
            },
        ),
        (
            "root-b".to_string(),
            NodeState {
                session_id: "workflow-1735689600840-root-b".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600840-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        definition.clone(),
        node_states.clone(),
        8,
    );

    for (session_id, last_value) in [
        (
            "workflow-1735689600840-root-a",
            "Step 1 Response: Alpha [[context]]",
        ),
        (
            "workflow-1735689600840-root-b",
            "Step 1 Response: Beta [[context]]",
        ),
    ] {
        let uri = workflow_session_uri(session_id);
        let (frontmatter, content, _) =
            read_markdown(&uri, &context.config).expect("read upstream");
        let mut frontmatter = frontmatter.expect("upstream frontmatter");
        frontmatter.insert(
            "last".to_string(),
            YamlValue::String(last_value.to_string()),
        );
        write_markdown(&uri, Some(&frontmatter), &content, &context.config)
            .expect("write upstream last");
    }

    let content = predecessor_context_content(&context, &definition, &node_states, "child")
        .expect("predecessor context lookup")
        .expect("predecessor context content");

    assert!(content.contains("### From root-a"));
    assert!(content.contains("Alpha [[context]] conclusion"));
    assert!(!content.contains("Step 1 Response: Alpha [[context]]"));
    assert!(content.contains("### From root-b"));
    assert!(content.contains("Step 1 Response: Beta [[context]]"));
}
