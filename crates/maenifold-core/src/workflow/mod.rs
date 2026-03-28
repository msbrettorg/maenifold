mod handler;
mod handler_helpers;
mod handler_session;
#[cfg(test)]
mod handler_session_tests;
pub mod pipeline;
mod queue;
pub mod runner;
#[cfg(test)]
mod runner_format_tests;
pub(crate) mod runner_helpers;
#[cfg(test)]
pub(crate) mod runner_test_support;
#[cfg(test)]
mod runner_tests;
mod status;
mod submachine;
pub(crate) mod think_runner;
#[cfg(test)]
mod think_runner_tests;

// T-12.0: RTM NFR-1 consolidates the workflow facade, helpers, and session logic under one module.
pub(crate) use handler::workflow;
pub(crate) use handler_helpers::{
    build_workflow_response_content, format_workflow_step, parse_workflow_identifiers,
    read_workflow_definition, set_current_step, workflow_steps,
};
pub(crate) use queue::{
    append_workflow_queue, frontmatter_usize, initial_workflow_frontmatter, read_workflow_queue,
    start_workflow_session, view_workflow_session,
};
pub use runner::{WorkflowResult, WorkflowRunStatus, run_workflow};
#[allow(unused_imports)]
pub(crate) use status::{
    handle_cancelled_workflow_status, handle_completed_workflow_status, handle_queue_exhaustion,
    transition_to_next_workflow,
};
#[allow(unused_imports)]
pub(crate) use submachine::{
    ResumeWorkflowResult, maybe_resume_workflow_submachine, register_workflow_submachine,
};
pub use think_runner::{ThinkOptions, ThinkResult, run_think};
