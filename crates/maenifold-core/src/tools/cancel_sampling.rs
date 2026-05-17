use serde_json::{Value, json};
use uuid::Uuid;

use super::{ToolContext, render};
use crate::error::{MaenifoldError, Result};
use crate::mcp::resources::sampling;
use crate::mcp::sampling_session_registry::SamplingSessionStatus;

pub fn cancel_sampling_session(context: &ToolContext, payload: &Value) -> Result<String> {
    let sampling_request_id = payload
        .get("samplingRequestId")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("samplingRequestId"))?;
    let sampling_request_id = Uuid::parse_str(sampling_request_id)
        .map_err(|error| MaenifoldError::InvalidArgument(error.to_string()))?;
    let registry = sampling::registry().ok_or_else(|| {
        MaenifoldError::Message("sampling session registry is not initialized".to_string())
    })?;

    run_cancel(async move {
        let dispatch = registry
            .snapshot()
            .await
            .into_iter()
            .find(|dispatch| dispatch.sampling_request_id == sampling_request_id)
            .ok_or_else(|| {
                MaenifoldError::AssetNotFound(
                    "sampling-session".to_string(),
                    sampling_request_id.to_string(),
                )
            })?;

        if is_cancellable_status(&dispatch.status) {
            registry
                .update_status(sampling_request_id, SamplingSessionStatus::Cancelled)
                .await
                .map_err(|_| {
                    MaenifoldError::AssetNotFound(
                        "sampling-session".to_string(),
                        sampling_request_id.to_string(),
                    )
                })?;
            jockey::acp::cancel_session(
                dispatch.runtime_kind(),
                dispatch.role_name(),
                Some(&dispatch.app_session_id.to_string()),
            )
            .await;
        }

        Ok(json!({
            "ok": true,
            "samplingRequestId": sampling_request_id,
        }))
    })
    .map(|value| render(context.output_format, value))
}

fn is_cancellable_status(status: &SamplingSessionStatus) -> bool {
    matches!(
        status,
        SamplingSessionStatus::Dispatching
            | SamplingSessionStatus::Streaming
            | SamplingSessionStatus::BudgetExceeded { .. }
    )
}

fn run_cancel<F>(future: F) -> Result<Value>
where
    F: std::future::Future<Output = Result<Value>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => task_block_on(handle, future),
        Err(_) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| MaenifoldError::Message(error.to_string()))?;
            runtime.block_on(future)
        }
    }
}

fn task_block_on<F>(handle: tokio::runtime::Handle, future: F) -> Result<Value>
where
    F: std::future::Future<Output = Result<Value>>,
{
    tokio::task::block_in_place(|| handle.block_on(future))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_exceeded_sessions_are_cancellable() {
        assert!(is_cancellable_status(&SamplingSessionStatus::Dispatching));
        assert!(is_cancellable_status(&SamplingSessionStatus::Streaming));
        assert!(is_cancellable_status(
            &SamplingSessionStatus::BudgetExceeded {
                allowed_so_far: 10,
                budget: 10,
            }
        ));
        assert!(!is_cancellable_status(&SamplingSessionStatus::Completed));
        assert!(!is_cancellable_status(&SamplingSessionStatus::Cancelled));
        assert!(!is_cancellable_status(&SamplingSessionStatus::Failed));
    }
}
