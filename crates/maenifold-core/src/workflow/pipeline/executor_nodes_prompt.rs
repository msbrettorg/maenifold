use jockey::EventSender;

use crate::tools::ToolContext;

use super::super::runner::AgentSessionConfig;
use super::executor_nodes_dispatch::missing_agent_configuration;
use super::executor_run::{NodeTaskResult, run_prompt_node};

#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_prompt_node_request(
    context: &ToolContext,
    node_id: &str,
    session_id: String,
    prompt: String,
    tool_mode: String,
    session_config: AgentSessionConfig,
    agent: Option<&str>,
    event_tx: EventSender,
) -> NodeTaskResult {
    match agent {
        Some(agent) => NodeTaskResult::Prompt(
            run_prompt_node(
                context,
                &event_tx,
                agent,
                &session_id,
                &prompt,
                &tool_mode,
                session_config,
                true,
            )
            .await,
        ),
        None => NodeTaskResult::Prompt(Err(missing_agent_configuration(node_id, "prompt"))),
    }
}
