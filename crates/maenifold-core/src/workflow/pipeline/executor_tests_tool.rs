use std::collections::HashMap;

use serde_json::{Value, json};
use serial_test::serial;
use tokio_util::sync::CancellationToken;

use crate::util::json::OutputFormat;
use crate::workflow::pipeline::{PipelineDefinition, PipelineNode};

use super::executor::{ExecutionResult, execute_pipeline};
use super::executor_tests_support::{
    pending_node_state, pipeline_test_context, read_pipeline_node_states,
    seed_pipeline_session_with_nodes,
};

#[tokio::test]
#[serial(config)]
async fn executor_runs_tool_nodes_and_captures_tool_output() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-executor-tool";

    seed_pipeline_session_with_nodes(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![PipelineNode::Tool {
                id: "tool".to_string(),
                tool_name: "GetConfig".to_string(),
                tool_args: Some(json!({})),
                agent: None,
            }],
            edges: vec![],
        },
        HashMap::from([pending_node_state("tool", "workflow-executor-tool-node")]),
        1,
    );

    let result = execute_pipeline(
        &context,
        pipeline_id,
        "Ambient context should not affect tool execution",
        CancellationToken::new(),
    )
    .await
    .expect("executor should run tool node");

    assert_eq!(
        result,
        ExecutionResult {
            completed: 1,
            abandoned: 0,
            status: "completed".to_string(),
        }
    );
    let output = read_pipeline_node_states(&context, pipeline_id)["tool"]
        .output
        .as_deref()
        .expect("tool node output")
        .to_string();
    let envelope: Value = serde_json::from_str(&output).expect("tool output JSON");
    assert_eq!(envelope["success"].as_bool(), Some(true));
    assert!(
        envelope["data"]
            .as_str()
            .is_some_and(|text| text.contains("Maenifold Configuration:")),
        "unexpected tool output: {output}"
    );
}
