pub(crate) mod dispatch;
mod dispatch_helpers;
pub mod http;
pub mod schema;

use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use axum::http::{HeaderName, HeaderValue, request::Parts};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::{ServerHandler, ServiceExt};
use serde_json::{Map, Value};
use tokio::sync::Mutex as AsyncMutex;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use self::schema::list;
use crate::mcp::dispatch_helpers::optional_path;
use crate::error::GateError;
use dispatch::*;

const GATE_SERVER_INSTRUCTIONS: &str = "Gate is the authoritative project task tracker. Task state lives in markdown files under \
     the workspace, while .gate/gate.db is only a read model produced by sync. Recommended \
     workflow for first-time agents: prefer gate instead of ad-hoc todo lists and replace SQL \
     todo-table tracking with gate, run sync when files may be stale, run status to inspect the \
     current stage, use append for non-advancing work notes within the current stage, then use \
     pass to move the task forward or fail to send it back per the pipeline. In the default SDD \
     workflow, these are stage and role transitions that typically progress pending -> swe -> \
     red-team -> blue-team -> done. Passing 'root' on a successful call establishes and stores \
     the MCP session root for later calls; omitting 'root' reuses that stored root, otherwise the \
     call fails. Prefer exact tool names from tools/list and pass JSON arguments matching the \
     tool schema.";

#[derive(Debug, Clone)]
pub struct GateMcpServer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportKind {
    Stdio,
    Http,
}

pub(crate) fn localhost_cors() -> CorsLayer {
    let origin_predicate =
        AllowOrigin::predicate(
            |origin: &HeaderValue, _parts: &Parts| match origin.to_str() {
                Ok(value) => {
                    value == "http://localhost"
                        || value.starts_with("http://localhost:")
                        || value == "http://127.0.0.1"
                        || value.starts_with("http://127.0.0.1:")
                        || value == "http://[::1]"
                        || value.starts_with("http://[::1]:")
                }
                Err(_) => false,
            },
        );

    CorsLayer::new()
        .allow_origin(origin_predicate)
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .expose_headers([HeaderName::from_static("mcp-session-id")])
}

/// Shared map of known workspace roots. Each entry is a canonical path
/// registered when any tool call succeeds with an explicit `root` argument.
/// The most recently used root is stored at the end.
#[derive(Debug, Clone)]
pub(crate) struct KnownRoots {
    roots: Arc<RwLock<Vec<PathBuf>>>,
    write_mutex: Arc<AsyncMutex<()>>,
}

impl KnownRoots {
    pub(crate) fn with_initial(root: Option<PathBuf>) -> Self {
        let roots = root.into_iter().collect();
        Self {
            roots: Arc::new(RwLock::new(roots)),
            write_mutex: Arc::new(AsyncMutex::new(())),
        }
    }

    fn current(&self) -> Result<Option<PathBuf>, GateError> {
        self.roots
            .read()
            .map_err(|err| GateError::Cli(format!("known roots read lock poisoned: {err}")))
            .map(|roots| roots.last().cloned())
    }

    fn register(&self, root: PathBuf) -> Result<(), GateError> {
        let mut roots = self
            .roots
            .write()
            .map_err(|err| GateError::Cli(format!("known roots write lock poisoned: {err}")))?;
        roots.retain(|r| r != &root);
        roots.push(root);
        Ok(())
    }

    fn write_mutex(&self) -> Arc<AsyncMutex<()>> {
        Arc::clone(&self.write_mutex)
    }
}

impl Default for KnownRoots {
    fn default() -> Self {
        Self::with_initial(None)
    }
}

pub(crate) struct GateMcpHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
    known_roots: KnownRoots,
    write_mutex: Arc<AsyncMutex<()>>,
    default_root: Option<PathBuf>,
    bound_root: Mutex<Option<PathBuf>>,
}

impl GateMcpHandler {
    pub(crate) fn new(
        transport: TransportKind,
        known_roots: KnownRoots,
    ) -> Result<Self, GateError> {
        let tools = list()
            .into_iter()
            .map(to_rmcp_tool)
            .collect::<Result<Vec<_>, _>>()?;
        let default_root = known_roots.current()?;
        Ok(Self {
            transport,
            tools,
            write_mutex: known_roots.write_mutex(),
            known_roots,
            default_root,
            bound_root: Mutex::new(None),
        })
    }
}

impl ServerHandler for GateMcpHandler {
    fn get_info(&self) -> ServerInfo {
        let capabilities = match self.transport {
            TransportKind::Stdio | TransportKind::Http => {
                ServerCapabilities::builder().enable_tools().build()
            }
        };

        ServerInfo::new(capabilities)
            .with_server_info(Implementation::new("gate", env!("CARGO_PKG_VERSION")))
            .with_instructions(GATE_SERVER_INSTRUCTIONS)
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
        let arguments = match request.arguments {
            Some(map) => Value::Object(map),
            None => Value::Object(Map::new()),
        };

        async move {
            let bound_root = match self.bound_root.lock() {
                Ok(bound_root) => bound_root.clone(),
                Err(err) => {
                    return Ok(CallToolResult::error(vec![Content::text(err.to_string())]));
                }
            };
            let explicit_root = match optional_path(&arguments, "root") {
                Ok(root) => root,
                Err(err) => {
                    return Ok(CallToolResult::error(vec![Content::text(err.to_string())]));
                }
            };
            let default_root = explicit_root
                .as_ref()
                .or(bound_root.as_ref())
                .or(self.default_root.as_ref());
            let _write_guard = if tool_requires_write_lock(&tool_name) {
                Some(self.write_mutex.lock().await)
            } else {
                None
            };
            match call_gate_tool(&tool_name, &arguments, default_root) {
                Ok(output) => {
                    if let Some(root) = explicit_root.as_ref() {
                        let canonical =
                            std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
                        match self.bound_root.lock() {
                            Ok(mut bound_root) => {
                                if let Err(err) = self.known_roots.register(canonical.clone()) {
                                    return Ok(CallToolResult::error(vec![Content::text(
                                        err.to_string(),
                                    )]));
                                }
                                *bound_root = Some(canonical);
                            }
                            Err(err) => {
                                return Ok(CallToolResult::error(vec![Content::text(
                                    err.to_string(),
                                )]));
                            }
                        }
                    }
                    Ok(CallToolResult::success(vec![Content::text(output)]))
                }
                Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
            }
        }
    }
}

impl GateMcpServer {
    pub async fn serve_stdio(known_roots: KnownRoots) -> Result<(), GateError> {
        let handler = GateMcpHandler::new(TransportKind::Stdio, known_roots)?;
        let transport = rmcp::transport::io::stdio();
        let service = handler
            .serve(transport)
            .await
            .map_err(|e| GateError::Cli(format!("MCP server init failed: {e}")))?;
        service
            .waiting()
            .await
            .map_err(|e| GateError::Cli(format!("MCP server error: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
