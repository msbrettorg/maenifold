mod dispatch;
mod dispatch_non_workflow;
mod dispatch_prompt;
#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod dispatch_tests_concurrency;
mod dispatch_thinking;
mod dispatch_tool;
mod dispatch_workflow;
#[cfg(test)]
mod e2e_tests_lifecycle;
#[cfg(test)]
mod e2e_tests_surgery_live;
#[cfg(test)]
mod e2e_tests_tool_chain;
mod engine;
#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod engine_tests_edges_smoke;
pub(super) mod executor;
mod executor_helpers;
mod executor_mapping;
mod executor_nodes;
mod executor_run;
#[cfg(test)]
mod executor_tests;
#[cfg(test)]
mod executor_tests_support;
mod handler;
mod handler_crud;
mod handler_lifecycle;
mod handler_nodes;
mod handler_surgery;
#[cfg(test)]
mod handler_tests;
#[cfg(test)]
mod handler_tests_create_smoke;
#[cfg(test)]
mod handler_tests_create_smoke_b;
#[cfg(test)]
mod handler_tests_create_smoke_nodeid;
#[cfg(test)]
mod handler_tests_create_validation;
#[cfg(test)]
mod handler_tests_delete_smoke;
#[cfg(test)]
mod handler_tests_e2e_smoke;
#[cfg(test)]
mod handler_tests_graft_smoke;
#[cfg(test)]
mod handler_tests_lifecycle_smoke;
#[cfg(test)]
mod handler_tests_param_smoke;
#[cfg(test)]
mod handler_tests_param_smoke_b;
#[cfg(test)]
mod handler_tests_param_smoke_c;
#[cfg(test)]
mod handler_tests_rewind_smoke;
#[cfg(test)]
mod handler_tests_state_matrix;
#[cfg(test)]
mod handler_tests_surgery_smoke;
mod handler_validation;
mod session_io;
#[cfg(test)]
mod session_io_tests;
mod session_read;
mod session_write;
mod supervisor;
mod supervisor_detection;
mod supervisor_lifecycle;
#[cfg(test)]
mod supervisor_test_support;
#[cfg(test)]
mod supervisor_tests;
#[cfg(test)]
mod test_context;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod test_fixtures_helpers;
#[cfg(test)]
mod test_support;
mod types;
#[cfg(test)]
mod types_tests;
mod types_wire;
mod update;
mod update_context;
mod update_restart;
mod update_status;
#[cfg(test)]
mod update_tests;
#[cfg(test)]
mod update_tests_context;
#[cfg(test)]
mod update_tests_restart;
#[cfg(test)]
mod update_tests_restart_cleanup;
#[cfg(test)]
mod update_tests_sections;
#[cfg(test)]
mod update_tests_status_smoke;
mod validation;
#[cfg(test)]
mod validation_test_support;
#[cfg(test)]
mod validation_tests;
#[cfg(test)]
mod validation_tests_fields;
#[cfg(test)]
mod validation_tests_fields_smoke;
#[cfg(test)]
mod validation_tests_fields_smoke_b;
#[cfg(test)]
mod validation_tests_fields_smoke_c;
#[cfg(test)]
mod validation_tests_fields_smoke_d;
mod view;
mod view_cognitive;
mod view_mermaid;
mod view_nodes;
#[cfg(test)]
mod view_tests;
#[cfg(test)]
mod view_tests_smoke_extra;
mod workflow_id;

pub use engine::{compute_pipeline_status, compute_ready_set, has_remaining_work};
pub use handler::pipeline;
pub(crate) use supervisor::SUPERVISOR_RUNTIME;
pub use types::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    PredecessorOutput,
};
pub use validation::validate_pipeline;
use workflow_id::workflow_id_for_node;
