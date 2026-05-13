use jockey::EventSender;

use crate::tools::ToolContext;

use super::executor_nodes::NodeExecutionRequest;
use super::executor_run::NodeTaskResult;

pub(super) async fn execute_node_request_with_pipeline_session(
    context: ToolContext,
    pipeline_session_id: String,
    node_id: String,
    execution: NodeExecutionRequest,
    agent: Option<String>,
    event_tx: EventSender,
) -> (String, NodeTaskResult) {
    let result = match execution {
        NodeExecutionRequest::Workflow {
            workflow_id,
            existing_session_id,
            prompt,
            adopt_instructions,
            adopt_role,
            requires_role,
            session_config,
        } => {
            super::executor_nodes_workflow::execute_workflow_node_request(
                &context,
                &node_id,
                &pipeline_session_id,
                workflow_id,
                existing_session_id,
                prompt,
                adopt_instructions,
                adopt_role,
                requires_role,
                session_config,
                agent.as_deref(),
                event_tx.clone(),
            )
            .await
        }
        NodeExecutionRequest::Prompt {
            session_id,
            prompt,
            tool_mode,
            adopt_instructions,
            adopt_role,
            requires_role,
            session_config,
        } => {
            super::executor_nodes_prompt::execute_prompt_node_request(
                &context,
                &node_id,
                &pipeline_session_id,
                session_id,
                prompt,
                tool_mode,
                adopt_instructions,
                adopt_role,
                requires_role,
                session_config,
                agent.as_deref(),
                event_tx.clone(),
            )
            .await
        }
        NodeExecutionRequest::SequentialThinking {
            node_session_id,
            topic,
            prompt_context,
            total_thoughts,
            adopt_instructions,
            adopt_role,
            requires_role,
            session_config,
        } => {
            super::executor_nodes_thinking::execute_thinking_node_request(
                &context,
                &node_id,
                &pipeline_session_id,
                node_session_id,
                topic,
                prompt_context,
                total_thoughts,
                adopt_instructions,
                adopt_role,
                requires_role,
                session_config,
                agent.as_deref(),
                event_tx,
            )
            .await
        }
        NodeExecutionRequest::Tool {
            tool_name,
            tool_args,
            agent,
            allowed_tools,
            cwd,
            ..
        } => {
            super::executor_nodes_tool::execute_tool_node_request(
                &context,
                tool_name,
                tool_args,
                agent,
                allowed_tools,
                cwd,
            )
            .await
        }
    };

    (node_id, result)
}

pub(super) fn missing_agent_configuration(
    node_id: &str,
    node_type: &str,
) -> crate::error::MaenifoldError {
    crate::error::MaenifoldError::Message(format!(
        "ERROR: Pipeline node '{node_id}' of type '{node_type}' is missing required agent configuration"
    ))
}
