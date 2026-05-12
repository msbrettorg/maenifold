use std::{future::Future, time::Duration};

use serde_json::Value;
use tokio::runtime::{Builder, Handle};

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};

use super::resource_notifications::subscribe_pipeline_updates;
use super::view::{is_pipeline_terminal_status, view_pipeline_session_value};

#[cfg(test)]
static ACTIVE_WAITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub(super) fn wait_requested(payload: &Value) -> bool {
    payload
        .get("wait")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(super) fn parse_timeout_ms(payload: &Value) -> Result<Option<u64>> {
    match payload.get("timeoutMs") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value.as_u64().map(Some).ok_or_else(|| {
            MaenifoldError::InvalidArgument("timeoutMs must be an unsigned integer".to_string())
        }),
        Some(_) => Err(MaenifoldError::InvalidArgument(
            "timeoutMs must be an unsigned integer".to_string(),
        )),
    }
}

pub(super) fn await_terminal(
    context: &ToolContext,
    session_id: &str,
    timeout_ms: Option<u64>,
) -> Result<String> {
    let result = block_on_wait(await_terminal_view(context, session_id, timeout_ms))?;
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

pub(super) async fn await_terminal_view(
    context: &ToolContext,
    session_id: &str,
    timeout_ms: Option<u64>,
) -> Result<Value> {
    #[cfg(test)]
    let _guard = ActiveWaitGuard::new();
    let notifier = subscribe_pipeline_updates(session_id);
    let cancel = context.cancellation_token.clone();
    let timeout = timeout_ms.map(|ms| tokio::time::sleep(Duration::from_millis(ms)));
    tokio::pin!(timeout);

    loop {
        if cancel.as_ref().is_some_and(|token| token.is_cancelled()) {
            return Err(MaenifoldError::Message(
                "Pipeline wait cancelled by MCP request abort".to_string(),
            ));
        }
        let notified = notifier.notified();
        let mut view = view_pipeline_session_value(context, session_id)?;
        let status = view.get("status").and_then(Value::as_str).unwrap_or("");
        let final_status = is_pipeline_terminal_status(status);
        view["final"] = Value::Bool(final_status);
        if final_status {
            return Ok(view);
        }

        match (timeout.as_mut().as_pin_mut(), cancel.as_ref()) {
            (Some(timeout), Some(cancel)) => {
                tokio::select! {
                    () = notified => {}
                    () = cancel.cancelled() => {
                        return Err(MaenifoldError::Message(
                            "Pipeline wait cancelled by MCP request abort".to_string(),
                        ));
                    }
                    () = timeout => return timed_out_view(context, session_id),
                }
            }
            (Some(timeout), None) => {
                tokio::select! {
                    () = notified => {}
                    () = timeout => return timed_out_view(context, session_id),
                }
            }
            (None, Some(cancel)) => {
                tokio::select! {
                    () = notified => {}
                    () = cancel.cancelled() => {
                        return Err(MaenifoldError::Message(
                            "Pipeline wait cancelled by MCP request abort".to_string(),
                        ));
                    }
                }
            }
            (None, None) => notified.await,
        }
    }
}

#[cfg(test)]
pub(super) fn active_wait_count() -> usize {
    ACTIVE_WAITS.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(test)]
struct ActiveWaitGuard;

#[cfg(test)]
impl ActiveWaitGuard {
    fn new() -> Self {
        ACTIVE_WAITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self
    }
}

#[cfg(test)]
impl Drop for ActiveWaitGuard {
    fn drop(&mut self) {
        ACTIVE_WAITS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

fn timed_out_view(context: &ToolContext, session_id: &str) -> Result<Value> {
    let mut view = view_pipeline_session_value(context, session_id)?;
    view["final"] = Value::Bool(false);
    view["timedOut"] = Value::Bool(true);
    Ok(view)
}

fn block_on_wait<T>(future: impl Future<Output = T>) -> T {
    if let Ok(handle) = Handle::try_current() {
        return tokio::task::block_in_place(|| handle.block_on(future));
    }

    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("pipeline wait runtime")
        .block_on(future)
}
