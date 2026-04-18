use std::path::PathBuf;

use serde_json::Value as JsonValue;

use crate::tools::ToolContext;

use super::executor_run::{NodeTaskResult, run_tool_node};

pub(super) async fn execute_tool_node_request(
    context: &ToolContext,
    tool_name: String,
    tool_args: Option<JsonValue>,
    agent: Option<String>,
    allowed_tools: Option<Vec<String>>,
    cwd: PathBuf,
) -> NodeTaskResult {
    NodeTaskResult::Tool(
        run_tool_node(
            context,
            &tool_name,
            tool_args.as_ref(),
            agent.as_deref(),
            allowed_tools.as_deref(),
            &cwd,
        )
        .await,
    )
}
