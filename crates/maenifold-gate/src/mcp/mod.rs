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

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::{ServerHandler, ServiceExt};
use std::{path::PathBuf, sync::Mutex};

pub struct GateMcpServer;

struct GateHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
    bound_root: Mutex<Option<PathBuf>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportKind {
    Stdio,
    Http,
}

impl GateHandler {
    fn new(transport: TransportKind) -> Self {
        Self {
            transport,
            tools: self::tools::build_tool_list(),
            bound_root: Mutex::new(None),
        }
    }
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
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<CallToolResult, rmcp::ErrorData>> + Send + '_
    {
        let tool_name = request.name.to_string();
        let mut arguments = request.arguments.unwrap_or_default();
        let tool_name_for_dispatch = tool_name.clone();
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
            if requires_bound_root(&tool_name)
                && !arguments.contains_key("root")
                && bound_root.is_none()
            {
                return Ok(CallToolResult::error(vec![Content::text(
                    self::tools::SESSION_INIT_ERROR.to_string(),
                )]));
            }
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
        let handler = GateHandler::new(TransportKind::Stdio);
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
