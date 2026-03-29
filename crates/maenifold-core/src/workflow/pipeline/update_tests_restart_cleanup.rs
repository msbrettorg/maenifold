use super::session_io::frontmatter_value;
use super::test_support::*;
use super::update::restart_pipeline_node;
use super::{NodeState, NodeStatus};
use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use serde_json::Value;
use serial_test::serial;
use std::collections::HashMap;

#[test]
#[serial(config)]
fn pipeline_restart_clears_thinking_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689601050";
    let original_child_session = "workflow-1735689601050-child";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689601050-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: original_child_session.to_string(),
                status: NodeStatus::Abandoned,
                attempts: 1,
                thinking_session_id: Some("thinking-stale-xyz".to_string()),
                output: Some("stale output".to_string()),
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 8);

    let output = restart_pipeline_node(&context, pipeline_id, "child").expect("restart succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let _ = response;

    let (pipeline_frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");
    let updated_states: HashMap<String, NodeState> =
        frontmatter_value(&pipeline_frontmatter, "nodeStates").expect("node states");

    assert!(
        updated_states["child"].thinking_session_id.is_none(),
        "thinking_session_id must be cleared on restart"
    );
    assert!(
        updated_states["child"].output.is_none(),
        "output must be cleared on restart"
    );
    assert_eq!(updated_states["child"].status, NodeStatus::Pending);
    assert_eq!(updated_states["child"].attempts, 2);
}
