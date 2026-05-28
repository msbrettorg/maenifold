// T-01.9: Extracted from mcp/mod.rs to keep mod.rs under the 300 LOC ceiling.
// Hosts the MCP tool definition shape, tool listing, error formatting helpers,
// and the transient-lock retry wrapper used by the MCP dispatch layer.

use super::schema::mcp_tool_from_registry_name;
use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use rmcp::model::{Meta, Tool, ToolExecution};
use serde::Serialize;
use serde_json::{Map, Value};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ToolExecution>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl McpToolDefinition {
    pub(crate) fn into_rmcp_tool(self) -> Tool {
        let schema = match self.input_schema {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        let mut tool = Tool::new(self.name, self.description, Arc::new(schema));
        tool.execution = self.execution;
        tool.meta = self.meta.map(Meta);
        tool
    }
}

// T-5.2: RTM FR-22.4 maps the registry's canonical tool set into MCP tools/list metadata.
pub fn list_mcp_tools(registry: &ToolRegistry) -> Result<Vec<McpToolDefinition>> {
    registry
        .canonical_names()
        .into_iter()
        .map(mcp_tool_from_registry_name)
        .collect()
}

pub(super) fn format_tool_error(tool_name: &str, err: &MaenifoldError) -> String {
    if is_validation_error(err) {
        err.to_string()
    } else {
        format!("Error invoking {tool_name}: {err}")
    }
}

// T-205.716: Detect transient lock-contention errors that should be retried at the
// MCP dispatch boundary.
//
// EDEADLK (errno 11 on macOS, errno 35 on Linux) is returned by the kernel from
// `fcntl(F_SETLKW)` when a circular wait is detected on POSIX record locks.
// SQLite is the only consumer of these locks in maenifold, but the resulting
// error can surface on whichever thread happens to be in the kernel wait at the
// moment of detection. It then bubbles up as a `std::io::Error` and is
// attributed to whichever MCP tool was being dispatched on that thread (often a
// lock-light operation like ListAssets that simply got unlucky under load).
//
// SQLite's own SQLITE_BUSY/SQLITE_LOCKED errors are also retried here as defense
// in depth. SQLite's per-connection `busy_timeout` already retries internally;
// this catches the rare case where the timeout expires with the lock still held.
pub(super) fn is_transient_lock_error(err: &MaenifoldError) -> bool {
    // EDEADLK errno values differ across Unix platforms.
    const EDEADLK_DARWIN: i32 = 11;
    const EDEADLK_LINUX: i32 = 35;
    match err {
        MaenifoldError::Io(io_err) => matches!(
            io_err.raw_os_error(),
            Some(EDEADLK_DARWIN) | Some(EDEADLK_LINUX)
        ),
        MaenifoldError::Sqlite(rusqlite::Error::SqliteFailure(code, _)) => matches!(
            code.code,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
        ),
        _ => false,
    }
}

// T-205.716: Retry an MCP dispatch closure when the kernel returns a transient
// lock-contention error. Bounded attempts with exponential backoff so that
// genuine deadlocks still surface (eventually) rather than spinning forever.
//
// Total worst-case wait: 25 + 50 + 100 + 200 = 375ms across 4 retries before
// the 5th attempt's error is returned to the caller. Combined with the 5s
// SQLite busy_timeout per attempt, total worst case stays well under the
// 60s ACP connection timeout, so genuine deadlocks (as opposed to brief
// contention) still produce a clear error.
pub(super) fn with_lock_retry<F, T>(mut op: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    const MAX_ATTEMPTS: u32 = 5;
    let mut delay_ms: u64 = 25;
    for attempt in 0..MAX_ATTEMPTS {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) => {
                if is_transient_lock_error(&err) && attempt + 1 < MAX_ATTEMPTS {
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    delay_ms = delay_ms.saturating_mul(2).min(500);
                    continue;
                }
                return Err(err);
            }
        }
    }
    // Loop always returns within the body; this is unreachable but keeps the
    // type checker happy for the explicit MAX_ATTEMPTS bound.
    Err(MaenifoldError::Message(
        "Lock retry exhausted without an error".to_string(),
    ))
}

pub(super) fn is_validation_error(err: &MaenifoldError) -> bool {
    matches!(
        err,
        MaenifoldError::UnknownTool(_)
            | MaenifoldError::InvalidPayload(_)
            | MaenifoldError::InvalidArgument(_)
            | MaenifoldError::InvalidResourceUri(_)
            | MaenifoldError::AssetNotFound(_, _)
            | MaenifoldError::PathTraversal(_)
            | MaenifoldError::MissingProperty(_)
    )
}
