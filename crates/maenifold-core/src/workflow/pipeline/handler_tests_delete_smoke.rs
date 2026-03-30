use super::handler::pipeline;
use super::test_support::*;
use super::{NodeState, NodeStatus};
use crate::markdown::file_exists;
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use serde_json::json;
use serial_test::serial;
use std::collections::HashMap;

fn node_state(id: &str, status: NodeStatus, attempts: u32) -> NodeState {
    NodeState {
        session_id: id.to_string(),
        status,
        attempts,
        thinking_session_id: None,
        output: None,
    }
}

#[test]
#[serial(config)]
fn del_06_delete_pending_pipeline_rejected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689602204";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            node_state("workflow-1735689600204-root", NodeStatus::Completed, 1),
        ),
        (
            "child".to_string(),
            node_state("workflow-1735689600204-child", NodeStatus::Pending, 0),
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 2);

    let error = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "delete": true }),
    )
    .expect_err("pending pipeline delete should fail");

    assert!(
        error
            .to_string()
            .contains("pause or cancel it before deleting"),
        "unexpected error: {error}"
    );
    assert!(
        file_exists(&pipeline_session_uri(pipeline_id), &context.config)
            .expect("pipeline still exists")
    );
}
