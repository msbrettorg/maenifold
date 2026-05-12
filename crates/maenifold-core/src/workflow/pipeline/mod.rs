use std::{collections::HashSet, sync::Arc, time::Duration};

use tracing::warn;

use crate::{
    acp_config::{AgentsConfig, McpServerEntry},
    error::Result,
    mcp::stdio_pool::{StdioPool, StdioServerConfig},
    tools::ToolContext,
};

mod audit;
pub(super) mod circuit_breaker;
#[cfg(test)]
mod conditional_extraction_tests;
mod conditional_output;
#[cfg(test)]
mod delete_verification_tests;
mod dispatch;
mod dispatch_non_workflow;
mod dispatch_prompt;
mod dispatch_thinking;
mod dispatch_tool;
mod dispatch_workflow;
#[cfg(test)]
mod durable_retry_tests;
mod engine_cascade;
mod engine_ready;
#[cfg(test)]
mod engine_ready_tests;
mod engine_runs;
mod engine_status;
pub(super) mod executor;
mod executor_apply;
mod executor_cleanup;
mod executor_helpers;
mod executor_nodes;
mod executor_nodes_build_helpers;
mod executor_nodes_cwd;
mod executor_nodes_dispatch;
mod executor_nodes_prompt;
mod executor_nodes_thinking;
mod executor_nodes_tool;
mod executor_nodes_workflow;
mod executor_orchestration;
mod executor_retry;
mod executor_retry_persist;
#[cfg(test)]
mod executor_retry_tests;
mod executor_run;
mod executor_timeout;
#[cfg(test)]
mod executor_timeout_tests;
mod external_mcp;
mod external_mcp_transport;
#[cfg(test)]
mod failure_reason_scope_tests;
#[cfg(test)]
mod failure_reason_scope_tests_cascade;
#[cfg(test)]
mod failure_reason_scope_tests_edge;
mod handler;
mod handler_crud;
#[cfg(test)]
mod handler_crud_audit_tests;
mod handler_delete;
#[cfg(test)]
mod handler_delete_degraded_tests;
mod handler_lifecycle;
mod handler_nodes;
mod handler_surgery;
mod handler_validation;
mod handler_wait;
#[cfg(test)]
mod handler_wait_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod integration_tests_cancel;
#[cfg(test)]
mod integration_tests_helpers;
mod lease;
mod mapping_apply;
mod mapping_predecessor;
mod mapping_resolve;
#[cfg(test)]
mod orphan_cascade_tests;
#[cfg(test)]
mod orphan_recovery_tests;
mod resource_notifications;
mod session_delete_files;
mod session_io;
mod session_read;
mod session_write;
mod session_write_files;
pub mod stream;
#[cfg(test)]
mod t1047_scope_tests;
#[cfg(test)]
mod test_context;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod test_fixtures_helpers;
#[cfg(test)]
mod test_support;
mod types;
mod types_node;
mod types_wire;
mod update;
mod update_context;
mod update_restart;
mod update_status;
mod validation;
mod view;
mod view_cognitive;
mod view_mermaid;
mod view_nodes;
mod workflow_id;

pub use engine_cascade::compute_blocked_cascade;
pub use engine_ready::{compute_ready_set, has_remaining_work};
pub use engine_status::{
    compute_pipeline_status, compute_pipeline_status_with_definition, compute_terminal_node_ids,
};
pub use executor::ExecutionResult;
pub use executor_cleanup::{
    cancel_active_nodes, finalize_failed_pipeline_execution, recover_crashed_nodes,
};
pub(crate) use executor_run::persist_active_node_session;
pub use handler::pipeline;
pub use lease::pipeline_lock_path;
pub use session_write::persist_final_pipeline_status;
pub use types::{
    EdgeType, NodeAdopt, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    PredecessorOutput, RetryPolicy, SessionRef, ToolProvenance,
};
pub use validation::validate_pipeline;
use workflow_id::workflow_id_for_node;

struct PoolGuard {
    stdio_pool: Option<Arc<StdioPool>>,
}

impl PoolGuard {
    fn new(stdio_pool: Arc<StdioPool>) -> Self {
        Self {
            stdio_pool: Some(stdio_pool),
        }
    }
}

impl Drop for PoolGuard {
    fn drop(&mut self) {
        let Some(stdio_pool) = self.stdio_pool.take() else {
            return;
        };
        let _ = std::thread::Builder::new()
            .name("pipeline-stdio-pool-shutdown".to_string())
            .spawn(move || match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => {
                    runtime.block_on(stdio_pool.shutdown());
                }
                Err(error) => {
                    warn!(error = %error, "failed to build runtime for pipeline stdio pool shutdown");
                }
            });
    }
}

pub async fn execute_pipeline(
    context: &ToolContext,
    session_id: &str,
    prompt: &str,
    cancel: tokio_util::sync::CancellationToken,
    stdio_pool: Option<Arc<StdioPool>>,
) -> Result<ExecutionResult> {
    let (stdio_pool, _guard) = match stdio_pool {
        Some(stdio_pool) => (Some(stdio_pool), None),
        None => match default_stdio_pool().await {
            Some(stdio_pool) => (Some(stdio_pool.clone()), Some(PoolGuard::new(stdio_pool))),
            None => (None, None),
        },
    };
    let execution_context =
        ToolContext::new(context.config.clone(), context.output_format).with_stdio_pool(stdio_pool);
    executor::execute_pipeline(&execution_context, session_id, prompt, cancel).await
}

async fn default_stdio_pool() -> Option<Arc<StdioPool>> {
    let root = crate::config::Config::current().maenifold_root;
    let config = AgentsConfig::load(&root).ok()?;
    let servers = collect_stdio_servers(&config);
    if servers.is_empty() {
        return None;
    }

    match StdioPool::start(servers).await {
        Ok(pool) => Some(Arc::new(pool)),
        Err(error) => {
            warn!(error = %error, root = %root.display(), "failed to start pipeline stdio MCP pool");
            None
        }
    }
}

fn collect_stdio_servers(config: &AgentsConfig) -> Vec<StdioServerConfig> {
    let mut servers = Vec::new();
    let mut seen = HashSet::new();
    collect_stdio_entries(&mut servers, &mut seen, &config.mcp_servers);
    for agent in config.agents.values() {
        collect_stdio_entries(&mut servers, &mut seen, &agent.mcp_servers);
    }
    servers
}

fn collect_stdio_entries(
    servers: &mut Vec<StdioServerConfig>,
    seen: &mut HashSet<String>,
    entries: &[McpServerEntry],
) {
    for entry in entries {
        let McpServerEntry::Stdio {
            name,
            command,
            args,
            timeout,
            env,
            ..
        } = entry
        else {
            continue;
        };
        if !seen.insert(name.clone()) {
            continue;
        }
        servers.push(StdioServerConfig {
            name: name.clone(),
            command: command.clone(),
            args: args.clone(),
            env: env.clone(),
            timeout: timeout.map(Duration::from_secs),
        });
    }
}
