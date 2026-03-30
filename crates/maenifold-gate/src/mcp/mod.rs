pub mod http;
pub mod tools;

pub(crate) fn localhost_cors() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

    let origin_predicate = AllowOrigin::predicate(
        |origin: &axum::http::HeaderValue, _parts: &axum::http::request::Parts| {
            let Ok(s) = origin.to_str() else {
                return false;
            };
            s == "http://localhost"
                || s.starts_with("http://localhost:")
                || s == "http://127.0.0.1"
                || s.starts_with("http://127.0.0.1:")
                || s == "http://[::1]"
                || s.starts_with("http://[::1]:")
        },
    );

    CorsLayer::new()
        .allow_origin(origin_predicate)
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .expose_headers([axum::http::HeaderName::from_static("mcp-session-id")])
}

use crate::trace::{Direction, TraceBuffer, TraceEntry};
use chrono::Utc;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler, ServiceExt};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Mutex};

pub struct GateMcpServer;

struct GateHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
    bound_root: Mutex<Option<PathBuf>>,
    trace_buffer: Option<TraceBuffer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportKind {
    Stdio,
    Http,
}

impl GateHandler {
    fn new(transport: TransportKind, trace_buffer: Option<TraceBuffer>) -> Self {
        Self {
            transport,
            tools: self::tools::build_tool_list(),
            bound_root: Mutex::new(initial_bound_root(transport)),
            trace_buffer,
        }
    }

    fn trace_request_id(context: &RequestContext<RoleServer>) -> Option<Value> {
        serde_json::to_value(context.id.clone()).ok()
    }

    fn trace_session_id(context: &RequestContext<RoleServer>) -> Option<String> {
        context
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| parts.headers.get("mcp-session-id"))
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned)
    }

    fn record_trace(
        &self,
        direction: Direction,
        method: &str,
        id: Option<Value>,
        session_id: Option<String>,
        body: serde_json::Value,
    ) {
        let Some(trace_buffer) = &self.trace_buffer else {
            return;
        };

        trace_buffer.push(TraceEntry {
            timestamp: Utc::now().format("%H:%M:%S%.3f").to_string(),
            direction,
            method: method.to_string(),
            id,
            body: serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()),
            session_id,
        });
    }

    fn record_incoming_tool_call(
        &self,
        tool_name: &str,
        request_id: Option<Value>,
        session_id: Option<String>,
        arguments: &serde_json::Map<String, Value>,
    ) {
        self.record_trace(
            Direction::Incoming,
            tool_name,
            request_id.clone(),
            session_id,
            json!({
                "jsonrpc": "2.0",
                "id": request_id.clone().unwrap_or(Value::Null),
                "method": "tools/call",
                "params": {
                    "name": tool_name,
                    "arguments": arguments,
                }
            }),
        );
    }

    fn record_outgoing_tool_call(
        &self,
        tool_name: &str,
        request_id: Option<Value>,
        session_id: Option<String>,
        response: &Result<CallToolResult, rmcp::ErrorData>,
    ) {
        let body = match response {
            Ok(result) => json!({
                "jsonrpc": "2.0",
                "id": request_id.clone().unwrap_or(Value::Null),
                "result": result,
            }),
            Err(err) => json!({
                "jsonrpc": "2.0",
                "id": request_id.clone().unwrap_or(Value::Null),
                "error": err,
            }),
        };

        self.record_trace(Direction::Outgoing, tool_name, request_id, session_id, body);
    }
}

fn initial_bound_root(transport: TransportKind) -> Option<PathBuf> {
    if transport != TransportKind::Http {
        return None;
    }
    std::env::current_dir().ok().and_then(|root| {
        if root.join("todo").is_dir() || root.join("done").is_dir() {
            Some(root)
        } else {
            None
        }
    })
}

impl ServerHandler for GateHandler {
    fn get_info(&self) -> ServerInfo {
        let capabilities = match self.transport {
            TransportKind::Stdio | TransportKind::Http => {
                ServerCapabilities::builder().enable_tools().build()
            }
        };

        ServerInfo::new(capabilities)
            .with_server_info(Implementation::new("gate", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Use gate tools to inspect task state, append agent notes, and move tasks \
                 through pass/reject transitions. Call gate_sync with a root path to bind \
                 the session before relying on rootless tool calls. Prefer exact tool names \
                 from tools/list and pass JSON arguments that match each tool schema.",
            )
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListToolsResult, rmcp::ErrorData>> + Send + '_
    {
        std::future::ready(Ok(ListToolsResult {
            tools: self.tools.clone(),
            next_cursor: None,
            meta: None,
        }))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<CallToolResult, rmcp::ErrorData>> + Send + '_
    {
        let tool_name = request.name.to_string();
        let mut arguments = request.arguments.clone().unwrap_or_default();
        let tool_name_for_dispatch = tool_name.clone();
        let request_id = Self::trace_request_id(&context);
        let session_id = Self::trace_session_id(&context);
        self.record_incoming_tool_call(
            &tool_name,
            request_id.clone(),
            session_id.clone(),
            &arguments,
        );
        let bound_root = self
            .bound_root
            .lock()
            .map(|guard| guard.clone())
            .map_err(|err| {
                rmcp::ErrorData::internal_error(
                    format!("failed to read bound gate root: {err}"),
                    None,
                )
            });

        async move {
            let bound_root = bound_root?;
            let response = if requires_bound_root(&tool_name)
                && !arguments.contains_key("root")
                && bound_root.is_none()
            {
                Ok(CallToolResult::error(vec![Content::text(
                    self::tools::SESSION_INIT_ERROR.to_string(),
                )]))
            } else {
                let result = tokio::task::spawn_blocking(move || {
                    self::tools::dispatch(
                        &tool_name_for_dispatch,
                        &mut arguments,
                        bound_root.as_deref(),
                    )
                })
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

                match result {
                    Ok(result) => {
                        if let Some(root) = result.synced_root {
                            let mut guard = self.bound_root.lock().map_err(|err| {
                                rmcp::ErrorData::internal_error(
                                    format!("failed to update bound gate root: {err}"),
                                    None,
                                )
                            })?;
                            *guard = Some(root);
                        }

                        Ok(CallToolResult::success(vec![Content::text(result.output)]))
                    }
                    Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
                }
            };

            self.record_outgoing_tool_call(
                &tool_name,
                request_id.clone(),
                session_id.clone(),
                &response,
            );

            response
        }
    }
}

fn requires_bound_root(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "gate_pass" | "gate_reject" | "gate_append" | "gate_status"
    )
}

impl GateMcpServer {
    pub async fn serve_stdio() -> anyhow::Result<()> {
        let handler = GateHandler::new(TransportKind::Stdio, None);
        let transport = rmcp::transport::io::stdio();
        let service = handler
            .serve(transport)
            .await
            .map_err(|e| anyhow::anyhow!("MCP server init failed: {e}"))?;
        service
            .waiting()
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
        Ok(())
    }
}
