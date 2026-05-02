use jockey::EventSender;

use crate::tools::ToolContext;

use super::super::runner::{self, AgentSessionConfig};
use super::executor_nodes_dispatch::missing_agent_configuration;
use super::executor_run::NodeTaskResult;

#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_thinking_node_request(
    context: &ToolContext,
    node_id: &str,
    pipeline_session_id: &str,
    node_session_id: String,
    topic: String,
    prompt_context: String,
    total_thoughts: u32,
    adopt_instructions: Vec<String>,
    adopt_role: Option<String>,
    requires_role: Option<String>,
    session_config: AgentSessionConfig,
    agent: Option<&str>,
    event_tx: EventSender,
) -> NodeTaskResult {
    match agent {
        Some(agent) => NodeTaskResult::SequentialThinking(
            runner::run_think_with_pool(
                context,
                agent,
                &topic,
                &prompt_context,
                runner::ThinkOptions {
                    total_thoughts,
                    parent_workflow_id: Some(node_session_id),
                    model: session_config.model.clone(),
                },
                &adopt_instructions,
                adopt_role.as_deref(),
                requires_role.as_deref(),
                session_config,
                event_tx,
                None,
                true,
                Some(runner::ActiveNodeSessionPersistence {
                    pipeline_session_id: pipeline_session_id.to_string(),
                    node_id: node_id.to_string(),
                    thinking_session_id: None,
                }),
            )
            .await,
        ),
        None => NodeTaskResult::SequentialThinking(Err(missing_agent_configuration(
            node_id,
            "sequentialThinking",
        ))),
    }
}
