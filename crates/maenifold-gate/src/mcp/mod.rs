pub mod http;
pub mod schema;

use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::http::{HeaderName, HeaderValue, request::Parts};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::{ServerHandler, ServiceExt};
use serde_json::{Map, Value};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use self::schema::{McpToolDefinition, list};
use crate::cli::{AppendArgs, GateContext, HistoryArgs, OutcomeArgs, StatusArgs, SyncArgs};
use crate::error::GateError;
use crate::gate_tool_service::GateToolService;
use crate::status_filter::StatusFilter;

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
#[derive(Debug, Clone, Default)]
pub(crate) struct KnownRoots(Arc<RwLock<Vec<PathBuf>>>);

impl KnownRoots {
    pub(crate) fn with_initial(root: Option<PathBuf>) -> Self {
        let roots = root.into_iter().collect();
        Self(Arc::new(RwLock::new(roots)))
    }

    fn current(&self) -> Option<PathBuf> {
        self.0.read().unwrap().last().cloned()
    }

    fn register(&self, root: PathBuf) {
        let mut roots = self.0.write().unwrap();
        roots.retain(|r| r != &root);
        roots.push(root);
    }
}

pub(crate) struct GateMcpHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
    known_roots: KnownRoots,
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
        Ok(Self {
            transport,
            tools,
            known_roots,
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
            let current_root = self.known_roots.current();
            let explicit_root = match optional_path(&arguments, "root") {
                Ok(root) => root,
                Err(err) => {
                    return Ok(CallToolResult::error(vec![Content::text(err.to_string())]));
                }
            };
            match call_gate_tool(&tool_name, &arguments, current_root.as_ref()) {
                Ok(output) => {
                    if let Some(explicit_root) = explicit_root {
                        self.known_roots.register(explicit_root);
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

fn call_gate_tool(
    tool_name: &str,
    arguments: &Value,
    root: Option<&PathBuf>,
) -> Result<String, GateError> {
    let result = match tool_name {
        "status" => GateToolService::status(StatusArgs {
            context: context_from_value(&arguments, false, root)?,
            filter: StatusFilter {
                state: optional_string(arguments, "state")?,
                ready_only: optional_bool(arguments, "ready")?.unwrap_or(false),
                include_done: optional_bool(arguments, "includeDone")?.unwrap_or(false),
            },
        })?,
        "append" => GateToolService::append(AppendArgs {
            context: context_from_value(&arguments, false, root)?,
            task_id: required_string(&arguments, "taskId")?,
            agent: required_string(&arguments, "agent")?,
            notes: required_string(&arguments, "notes")?,
            model: optional_string(&arguments, "model")?,
        })?,
        "pass" => GateToolService::pass(OutcomeArgs {
            context: context_from_value(&arguments, false, root)?,
            task_id: required_string(&arguments, "taskId")?,
            agent: required_string(&arguments, "agent")?,
            notes: required_string(&arguments, "notes")?,
            model: optional_string(&arguments, "model")?,
        })?,
        "fail" => GateToolService::fail(OutcomeArgs {
            context: context_from_value(&arguments, false, root)?,
            task_id: required_string(&arguments, "taskId")?,
            agent: required_string(&arguments, "agent")?,
            notes: required_string(&arguments, "notes")?,
            model: optional_string(&arguments, "model")?,
        })?,
        "history" => GateToolService::history(HistoryArgs {
            context: context_from_value(&arguments, false, root)?,
            task_id: required_string(&arguments, "taskId")?,
        })?,
        "sync" => GateToolService::sync(SyncArgs {
            context: context_from_value(&arguments, false, root)?,
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

fn context_from_value(
    arguments: &Value,
    json_output: bool,
    default_root: Option<&PathBuf>,
) -> Result<GateContext, GateError> {
    let root = match optional_path(arguments, "root")? {
        Some(root) => root,
        None => default_root.cloned().ok_or_else(|| {
            GateError::Cli(
                "no root bound to this session; pass 'root' argument on any tool call to bind a workspace"
                    .to_string(),
            )
        })?,
    };

    Ok(GateContext {
        root,
        requested_pipeline_name: optional_string(arguments, "pipeline")?,
        json_output,
    })
}

fn optional_path(arguments: &Value, key: &str) -> Result<Option<PathBuf>, GateError> {
    Ok(optional_string(arguments, key)?.map(PathBuf::from))
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
    use tempfile::TempDir;

    use super::{
        GATE_SERVER_INSTRUCTIONS, GateMcpHandler, TransportKind, call_gate_tool, context_from_value,
    };

    #[test]
    fn mcp_status_tool_returns_task_array() {
        let root = new_custom_pipeline_root();
        let root_path = root.path().to_path_buf();
        call_gate_tool(
            "sync",
            &json!({ "root": root.path(), "rebuild": true }),
            Some(&root_path),
        )
        .expect("sync tool should succeed");
        let output = call_gate_tool("status", &json!({ "root": root.path() }), Some(&root_path))
            .expect("status tool should succeed");
        let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");
        let tasks = value.as_array().expect("status should return an array");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["taskId"], "T-9.1");
        assert_eq!(tasks[0]["state"], "pending");
    }

    #[test]
    fn mcp_status_tool_supports_ready_and_state_filters() {
        let root = new_custom_pipeline_root();
        let root_path = root.path().to_path_buf();
        fs::write(
            root.path().join("todo").join("T-9.2.md"),
            task_markdown("T-9.2", "custom"),
        )
        .expect("write second task");
        call_gate_tool(
            "sync",
            &json!({ "root": root.path(), "rebuild": true }),
            Some(&root_path),
        )
        .expect("sync tool should succeed");
        call_gate_tool(
            "pass",
            &json!({
                "root": root.path(),
                "taskId": "T-9.2",
                "agent": "swe",
                "notes": "advanced"
            }),
            Some(&root_path),
        )
        .expect("pass tool should succeed");

        let ready_output = call_gate_tool(
            "status",
            &json!({ "root": root.path(), "ready": true }),
            Some(&root_path),
        )
        .expect("ready status should succeed");
        let ready_value: serde_json::Value =
            serde_json::from_str(&ready_output).expect("valid ready JSON output");
        let ready_tasks = ready_value
            .as_array()
            .expect("ready status should return an array");
        assert_eq!(ready_tasks.len(), 1);
        assert_eq!(ready_tasks[0]["taskId"], "T-9.1");

        let swe_output = call_gate_tool(
            "status",
            &json!({ "root": root.path(), "state": "swe" }),
            Some(&root_path),
        )
        .expect("state status should succeed");
        let swe_value: serde_json::Value =
            serde_json::from_str(&swe_output).expect("valid state JSON output");
        let swe_tasks = swe_value
            .as_array()
            .expect("state status should return an array");
        assert_eq!(swe_tasks.len(), 1);
        assert_eq!(swe_tasks[0]["taskId"], "T-9.2");
    }

    #[test]
    fn mcp_status_excludes_done_by_default_and_restores_them_with_include_done() {
        let root = new_custom_pipeline_root();
        let root_path = root.path().to_path_buf();
        fs::write(
            root.path().join("todo").join("T-9.2.md"),
            task_markdown("T-9.2", "custom"),
        )
        .expect("write second task");
        call_gate_tool(
            "sync",
            &json!({ "root": root.path(), "rebuild": true }),
            Some(&root_path),
        )
        .expect("sync tool should succeed");
        call_gate_tool(
            "pass",
            &json!({
                "root": root.path(),
                "taskId": "T-9.2",
                "agent": "swe",
                "notes": "advanced to swe"
            }),
            Some(&root_path),
        )
        .expect("advance to swe");
        call_gate_tool(
            "pass",
            &json!({
                "root": root.path(),
                "taskId": "T-9.2",
                "agent": "red-team",
                "notes": "advanced to red-team"
            }),
            Some(&root_path),
        )
        .expect("advance to red-team");
        call_gate_tool(
            "pass",
            &json!({
                "root": root.path(),
                "taskId": "T-9.2",
                "agent": "red-team",
                "notes": "completed task"
            }),
            Some(&root_path),
        )
        .expect("complete task");

        let default_output =
            call_gate_tool("status", &json!({ "root": root.path() }), Some(&root_path))
                .expect("default status should succeed");
        let default_value: serde_json::Value =
            serde_json::from_str(&default_output).expect("valid default status json");
        let default_tasks = default_value
            .as_array()
            .expect("default status should return an array");
        assert_eq!(default_tasks.len(), 1);
        assert_eq!(default_tasks[0]["taskId"], "T-9.1");

        let all_output = call_gate_tool(
            "status",
            &json!({ "root": root.path(), "includeDone": true }),
            Some(&root_path),
        )
        .expect("includeDone status should succeed");
        let all_value: serde_json::Value =
            serde_json::from_str(&all_output).expect("valid includeDone status json");
        let all_tasks = all_value
            .as_array()
            .expect("includeDone status should return an array");
        assert_eq!(all_tasks.len(), 2);
        assert!(all_tasks.iter().any(|task| task["taskId"] == "T-9.2"));

        let done_output = call_gate_tool(
            "status",
            &json!({ "root": root.path(), "state": "done" }),
            Some(&root_path),
        )
        .expect("done state status should succeed");
        let done_value: serde_json::Value =
            serde_json::from_str(&done_output).expect("valid done status json");
        let done_tasks = done_value
            .as_array()
            .expect("done state status should return an array");
        assert_eq!(done_tasks.len(), 1);
        assert_eq!(done_tasks[0]["taskId"], "T-9.2");
    }

    #[test]
    fn mcp_pass_tool_advances_bound_task() {
        let root = new_custom_pipeline_root();
        let root_path = root.path().to_path_buf();
        let output = call_gate_tool(
            "pass",
            &json!({
                "root": root.path(),
                "taskId": "T-9.1",
                "agent": "swe",
                "notes": "swe completed"
            }),
            Some(&root_path),
        )
        .expect("pass tool should succeed");
        let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");
        assert_eq!(value["taskId"], "T-9.1");
        assert_eq!(value["state"], "swe");
    }

    #[test]
    fn mcp_history_tool_returns_stage_journey() {
        let root = new_custom_pipeline_root();
        let root_path = root.path().to_path_buf();

        call_gate_tool(
            "append",
            &json!({
                "root": root.path(),
                "taskId": "T-9.1",
                "agent": "swe",
                "notes": "started implementation",
                "model": "claude-sonnet-4.5"
            }),
            Some(&root_path),
        )
        .expect("append tool should succeed");
        call_gate_tool(
            "pass",
            &json!({
                "root": root.path(),
                "taskId": "T-9.1",
                "agent": "swe",
                "notes": "completed implementation"
            }),
            Some(&root_path),
        )
        .expect("pass tool should succeed");
        call_gate_tool(
            "fail",
            &json!({
                "root": root.path(),
                "taskId": "T-9.1",
                "agent": "red-team",
                "notes": "requested rework"
            }),
            Some(&root_path),
        )
        .expect("fail tool should succeed");

        let output = call_gate_tool(
            "history",
            &json!({ "root": root.path(), "taskId": "T-9.1" }),
            Some(&root_path),
        )
        .expect("history tool should succeed");
        let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");
        let history = value.as_array().expect("history should return an array");
        assert_eq!(history.len(), 3);
        assert_eq!(history[0]["stage"], "pending");
        assert_eq!(history[0]["event"], "append");
        assert_eq!(history[0]["agent"], "swe");
        assert_eq!(history[0]["model"], "claude-sonnet-4.5");
        assert_eq!(history[0]["notes"], "started implementation");
        assert!(history[0]["timestamp"].as_str().is_some());
        assert_eq!(history[1]["event"], "PASS");
        assert_eq!(history[1]["notes"], "completed implementation");
        assert_eq!(history[2]["stage"], "swe");
        assert_eq!(history[2]["event"], "FAIL");
        assert_eq!(history[2]["agent"], "red-team");
        assert_eq!(history[2]["notes"], "requested rework");
    }

    #[test]
    fn context_requires_explicit_root_when_service_default_is_unset() {
        let err = context_from_value(&json!({}), false, None).expect_err("missing root must fail");
        assert!(err.to_string().contains("no root bound to this session"));
    }

    #[test]
    fn server_instructions_explain_authoritative_gate_workflow() {
        let handler = GateMcpHandler::new(TransportKind::Stdio, KnownRoots::default())
            .expect("handler should build");
        let info = <GateMcpHandler as rmcp::ServerHandler>::get_info(&handler);
        let instructions = info
            .instructions
            .expect("server instructions should be present");

        assert_eq!(instructions, GATE_SERVER_INSTRUCTIONS);
        assert!(instructions.contains("authoritative project task tracker"));
        assert!(instructions.contains("prefer gate instead of ad-hoc todo lists"));
        assert!(instructions.contains("replace SQL todo-table tracking with gate"));
        assert!(instructions.contains("run sync"));
        assert!(instructions.contains("run status"));
        assert!(instructions.contains("use append"));
        assert!(instructions.contains("pass to move the task forward"));
        assert!(instructions.contains("fail to send it back"));
        assert!(instructions.contains("default SDD workflow"));
        assert!(instructions.contains("stage and role transitions"));
        assert!(instructions.contains("pending -> swe -> red-team -> blue-team -> done"));
        assert!(instructions.contains("establishes and stores"));
        assert!(instructions.contains("reuses that stored root"));
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
                "swe": { "on": { "fail": "pending", "pass": "red-team" } },
                "red-team": { "on": { "fail": "pending", "pass": "done" } },
                "done": { "terminal": true }
            }
        });
        let pipeline_text =
            serde_json::to_string_pretty(&pipeline_json).expect("serialize pipeline json");
        fs::write(root.join("pipeline.json"), &pipeline_text).expect("write pipeline");

        fs::write(
            root.join("todo").join("T-9.1.md"),
            task_markdown("T-9.1", "custom"),
        )
        .expect("write task");
        temp
    }

    fn task_markdown(task_id: &str, pipeline: &str) -> String {
        format!(
            "# {task_id} — MCP Test\n\n**Pipeline**: {pipeline}\n\n## Acceptance Criteria\n\n1. tool call succeeds\n"
        )
    }

    #[allow(dead_code)]
    fn _assert_exists(path: &Path) {
        assert!(path.exists(), "missing path {}", path.display());
    }
}
