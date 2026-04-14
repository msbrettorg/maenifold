use std::collections::HashMap;

use serial_test::serial;

use crate::util::json::OutputFormat;

use super::super::executor_nodes::{FailurePhase, NodeOutcome};
use super::super::session_read::{frontmatter_value, read_pipeline_session};
use super::super::test_support::{pipeline_test_context, seed_pipeline_session_with_workflows};
use super::super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

fn read_raw_node_states(
    context: &crate::tools::ToolContext,
    session_id: &str,
) -> HashMap<String, NodeState> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id).expect("read pipeline");
    frontmatter_value(&frontmatter, "nodeStates").expect("node states")
}

#[test]
#[serial]
fn apply_node_outcome_forwards_abandoned_failure_reason() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000-apply-node-outcome-failure-reason";
    let definition = PipelineDefinition {
        nodes: vec![PipelineNode::Workflow {
            id: "node".to_string(),
            display_label: None,
            workflow_id: "test-workflow-a".to_string(),
            acp_client: Some("agent-node".to_string()),
            prompt: None,
            cwd: None,
            timeout: None,
            agent_config: None,
        }],
        edges: vec![],
        inputs: None,
        template_id: None,
    };
    let node_states = HashMap::from([(
        "node".to_string(),
        NodeState {
            session_id: "workflow-1735689600004-node".to_string(),
            status: NodeStatus::Active,
            attempts: 1,
            acp_session_id: None,
            thinking_session_id: None,
            tool_provenance: None,
            output: None,
            failure_reason: None,
            retry_after: None,
            retry_count: None,
        },
    )]);
    seed_pipeline_session_with_workflows(&context, session_id, definition, node_states, 2);

    let mut counts = super::ExecutionCounts::default();
    super::apply_node_outcome(
        &context,
        session_id,
        "node",
        NodeOutcome::Abandoned {
            reason: "runner crashed".to_string(),
            phase: FailurePhase::PostPrompt,
            acp_session_id: None,
            thinking_session_id: None,
        },
        &mut counts,
    )
    .expect("apply abandoned outcome");

    let node_states = read_raw_node_states(&context, session_id);
    assert_eq!(node_states["node"].status, NodeStatus::Abandoned);
    assert_eq!(
        node_states["node"].failure_reason.as_deref(),
        Some("runner crashed")
    );
    assert_eq!(counts.abandoned, 1);
}
