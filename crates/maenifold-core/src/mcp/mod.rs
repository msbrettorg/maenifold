// T-12.0: RTM NFR-1 — consolidate MCP runtime into a directory module while preserving
// the existing crate::mcp facade and resource entrypoints.
pub mod http;
pub mod resources;
pub mod schema;
pub use resources::*;

// FR-32-SEC: Shared CORS factory restricting origins to localhost variants
// while keeping all methods and headers allowed for MCP transport.
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

use self::schema::mcp_tool_from_registry_name;
use crate::assets::AssetManager;
use crate::config::Config;
use crate::db::GraphDatabase;
use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListResourcesResult,
    ListToolsResult, ReadResourceRequestParams, ReadResourceResult, ServerCapabilities, ServerInfo,
    Tool,
};
use rmcp::{ServerHandler, ServiceExt};
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::Arc;

pub struct MaenifoldMcpServer;

// T-5.1: RTM FR-22.1 — rmcp-based async MCP handler.
struct MaenifoldHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportKind {
    Stdio,
    Http,
}

impl MaenifoldHandler {
    fn new(transport: TransportKind) -> Result<Self> {
        let mcp_tools = list_mcp_tools(&ToolRegistry::new())?;
        let tools = mcp_tools
            .into_iter()
            .map(|t| {
                let schema: serde_json::Map<String, Value> = match t.input_schema {
                    Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };
                Tool::new(t.name, t.description, Arc::new(schema))
            })
            .collect();
        Ok(Self { transport, tools })
    }
}

impl ServerHandler for MaenifoldHandler {
    fn get_info(&self) -> ServerInfo {
        // T-10.2: RTM FR-11 / RTM-068 — MCP initialize advertises native resources support.
        // T-5.1: RTM FR-22.1 — server info returned during MCP initialize handshake.
        let capabilities = match self.transport {
            TransportKind::Stdio => ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                // T-10.3: RTM FR-11 / RTM-071 — advertise MCP resource list change
                // notifications so clients know to expect notifications/resources/list_changed.
                .enable_resources_list_changed()
                .build(),
            TransportKind::Http => ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        };

        ServerInfo::new(capabilities)
            .with_server_info(Implementation::new("maenifold", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Use maenifold tools to read, write, search, and organize memory files, \
                 inspect graph relationships, and run workflow or thinking sessions. \
                 Prefer exact tool names from tools/list and pass JSON arguments that \
                 match each tool schema.",
            )
    }

    // T-5.2: RTM FR-22.4 — tools/list returns all 28 tool schemas.
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

    // T-10.2: RTM FR-11 / RTM-069 — resources/list exposes discoverable asset URIs across all 4 types.
    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListResourcesResult, rmcp::ErrorData>> + Send + '_
    {
        async move {
            tokio::task::spawn_blocking(self::resources::handle_list_resources)
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?
                .map_err(self::resources::map_resource_error)
        }
    }

    // T-10.2: RTM FR-11 / RTM-070 — resources/read resolves asset://{type}/{id} URIs to JSON payloads.
    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        let uri = request.uri.to_string();

        async move {
            tokio::task::spawn_blocking(move || self::resources::handle_read_resource(&uri))
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?
                .map_err(self::resources::map_resource_error)
        }
    }

    // T-5.1: RTM FR-22.1 — tools/call dispatches through ToolRegistry.
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<CallToolResult, rmcp::ErrorData>> + Send + '_
    {
        let tool_name = request.name.to_string();
        let arguments: Value = match request.arguments {
            Some(map) => Value::Object(map),
            None => json!({}),
        };
        let tool_name_for_invoke = tool_name.clone();

        async move {
            let result = tokio::task::spawn_blocking(move || {
                ToolRegistry::global().try_invoke(
                    &tool_name_for_invoke,
                    &arguments,
                    OutputFormat::Json,
                )
            })
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

            match result {
                Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
                // T-11.6: RTM FR-11 preserves raw validation diagnostics while prefixing internal failures.
                Err(err) => Ok(CallToolResult::error(vec![Content::text(
                    format_tool_error(&tool_name, &err),
                )])),
            }
        }
    }
}

impl MaenifoldMcpServer {
    pub fn list_tools() -> Result<Vec<McpToolDefinition>> {
        list_mcp_tools(ToolRegistry::global())
    }

    // T-5.1: RTM FR-22.1 — serve MCP over stdio using rmcp transport.
    pub async fn serve_stdio() -> Result<()> {
        let config = Config::current();
        initialize_runtime(&config)?;
        let handler = MaenifoldHandler::new(TransportKind::Stdio)?;
        let transport = rmcp::transport::io::stdio();
        let service = handler
            .serve(transport)
            .await
            .map_err(|e| MaenifoldError::Message(format!("MCP server init failed: {e}")))?;
        // T-12.0: RTM FR-11 / RTM-071 — delegate asset-resource change watching to
        // resources.rs so mod.rs stays focused on MCP server lifecycle and tool dispatch.
        let asset_watcher = self::resources::start_asset_watcher(&config, service.peer().clone())?;
        let wait_result = service
            .waiting()
            .await
            .map_err(|e| MaenifoldError::Message(format!("MCP server error: {e}")));
        asset_watcher.stop();
        wait_result?;
        Ok(())
    }
}

pub fn initialize_runtime(config: &Config) -> Result<()> {
    config.ensure_directories()?;
    AssetManager::initialize_assets(config)?;
    GraphDatabase::initialize_database(config)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// T-5.2: RTM FR-22.4 maps the registry's canonical tool set into MCP tools/list metadata.
pub fn list_mcp_tools(registry: &ToolRegistry) -> Result<Vec<McpToolDefinition>> {
    registry
        .canonical_names()
        .into_iter()
        .map(mcp_tool_from_registry_name)
        .collect()
}

fn format_tool_error(tool_name: &str, err: &MaenifoldError) -> String {
    if is_validation_error(err) {
        err.to_string()
    } else {
        format!("Error invoking {tool_name}: {err}")
    }
}

fn is_validation_error(err: &MaenifoldError) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // T-5.2: RTM FR-22.4 — all 28 tools are represented in the MCP schema mapping.
    #[test]
    fn t_5_2_registry_mapping_covers_all_tools() {
        let tools = list_mcp_tools(&ToolRegistry::new()).expect("registry mapping should succeed");
        let names = tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(tools.len(), 28);
        assert_eq!(names.len(), 28);
        assert!(names.contains("WriteMemory"));
        assert!(names.contains("ReadMcpResource"));
        assert!(names.contains("RunFullBenchmark"));
    }

    // T-5.2: RTM FR-22.4 — WriteMemory schema declares required fields.
    #[test]
    fn t_5_2_write_memory_schema_requires_title_and_content() {
        let tools = list_mcp_tools(&ToolRegistry::new()).expect("registry mapping should succeed");
        let write_memory = tools
            .into_iter()
            .find(|tool| tool.name == "WriteMemory")
            .expect("WriteMemory tool should exist");

        let required = write_memory
            .input_schema
            .get("required")
            .and_then(Value::as_array)
            .expect("required fields should be present")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();

        assert_eq!(required, vec!["title", "content"]);
    }

    // T-5.2: RTM FR-22.4 — canonical names survive the McpToolDefinition mapping.
    #[test]
    fn t_5_2_registry_names_match_canonical_names() {
        let tools = list_mcp_tools(&ToolRegistry::new()).expect("registry mapping should succeed");
        let names = tools.into_iter().map(|tool| tool.name).collect::<Vec<_>>();

        assert!(names.contains(&"WriteMemory".to_string()));
        assert!(names.contains(&"ReadMcpResource".to_string()));
        assert!(names.contains(&"RunFullBenchmark".to_string()));
    }

    // T-5.1: RTM FR-22.1 — handler builds and exposes correct server info.
    #[test]
    fn t_5_1_handler_server_info_matches_expected() {
        let handler = MaenifoldHandler::new(TransportKind::Stdio).expect("handler should build");
        let info = handler.get_info();

        assert_eq!(info.server_info.name, "maenifold");
        assert!(!info.server_info.version.is_empty());
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
        assert_eq!(
            info.capabilities
                .resources
                .as_ref()
                .and_then(|resources| resources.list_changed),
            Some(true)
        );
        assert!(info.instructions.is_some());
        assert!(
            info.instructions
                .as_ref()
                .unwrap()
                .contains("maenifold tools")
        );
    }

    // T-5.1: RTM FR-22.1 — handler converts all 28 McpToolDefinitions into rmcp Tools.
    #[test]
    fn t_5_1_handler_builds_28_rmcp_tools() {
        let handler = MaenifoldHandler::new(TransportKind::Stdio).expect("handler should build");
        assert_eq!(handler.tools.len(), 28);

        let names: BTreeSet<&str> = handler.tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains("WriteMemory"));
        assert!(names.contains("SequentialThinking"));
        assert!(names.contains("ReadMcpResource"));
    }

    #[test]
    fn t_27_4_http_handler_does_not_advertise_resource_list_changed() {
        let handler = MaenifoldHandler::new(TransportKind::Http).expect("handler should build");
        let info = handler.get_info();

        assert!(info.capabilities.resources.is_some());
        assert_eq!(
            info.capabilities
                .resources
                .as_ref()
                .and_then(|resources| resources.list_changed),
            None
        );
    }

    #[test]
    fn t_27_5_message_errors_are_not_validation_errors() {
        let err = MaenifoldError::Message("database unavailable".to_string());

        assert!(!is_validation_error(&err));
        assert_eq!(
            format_tool_error("WriteMemory", &err),
            "Error invoking WriteMemory: database unavailable"
        );
    }

    #[test]
    fn t_27_5_message_resource_errors_map_to_internal_error() {
        let err = self::resources::map_resource_error(MaenifoldError::Message(
            "database unavailable".to_string(),
        ));

        assert_eq!(err.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
        assert_eq!(err.message.as_ref(), "database unavailable");
    }
}
