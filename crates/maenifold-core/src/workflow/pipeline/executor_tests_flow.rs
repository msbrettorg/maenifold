use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use serial_test::serial;
use tokio_util::sync::CancellationToken;

use crate::util::json::OutputFormat;
use crate::workflow::pipeline::{EdgeType, NodeStatus, PipelineDefinition, PipelineEdge};

use super::executor::execute_pipeline;
use super::executor_tests_stub::{
    install_acpx_stub, read_stub_event_order, read_stub_max, wait_for_stub_max, write_stub_config,
};
use super::executor_tests_support::{
    pending_node_state, pipeline_node, pipeline_test_context, read_pipeline_node_states,
    seed_pipeline_session_with_workflows, write_workflow_definition,
};

#[tokio::test]
#[serial(config)]
async fn executor_handles_node_failure() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);
    let pipeline_id = "pipeline-executor-failure";

    write_workflow_definition(&context.config.assets_path, "test-workflow-a");
    write_stub_config(
        &context.config.maenifold_root,
        json!({"failing-agent": { "error": "simulated failure" }}),
    );
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("failing-agent".to_string()),
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([pending_node_state("root", "workflow-executor-failure-root")]),
        8,
    );

    let result = execute_pipeline(
        &context,
        pipeline_id,
        "Execute the workflow",
        CancellationToken::new(),
    )
    .await
    .expect("executor should handle failures");

    assert_eq!(result.completed, 0);
    assert_eq!(result.abandoned, 1);
    assert_eq!(result.status, "abandoned");
    assert_eq!(
        read_pipeline_node_states(&context, pipeline_id)["root"].status,
        NodeStatus::Abandoned
    );
}

#[tokio::test]
#[serial(config)]
async fn executor_respects_cancellation() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);
    let pipeline_id = "pipeline-executor-cancel";

    write_workflow_definition(&context.config.assets_path, "test-workflow-a");
    write_workflow_definition(&context.config.assets_path, "test-workflow-b");
    write_stub_config(
        &context.config.maenifold_root,
        json!({
            "test-agent-a": { "delay": 5.0, "text": "A [[step]]" },
            "test-agent-b": { "delay": 5.0, "text": "B [[step]]" }
        }),
    );
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                pipeline_node("a", "test-workflow-a"),
                pipeline_node("b", "test-workflow-b"),
            ],
            edges: vec![],
        },
        HashMap::from([
            pending_node_state("a", "workflow-executor-cancel-a"),
            pending_node_state("b", "workflow-executor-cancel-b"),
        ]),
        2,
    );

    wait_for_stub_max(&context.config.maenifold_root, 0).await;
    let context = Arc::new(context);
    let cancel = CancellationToken::new();
    let handle = tokio::spawn({
        let task_context = context.clone();
        let cancel_for_task = cancel.clone();
        async move {
            execute_pipeline(
                &task_context,
                pipeline_id,
                "Execute the workflow",
                cancel_for_task,
            )
            .await
        }
    });

    wait_for_stub_max(&context.config.maenifold_root, 2).await;
    cancel.cancel();
    let result = handle
        .await
        .expect("join executor task")
        .expect("executor should exit cleanly");

    assert_eq!(result.completed, 0);
    assert_eq!(result.abandoned, 2);
    assert_eq!(result.status, "cancelled");
    let node_states = read_pipeline_node_states(&context, pipeline_id);
    assert!(matches!(node_states["a"].status, NodeStatus::Abandoned));
    assert!(matches!(node_states["b"].status, NodeStatus::Abandoned));
}

#[tokio::test]
#[serial(config)]
async fn executor_respects_max_concurrency() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);
    let pipeline_id = "pipeline-executor-concurrency";

    write_workflow_definition(&context.config.assets_path, "test-workflow-a");
    write_workflow_definition(&context.config.assets_path, "test-workflow-b");
    write_stub_config(
        &context.config.maenifold_root,
        json!({
            "test-agent-a": { "delay": 0.2, "text": "A [[step]]" },
            "test-agent-b": { "delay": 0.2, "text": "B [[step]]" },
            "test-agent-c": { "delay": 0.2, "text": "C [[step]]" },
            "test-agent-d": { "delay": 0.2, "text": "D [[step]]" }
        }),
    );
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                pipeline_node("a", "test-workflow-a"),
                pipeline_node("b", "test-workflow-b"),
                pipeline_node("c", "test-workflow-a"),
                pipeline_node("d", "test-workflow-b"),
            ],
            edges: vec![],
        },
        HashMap::from([
            pending_node_state("a", "workflow-executor-concurrency-a"),
            pending_node_state("b", "workflow-executor-concurrency-b"),
            pending_node_state("c", "workflow-executor-concurrency-c"),
            pending_node_state("d", "workflow-executor-concurrency-d"),
        ]),
        2,
    );

    let result = execute_pipeline(
        &context,
        pipeline_id,
        "Execute the workflow",
        CancellationToken::new(),
    )
    .await
    .expect("executor should succeed");

    assert_eq!(result.completed, 4);
    assert_eq!(result.abandoned, 0);
    assert_eq!(result.status, "completed");
    assert_eq!(read_stub_max(&context.config.maenifold_root), 2);
}

#[tokio::test]
#[serial(config)]
async fn executor_cascades_through_dag() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);
    let pipeline_id = "pipeline-executor-dag";

    write_workflow_definition(&context.config.assets_path, "test-workflow-a");
    write_workflow_definition(&context.config.assets_path, "test-workflow-b");
    write_stub_config(
        &context.config.maenifold_root,
        json!({
            "test-agent-a": { "text": "A complete [[step]]" },
            "test-agent-b": { "text": "B complete [[step]]" },
            "test-agent-c": { "text": "C complete [[step]]" }
        }),
    );
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                pipeline_node("a", "test-workflow-a"),
                pipeline_node("b", "test-workflow-b"),
                pipeline_node("c", "test-workflow-a"),
            ],
            edges: vec![
                PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
                PipelineEdge {
                    source: "b".to_string(),
                    target: "c".to_string(),
                    edge_type: EdgeType::Success,
                    mapping: None,
                },
            ],
        },
        HashMap::from([
            pending_node_state("a", "workflow-executor-dag-a"),
            pending_node_state("b", "workflow-executor-dag-b"),
            pending_node_state("c", "workflow-executor-dag-c"),
        ]),
        3,
    );

    let result = execute_pipeline(
        &context,
        pipeline_id,
        "Execute the workflow",
        CancellationToken::new(),
    )
    .await
    .expect("executor should succeed");

    assert_eq!(result.completed, 3);
    assert_eq!(result.abandoned, 0);
    assert_eq!(result.status, "completed");
    assert_eq!(
        read_stub_event_order(&context.config.maenifold_root),
        vec![
            "test-agent-a".to_string(),
            "test-agent-b".to_string(),
            "test-agent-c".to_string(),
        ]
    );
    assert!(
        read_pipeline_node_states(&context, pipeline_id)
            .values()
            .all(|state| matches!(state.status, NodeStatus::Completed))
    );
}
