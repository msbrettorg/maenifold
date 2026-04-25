use agent_client_protocol as acp;

use super::super::session::resume::sanitized_resume_failure_kind;
use super::types::{AcpExecuteError, AcpExecuteSuccess};

pub(crate) fn resume_start_error(message: String, requested_resume: bool) -> AcpExecuteError {
    AcpExecuteError {
        resume_failure_kind: requested_resume
            .then(|| sanitized_resume_failure_kind(&message).to_string()),
        message,
        requested_resume,
        resumed: false,
    }
}

pub(crate) fn generic_error(
    message: String,
    requested_resume: bool,
    resumed: bool,
) -> AcpExecuteError {
    AcpExecuteError {
        resume_failure_kind: None,
        message,
        requested_resume,
        resumed,
    }
}

pub(crate) fn prompt_error(
    message: String,
    requested_resume: bool,
    resumed: bool,
) -> AcpExecuteError {
    AcpExecuteError {
        resume_failure_kind: requested_resume
            .then(|| sanitized_resume_failure_kind(&message).to_string())
            .filter(|_| !resumed),
        message,
        requested_resume,
        resumed,
    }
}

pub(crate) fn normalize_prompt_result_after_cancel(
    result: Result<AcpExecuteSuccess, AcpExecuteError>,
    was_cancelled: bool,
    session_id: &acp::SessionId,
    resumed: bool,
) -> Result<AcpExecuteSuccess, AcpExecuteError> {
    if was_cancelled {
        Ok(AcpExecuteSuccess {
            output: String::new(),
            session_id: session_id.to_string(),
            resumed,
        })
    } else {
        result
    }
}

#[cfg(test)]
#[path = "resume_execute_tests.rs"]
mod resume_execute_tests;
