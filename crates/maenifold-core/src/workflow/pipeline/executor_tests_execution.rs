use std::collections::HashMap;

use serde_json::json;
use serde_yaml::Value as YamlValue;
use serial_test::serial;
use tokio_util::sync::CancellationToken;

use crate::markdown::read_markdown;
use crate::thinking::sequential_session_uri;
use crate::util::json::OutputFormat;
use crate::workflow::pipeline::{
    EdgeType, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
};

use super::executor::{ExecutionResult, execute_pipeline};
use super::executor_tests_stub::{install_acpx_stub, read_stub_prompts, write_stub_config};
use super::executor_tests_support::{
    pending_node_state, pipeline_test_context, read_pipeline_node_states, read_workflow_conclusion,
    read_workflow_status, seed_pipeline_session_with_nodes, seed_pipeline_session_with_workflows,
    write_workflow_definition,
};

fn thinking_xml(
    response: &str,
    done: bool,
    needs_more: bool,
    revision_of: Option<u32>,
    conclusion: Option<&str>,
) -> String {
    format!(
        "<output><response>{response}</response><done>{done}</done><needs_more>{needs_more}</needs_more><revision_of>{}</revision_of><conclusion>{}</conclusion></output>",
        revision_of
            .map(|value| value.to_string())
            .unwrap_or_default(),
        conclusion.unwrap_or_default()
    )
}

#[tokio::test]
#[serial(config)]
async fn executor_completes_single_node_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);
    let pipeline_id = "pipeline-executor-single";
    let workflow_session_id = "workflow-executor-single-root";

    write_workflow_definition(&context.config.assets_path, "test-workflow-a");
    write_stub_config(
        &context.config.maenifold_root,
        json!({"test-agent-root": { "text": "Single node completed [[step]]" }}),
    );
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("test-agent-root".to_string()),
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([pending_node_state("root", workflow_session_id)]),
        8,
    );

    let result = execute_pipeline(
        &context,
        pipeline_id,
        "Execute the workflow",
        CancellationToken::new(),
    )
    .await
    .expect("executor should succeed");

    assert_eq!(
        result,
        ExecutionResult {
            completed: 1,
            abandoned: 0,
            status: "completed".to_string(),
        }
    );
    let node_states = read_pipeline_node_states(&context, pipeline_id);
    assert_eq!(node_states["root"].status, NodeStatus::Completed);
    assert_eq!(
        node_states["root"].output.as_deref(),
        Some("Single node completed [[step]]")
    );
    assert_eq!(
        read_workflow_status(&context, workflow_session_id),
        Some("completed".to_string())
    );
    assert_eq!(
        read_workflow_conclusion(&context, workflow_session_id),
        Some("Single node completed [[step]]".to_string())
    );
    let prompts = read_stub_prompts(&context.config.maenifold_root, "test-agent-root");
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("Execute the workflow"));
    assert!(prompts[0].contains("Step 1/1"));
}

#[tokio::test]
#[serial(config)]
async fn executor_assembles_workflow_prompt_with_predecessor_context() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);
    let pipeline_id = "pipeline-executor-workflow-prompt";

    write_workflow_definition(&context.config.assets_path, "test-workflow-a");
    write_workflow_definition(&context.config.assets_path, "test-workflow-b");
    write_stub_config(
        &context.config.maenifold_root,
        json!({
            "test-agent-a": { "text": "Upstream conclusion [[step]]" },
            "test-agent-b": { "text": "Downstream conclusion [[step]]" }
        }),
    );
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("test-agent-a".to_string()),
                    prompt: None,
                },
                PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: Some("test-agent-b".to_string()),
                    prompt: Some("Node-specific instruction".to_string()),
                },
            ],
            edges: vec![PipelineEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            }],
        },
        HashMap::from([
            pending_node_state("a", "workflow-executor-workflow-prompt-a"),
            pending_node_state("b", "workflow-executor-workflow-prompt-b"),
        ]),
        2,
    );

    let result = execute_pipeline(
        &context,
        pipeline_id,
        "Ambient context",
        CancellationToken::new(),
    )
    .await
    .expect("executor should succeed");

    assert_eq!(result.completed, 2);
    assert_eq!(result.abandoned, 0);
    assert_eq!(result.status, "completed");
    let prompts = read_stub_prompts(&context.config.maenifold_root, "test-agent-b");
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("### From a\n\nUpstream conclusion [[step]]"));
    assert!(prompts[0].contains("Ambient context\n\nNode-specific instruction"));
    assert!(prompts[0].contains("Step 1/1"));
}

#[tokio::test]
#[serial(config)]
async fn executor_runs_prompt_and_thinking_nodes_and_captures_output() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);
    let pipeline_id = "pipeline-executor-mixed";

    write_workflow_definition(&context.config.assets_path, "test-workflow-a");
    write_stub_config(
        &context.config.maenifold_root,
        json!({
            "test-agent-a": { "text": "Workflow output [[step]]" },
            "prompt-agent": { "text": "Prompt output [[note]]" },
            "think-agent": {
                "text": thinking_xml(
                    "DONE Thinking output [[idea]]",
                    true,
                    false,
                    None,
                    Some("DONE Thinking output [[idea]]")
                )
            }
        }),
    );
    seed_pipeline_session_with_nodes(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("test-agent-a".to_string()),
                    prompt: None,
                },
                PipelineNode::Prompt {
                    id: "prompt".to_string(),
                    prompt: "Draft next action".to_string(),
                    agent: "prompt-agent".to_string(),
                    tool_mode: "default".to_string(),
                },
                PipelineNode::SequentialThinking {
                    id: "think".to_string(),
                    topic: "Plan follow-up".to_string(),
                    total_thoughts: 2,
                    analysis_type: Some("architecture".to_string()),
                    agent: "think-agent".to_string(),
                },
            ],
            edges: vec![
                PipelineEdge {
                    source: "a".to_string(),
                    target: "prompt".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
                PipelineEdge {
                    source: "prompt".to_string(),
                    target: "think".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
            ],
        },
        HashMap::from([
            pending_node_state("a", "workflow-executor-mixed-a"),
            pending_node_state("prompt", "workflow-executor-mixed-prompt"),
            pending_node_state("think", "workflow-executor-mixed-think"),
        ]),
        3,
    );

    let result = execute_pipeline(
        &context,
        pipeline_id,
        "Ambient context",
        CancellationToken::new(),
    )
    .await
    .expect("executor should succeed");

    assert_eq!(
        result,
        ExecutionResult {
            completed: 3,
            abandoned: 0,
            status: "completed".to_string(),
        }
    );
    let node_states = read_pipeline_node_states(&context, pipeline_id);
    assert_eq!(
        node_states["a"].output.as_deref(),
        Some("Workflow output [[step]]")
    );
    assert_eq!(
        node_states["prompt"].output.as_deref(),
        Some("Prompt output [[note]]")
    );
    assert_eq!(
        node_states["think"].output.as_deref(),
        Some("DONE Thinking output [[idea]]")
    );
    let thinking_session_id = node_states["think"]
        .thinking_session_id
        .as_deref()
        .expect("thinking session id should be persisted");
    assert!(thinking_session_id.starts_with("session-"));

    let (thinking_frontmatter, _, _) = read_markdown(
        &sequential_session_uri(thinking_session_id),
        &context.config,
    )
    .expect("read thinking session");
    let thinking_frontmatter = thinking_frontmatter.expect("thinking frontmatter");
    assert_eq!(
        thinking_frontmatter.get("parent"),
        Some(&YamlValue::String(
            "[[workflow/workflow-executor-mixed-think]]".to_string()
        ))
    );
    let prompt_prompts = read_stub_prompts(&context.config.maenifold_root, "prompt-agent");
    assert_eq!(
        prompt_prompts[0],
        "### From a\n\nWorkflow output [[step]]\n\nAmbient context\n\nDraft next action"
    );
    let think_prompts = read_stub_prompts(&context.config.maenifold_root, "think-agent");
    assert_eq!(think_prompts.len(), 1);
    assert!(think_prompts[0].starts_with(
        "Thought 1/~2 on: ### From prompt\n\nPrompt output [[note]]\n\nAmbient context\n\nPlan follow-up."
    ));
    assert!(think_prompts[0].contains("Include [[WikiLinks]]."));
    assert!(think_prompts[0].contains("Wrap your response in <output> tags:"));
    assert!(think_prompts[0].contains("Only the <output> block is parsed."));
}
