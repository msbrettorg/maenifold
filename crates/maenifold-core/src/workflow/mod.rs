mod adoption;
mod agent_session;
mod agent_session_config;
mod agent_session_resume;
mod handler;
mod handler_helpers;
mod handler_session;
pub mod output_parser;
pub mod output_types;
pub mod pipeline;
mod queue;
pub mod runner;
pub(crate) mod runner_helpers;
mod runner_prompt;
mod runner_state;
mod status;
mod submachine;
mod submachine_gate;
mod submachine_link;
mod submachine_register;
#[cfg(test)]
mod submachine_tests;
pub(crate) mod think_runner;
mod think_runner_payloads;

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
pub use runner::{WorkflowResult, WorkflowRunStatus};
#[allow(unused_imports)]
pub(crate) use status::{
    handle_cancelled_workflow_status, handle_completed_workflow_status, handle_queue_exhaustion,
    transition_to_next_workflow,
};
#[allow(unused_imports)]
pub(crate) use submachine::{
    HostSessionType, ResumeSubmachineResult, SubmachineType, clear_submachine_frontmatter,
    parse_submachine_type,
};
#[allow(unused_imports)]
pub(crate) use submachine_register::{
    maybe_resume_submachine, register_host_submachine, register_workflow_submachine,
};
pub use think_runner::{ThinkOptions, ThinkResult};
