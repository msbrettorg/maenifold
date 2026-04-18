pub(super) mod circuit_breaker;
mod dispatch;
mod dispatch_non_workflow;
mod dispatch_prompt;
mod dispatch_thinking;
mod dispatch_tool;
mod dispatch_workflow;
mod engine_cascade;
mod engine_ready;
#[cfg(test)]
mod engine_ready_tests;
mod engine_status;
pub(super) mod executor;
mod executor_cleanup;
mod executor_helpers;
mod executor_nodes;
mod executor_nodes_cwd;
mod executor_nodes_dispatch;
mod executor_nodes_prompt;
mod executor_nodes_thinking;
mod executor_nodes_tool;
mod executor_nodes_workflow;
mod executor_orchestration;
mod executor_retry;
#[cfg(test)]
mod executor_retry_tests;
mod executor_run;
mod executor_timeout;
#[cfg(test)]
mod executor_timeout_tests;
#[cfg(test)]
mod integration_tests;
mod external_mcp;
#[cfg(test)]
mod failure_reason_scope_tests;
mod handler;
mod handler_crud;
mod handler_lifecycle;
mod handler_nodes;
mod handler_surgery;
mod handler_validation;
mod lease;
mod mapping_apply;
mod mapping_predecessor;
mod mapping_resolve;
#[cfg(test)]
mod orphan_cascade_tests;
mod session_io;
mod session_read;
mod session_write;
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
pub use executor::{ExecutionResult, execute_pipeline};
pub use executor_cleanup::{
    cancel_active_nodes, finalize_failed_pipeline_execution, recover_crashed_nodes,
};
pub use handler::pipeline;
pub use lease::pipeline_lock_path;
pub use session_write::persist_final_pipeline_status;
pub use types::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    PredecessorOutput, RetryPolicy, ToolProvenance,
};
pub use validation::validate_pipeline;
use workflow_id::workflow_id_for_node;
