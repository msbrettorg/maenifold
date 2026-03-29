mod dispatch;
mod dispatch_non_workflow;
mod dispatch_prompt;
#[cfg(test)]
mod dispatch_tests;
mod dispatch_thinking;
mod dispatch_tool;
mod dispatch_workflow;
mod engine;
#[cfg(test)]
mod engine_tests;
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
mod update_tests_sections;
mod validation;
#[cfg(test)]
mod validation_test_support;
#[cfg(test)]
mod validation_tests;
#[cfg(test)]
mod validation_tests_fields;
mod view;
mod view_cognitive;
mod view_mermaid;
mod view_nodes;
#[cfg(test)]
mod view_tests;
mod workflow_id;

pub use engine::{compute_pipeline_status, compute_ready_set};
pub use handler::pipeline;
pub use types::{
    EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode,
    PredecessorOutput,
};
pub use validation::validate_pipeline;
use workflow_id::workflow_id_for_node;
