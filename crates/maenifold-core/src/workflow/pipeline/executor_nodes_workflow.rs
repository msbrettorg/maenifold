use jockey::EventSender;

use crate::tools::ToolContext;

use super::super::runner::{self, AgentSessionConfig};
use super::executor_nodes_dispatch::missing_agent_configuration;
use super::executor_run::NodeTaskResult;

#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_workflow_node_request(
    context: &ToolContext,
    node_id: &str,
    workflow_id: String,
    existing_session_id: Option<String>,
    prompt: String,
    session_config: AgentSessionConfig,
    agent: Option<&str>,
    event_tx: EventSender,
) -> NodeTaskResult {
    match agent {
        Some(agent) => NodeTaskResult::Workflow(
            runner::run_workflow_with_pool_session_opts(
                context,
                &workflow_id,
                existing_session_id,
                agent,
                &prompt,
                session_config,
                event_tx,
                true,
            )
            .await,
        ),
        None => NodeTaskResult::Workflow(Err(missing_agent_configuration(node_id, "workflow"))),
    }
}
