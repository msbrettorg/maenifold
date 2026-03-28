use std::thread;
use std::time::{Duration, Instant};

use super::supervisor::supervisor_clear_all;
use super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};

pub(super) struct SupervisorReset;

impl Drop for SupervisorReset {
    fn drop(&mut self) {
        supervisor_clear_all();
    }
}

pub(super) fn reset_supervisor() -> SupervisorReset {
    supervisor_clear_all();
    SupervisorReset
}

pub(super) fn wait_for(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("condition was not met before timeout");
}

pub(super) fn pending_node_state(id: &str, session_id: &str) -> (String, NodeState) {
    (
        id.to_string(),
        NodeState {
            session_id: session_id.to_string(),
            status: NodeStatus::Pending,
            attempts: 0,
            thinking_session_id: None,
            output: None,
        },
    )
}

pub(super) fn single_workflow_definition(
    workflow_id: &str,
    acp_client: Option<&str>,
) -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![PipelineNode::Workflow {
            id: "root".to_string(),
            workflow_id: workflow_id.to_string(),
            acp_client: acp_client.map(ToString::to_string),
            prompt: None,
        }],
        edges: vec![],
    }
}
