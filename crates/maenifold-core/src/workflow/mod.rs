mod adoption;
mod agent_session;
mod agent_session_config;
mod agent_session_open;
mod agent_session_resume;
mod asset_delete;
#[cfg(test)]
mod asset_store_tests;
mod asset_save;
pub(crate) mod asset_store;
mod handler;
mod handler_helpers;
#[cfg(test)]
#[path = "handler_inputs_validation_tests.rs"]
mod handler_inputs_validation;
mod handler_session;
mod inputs_template;
#[cfg(test)]
mod inputs_template_tests;
mod inputs_validation;
pub mod output_parser;
pub mod output_types;
pub mod pipeline;
mod queue;
#[cfg(test)]
mod queue_inputs_test;
#[cfg(test)]
mod queue_tests;
pub mod runner;
pub(crate) mod runner_helpers;
mod runner_prompt;
mod runner_state;
mod status;
mod step_timings;
#[cfg(test)]
mod step_timings_tests;
mod submachine;
mod submachine_gate;
mod submachine_link;
mod submachine_register;
#[cfg(test)]
mod submachine_tests;
pub(crate) mod think_runner;
mod think_runner_payloads;

use std::sync::LazyLock;

use tokio::runtime::{Builder, Handle, Runtime};

use crate::mcp::resources::{notify_resource_updated, workflow::WORKFLOW_SESSIONS_URI};

// T-12.0: RTM NFR-1 consolidates the workflow facade, helpers, and session logic under one module.
pub(crate) use asset_delete::delete_workflow_asset;
pub(crate) use asset_save::save_workflow_asset;
pub(crate) use handler::workflow;
pub(crate) use handler_helpers::{
    build_workflow_response_content, format_workflow_step, format_workflow_step_with_inputs,
    parse_workflow_identifiers, read_workflow_definition, set_current_step, workflow_steps,
};
pub(crate) use inputs_validation::validate_workflow_inputs;
pub(crate) use queue::{
    append_workflow_queue, frontmatter_usize, initial_workflow_frontmatter, read_workflow_inputs,
    read_workflow_queue, start_workflow_session, view_workflow_session,
};
pub use runner::{WorkflowResult, WorkflowRunStatus};
#[allow(unused_imports)]
pub(crate) use status::{
    handle_cancelled_workflow_status, handle_completed_workflow_status, handle_queue_exhaustion,
    transition_to_next_workflow,
};
pub(crate) use step_timings::{
    WorkflowStepTimings, read_step_timings, record_step_started, record_workflow_step_ended,
    record_workflow_step_started,
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

static NOTIFICATION_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build workflow notification runtime: {error}"))
});

pub(crate) fn emit_workflow_resource_updated(session_id: &str) {
    let session_uri = format!("workflow://session/{session_id}");

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            notify_resource_updated(WORKFLOW_SESSIONS_URI).await;
            notify_resource_updated(&session_uri).await;
        });
        return;
    }

    if let Ok(runtime) = NOTIFICATION_RUNTIME.as_ref() {
        runtime.block_on(async {
            notify_resource_updated(WORKFLOW_SESSIONS_URI).await;
            notify_resource_updated(&session_uri).await;
        });
    }
}
