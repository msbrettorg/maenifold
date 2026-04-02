pub mod http;
pub mod schema;

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use axum::http::{HeaderName, HeaderValue, request::Parts};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::{ServerHandler, ServiceExt};
use serde_json::{Map, Value};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use self::schema::{McpToolDefinition, list};
use crate::cli::{AppendArgs, GateContext, OutcomeArgs, StatusArgs, SyncArgs};
use crate::error::GateError;
use crate::gate_tool_service::GateToolService;

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

pub(crate) struct GateMcpHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
}

impl GateMcpHandler {
    pub(crate) fn new(transport: TransportKind) -> Result<Self, GateError> {
        let tools = list()
            .into_iter()
            .map(to_rmcp_tool)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { transport, tools })
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
            .with_instructions(
                "Use the gate tools to inspect and mutate file-authoritative task state. \
                 Prefer exact tool names from tools/list and pass JSON arguments matching the tool schema.",
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
        let arguments = match request.arguments {
            Some(map) => Value::Object(map),
            None => Value::Object(Map::new()),
        };

        async move {
            match call_gate_tool(&tool_name, arguments) {
                Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
                Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
            }
        }
    }
}

impl GateMcpServer {
    pub async fn serve_stdio() -> Result<(), GateError> {
        let handler = GateMcpHandler::new(TransportKind::Stdio)?;
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

fn to_rmcp_tool(definition: McpToolDefinition) -> Result<Tool, GateError> {
    let schema = match definition.input_schema {
        Value::Object(map) => map,
        _ => {
            return Err(GateError::Cli(
                "tool schema must be a JSON object".to_string(),
            ));
        }
    };
    Ok(Tool::new(
        definition.name,
        definition.description,
        Arc::new(schema),
    ))
}

fn call_gate_tool(tool_name: &str, arguments: Value) -> Result<String, GateError> {
    let result = match tool_name {
        "status" => GateToolService::status(StatusArgs {
            context: context_from_value(&arguments, false)?,
        })?,
        "append" => GateToolService::append(AppendArgs {
            context: context_from_value(&arguments, false)?,
            task_id: required_string(&arguments, "taskId")?,
            agent: required_string(&arguments, "agent")?,
            notes: required_string(&arguments, "notes")?,
            model: optional_string(&arguments, "model")?,
        })?,
        "pass" => GateToolService::pass(OutcomeArgs {
            context: context_from_value(&arguments, false)?,
            task_id: required_string(&arguments, "taskId")?,
            agent: required_string(&arguments, "agent")?,
            notes: required_string(&arguments, "notes")?,
            model: optional_string(&arguments, "model")?,
        })?,
        "fail" => GateToolService::fail(OutcomeArgs {
            context: context_from_value(&arguments, false)?,
            task_id: required_string(&arguments, "taskId")?,
            agent: required_string(&arguments, "agent")?,
            notes: required_string(&arguments, "notes")?,
            model: optional_string(&arguments, "model")?,
        })?,
        "sync" => GateToolService::sync(SyncArgs {
            context: context_from_value(&arguments, false)?,
            rebuild: optional_bool(&arguments, "rebuild")?.unwrap_or(false),
        })?,
        _ => {
            return Err(GateError::Cli(format!(
                "unknown gate MCP tool: {tool_name}"
            )));
        }
    };

    serde_json::to_string_pretty(&result).map_err(GateError::from)
}

fn context_from_value(arguments: &Value, json_output: bool) -> Result<GateContext, GateError> {
    Ok(GateContext {
        root: optional_string(arguments, "root")?
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
        requested_pipeline_name: optional_string(arguments, "pipeline")?,
        json_output,
    })
}

fn required_string(arguments: &Value, key: &str) -> Result<String, GateError> {
    optional_string(arguments, key)?
        .ok_or_else(|| GateError::Cli(format!("missing required MCP argument '{key}'")))
}

fn optional_string(arguments: &Value, key: &str) -> Result<Option<String>, GateError> {
    match arguments.get(key) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(GateError::Cli(format!(
            "MCP argument '{key}' must be a string"
        ))),
    }
}

fn optional_bool(arguments: &Value, key: &str) -> Result<Option<bool>, GateError> {
    match arguments.get(key) {
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(GateError::Cli(format!(
            "MCP argument '{key}' must be a boolean"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde_json::json;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;

    use super::call_gate_tool;

    #[test]
    fn mcp_status_tool_returns_task_array() {
        let root = new_custom_pipeline_root();
        let output = call_gate_tool("status", json!({ "root": root.path() }))
            .expect("status tool should succeed");
        let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");
        let tasks = value.as_array().expect("status should return an array");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["taskId"], "T-9.1");
        assert_eq!(tasks[0]["status"], "pending");
    }

    #[test]
    fn mcp_pass_tool_advances_bound_task() {
        let root = new_custom_pipeline_root();
        let output = call_gate_tool(
            "pass",
            json!({
                "root": root.path(),
                "taskId": "T-9.1",
                "agent": "swe",
                "notes": "swe completed"
            }),
        )
        .expect("pass tool should succeed");
        let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");
        assert_eq!(value["taskId"], "T-9.1");
        assert_eq!(value["status"], "red");
    }

    fn new_custom_pipeline_root() -> TempDir {
        let temp = tempfile::tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");

        let pipeline_json = json!({
            "schemaVersion": 1,
            "id": "custom",
            "name": "Custom",
            "initial": "pending",
            "states": {
                "pending": { "on": { "pass": "swe" } },
                "swe": { "on": { "fail": "pending", "pass": "red" } },
                "red": { "on": { "fail": "pending", "pass": "done" } },
                "done": { "terminal": true }
            }
        });
        let pipeline_text =
            serde_json::to_string_pretty(&pipeline_json).expect("serialize pipeline json");
        fs::write(root.join("pipeline.json"), &pipeline_text).expect("write pipeline");
        let digest = digest_hex(pipeline_text.as_bytes());

        fs::write(
            root.join("todo").join("T-9.1.md"),
            task_markdown("T-9.1", "custom", &digest),
        )
        .expect("write task");
        temp
    }

    fn task_markdown(task_id: &str, pipeline: &str, digest: &str) -> String {
        format!(
            "# {task_id} — MCP Test\n\n**Pipeline**: {pipeline}\n**Pipeline Digest**: {digest}\n\n## Acceptance Criteria\n\n1. tool call succeeds\n"
        )
    }

    fn digest_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    #[allow(dead_code)]
    fn _assert_exists(path: &Path) {
        assert!(path.exists(), "missing path {}", path.display());
    }
}
