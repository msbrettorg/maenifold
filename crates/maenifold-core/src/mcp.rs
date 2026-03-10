use crate::assets::{AssetKind, AssetManager, is_watchable_asset_json_path};
use crate::config::Config;
use crate::db::GraphDatabase;
use crate::error::{MaenifoldError, Result};
use crate::json::OutputFormat;
use crate::registry::ToolRegistry;
use notify::{Config as NotifyConfig, Event, EventKind, RecursiveMode, Watcher};
use rmcp::model::{
    AnnotateAble, CallToolRequestParams, CallToolResult, Content, Implementation,
    ListResourcesResult, ListToolsResult, RawResource, ReadResourceRequestParams,
    ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::{Peer, RoleServer, ServerHandler, ServiceExt};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct MaenifoldMcpServer;

// T-5.1: RTM FR-22.1 — rmcp-based async MCP handler.
struct MaenifoldHandler {
    tools: Vec<Tool>,
}

struct AssetWatcherState {
    watcher: notify::RecommendedWatcher,
    control_tx: Sender<AssetWatcherSignal>,
    worker: Option<JoinHandle<()>>,
}

impl AssetWatcherState {
    fn stop(mut self) {
        let _ = self.control_tx.send(AssetWatcherSignal::Stop);
        drop(self.watcher);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

enum AssetWatcherSignal {
    Event(Event),
    Stop,
}

impl MaenifoldHandler {
    fn new() -> Result<Self> {
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
        Ok(Self { tools })
    }
}

impl ServerHandler for MaenifoldHandler {
    fn get_info(&self) -> ServerInfo {
        // T-10.2: RTM FR-11 / RTM-068 — MCP initialize advertises native resources support.
        // T-5.1: RTM FR-22.1 — server info returned during MCP initialize handshake.
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                // T-10.3: RTM FR-11 / RTM-071 — advertise MCP resource list change
                // notifications so clients know to expect notifications/resources/list_changed.
                .enable_resources_list_changed()
                .build(),
        )
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
        std::future::ready(
            current_asset_manager()
                .and_then(list_asset_resources)
                .map(|resources| ListResourcesResult {
                    resources,
                    next_cursor: None,
                    meta: None,
                })
                .map_err(map_resource_error),
        )
    }

    // T-10.2: RTM FR-11 / RTM-070 — resources/read resolves asset://{type}/{id} URIs to JSON payloads.
    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        std::future::ready(
            current_asset_manager()
                .and_then(|assets| assets.read_resource(&request.uri))
                .and_then(|value| serde_json::to_string_pretty(&value).map_err(Into::into))
                .map(|text| {
                    ReadResourceResult::new(vec![
                        ResourceContents::text(text, request.uri)
                            .with_mime_type("application/json"),
                    ])
                })
                .map_err(map_resource_error),
        )
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

        std::future::ready(
            match ToolRegistry::global().try_invoke(&tool_name, &arguments, OutputFormat::Json) {
                Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
                // T-11.6: RTM FR-11 preserves raw validation diagnostics while prefixing internal failures.
                Err(err) => Ok(CallToolResult::error(vec![Content::text(
                    format_tool_error(&tool_name, &err),
                )])),
            },
        )
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
        let handler = MaenifoldHandler::new()?;
        let transport = rmcp::transport::io::stdio();
        let service = handler
            .serve(transport)
            .await
            .map_err(|e| MaenifoldError::Message(format!("MCP server init failed: {e}")))?;
        // T-10.3: RTM FR-11 / RTM-071 — start the asset watcher after the MCP handshake
        // so changed asset files emit notifications/resources/list_changed to the client.
        let asset_watcher = start_asset_watcher(&config, service.peer().clone())?;
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

impl McpToolDefinition {
    fn new(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        }
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

fn current_asset_manager() -> Result<AssetManager> {
    let config = Config::current();
    initialize_runtime(&config)?;
    Ok(AssetManager::new(&config))
}

fn start_asset_watcher(config: &Config, peer: Peer<RoleServer>) -> Result<AssetWatcherState> {
    let assets_path = config.assets_path.clone();
    let debounce_ms = 150_u64;
    let runtime = tokio::runtime::Handle::current();
    let (event_tx, event_rx) = mpsc::channel();
    let worker =
        spawn_asset_watcher_worker(assets_path.clone(), debounce_ms, peer, runtime, event_rx);
    let callback_tx = event_tx.clone();

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        let Ok(event) = event else {
            return;
        };
        if !is_relevant_asset_event_kind(&event.kind) {
            return;
        }
        let _ = callback_tx.send(AssetWatcherSignal::Event(event));
    })
    .map_err(map_notify_error)?;

    let _ = watcher.configure(
        NotifyConfig::default()
            .with_poll_interval(Duration::from_millis(50))
            .with_compare_contents(true),
    );

    if let Err(error) = watcher.watch(&assets_path, RecursiveMode::Recursive) {
        let _ = event_tx.send(AssetWatcherSignal::Stop);
        let _ = worker.join();
        return Err(map_notify_error(error));
    }

    Ok(AssetWatcherState {
        watcher,
        control_tx: event_tx,
        worker: Some(worker),
    })
}

fn spawn_asset_watcher_worker(
    assets_path: std::path::PathBuf,
    debounce_ms: u64,
    peer: Peer<RoleServer>,
    runtime: tokio::runtime::Handle,
    event_rx: Receiver<AssetWatcherSignal>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let debounce_window = Duration::from_millis(debounce_ms.max(1));
        let flush_grace_window = debounce_grace_window(debounce_window);
        let mut pending_paths = HashSet::new();

        loop {
            match event_rx.recv() {
                Ok(AssetWatcherSignal::Event(event)) => {
                    collect_pending_asset_paths(&assets_path, &event, &mut pending_paths);
                }
                Ok(AssetWatcherSignal::Stop) | Err(_) => {
                    flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                    break;
                }
            }

            loop {
                match event_rx.recv_timeout(debounce_window) {
                    Ok(AssetWatcherSignal::Event(event)) => {
                        collect_pending_asset_paths(&assets_path, &event, &mut pending_paths);
                    }
                    Ok(AssetWatcherSignal::Stop) => {
                        flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => match event_rx
                        .recv_timeout(flush_grace_window)
                    {
                        Ok(AssetWatcherSignal::Event(event)) => {
                            collect_pending_asset_paths(&assets_path, &event, &mut pending_paths);
                        }
                        Ok(AssetWatcherSignal::Stop) => {
                            flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                            return;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                            break;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                            return;
                        }
                    },
                    Err(RecvTimeoutError::Disconnected) => {
                        flush_asset_notifications(&runtime, &peer, &mut pending_paths);
                        return;
                    }
                }
            }
        }
    })
}

fn debounce_grace_window(debounce_window: Duration) -> Duration {
    debounce_window.min(Duration::from_millis(50))
}

fn collect_pending_asset_paths(
    assets_path: &std::path::Path,
    event: &Event,
    pending_paths: &mut HashSet<std::path::PathBuf>,
) {
    for path in &event.paths {
        if is_watchable_asset_json_path(assets_path, path) {
            pending_paths.insert(path.clone());
        }
    }
}

fn flush_asset_notifications(
    runtime: &tokio::runtime::Handle,
    peer: &Peer<RoleServer>,
    pending_paths: &mut HashSet<std::path::PathBuf>,
) {
    if pending_paths.is_empty() {
        return;
    }

    pending_paths.clear();
    let notification_peer = peer.clone();
    let _ = runtime.block_on(async move {
        let _ = notification_peer.notify_resource_list_changed().await;
    });
}

fn is_relevant_asset_event_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn list_asset_resources(asset_manager: AssetManager) -> Result<Vec<Resource>> {
    let mut resources = Vec::new();
    for kind in AssetKind::all() {
        let metadata = asset_manager.list_assets(Some(kind))?;
        let Value::Array(items) = metadata else {
            continue;
        };
        for item in items {
            if let Some(resource) = resource_from_metadata(kind, item) {
                resources.push(resource);
            }
        }
    }
    Ok(resources)
}

fn resource_from_metadata(kind: AssetKind, metadata: Value) -> Option<Resource> {
    let object = metadata.as_object()?;
    let id = object.get("id")?.as_str()?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(id)
        .to_string();
    let uri = asset_uri(kind, id);
    let mut resource = RawResource::new(uri, name)
        .with_mime_type("application/json")
        .optional_annotate(None);
    if let Some(title) = object.get("name").and_then(Value::as_str) {
        resource.title = Some(title.to_string());
    }
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        resource.description = Some(description.to_string());
    }
    Some(resource)
}

fn asset_uri(kind: AssetKind, id: &str) -> String {
    format!("asset://{}/{id}", kind.dir_name())
}

fn map_resource_error(err: MaenifoldError) -> rmcp::ErrorData {
    match err {
        // T-10.2: RTM FR-11 / RTM-070 — missing assets map to MCP resource-not-found.
        MaenifoldError::AssetNotFound(kind, id) => {
            rmcp::ErrorData::resource_not_found(format!("Asset not found: {kind}/{id}"), None)
        }
        // T-10.2: RTM FR-11 / RTM-069 / RTM-070 — malformed URIs stay invalid-params errors.
        MaenifoldError::UnknownTool(_)
        | MaenifoldError::InvalidPayload(_)
        | MaenifoldError::InvalidArgument(_)
        | MaenifoldError::InvalidResourceUri(_)
        | MaenifoldError::PathTraversal(_)
        | MaenifoldError::MissingProperty(_)
        | MaenifoldError::Message(_) => rmcp::ErrorData::invalid_params(err.to_string(), None),
        _ => rmcp::ErrorData::internal_error(err.to_string(), None),
    }
}

fn map_notify_error(error: notify::Error) -> MaenifoldError {
    MaenifoldError::Message(format!("Asset watcher failed: {error}"))
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
            | MaenifoldError::Message(_)
    )
}

fn mcp_tool_from_registry_name(registry_name: &str) -> Result<McpToolDefinition> {
    let tool = match registry_name {
        "WriteMemory" => McpToolDefinition::new(
            registry_name,
            "Creates new knowledge files with [[WikiLinks]] for graph integration.",
            object_schema(
                vec![
                    ("title", string_property("Title for this memory file")),
                    (
                        "content",
                        string_property(
                            "Content with [[Concept Names]]; at least one [[concept]] is required",
                        ),
                    ),
                    (
                        "folder",
                        string_property("Optional folder path for organizing files"),
                    ),
                    (
                        "tags",
                        string_array_property("Optional tags for categorizing this file"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["title", "content"],
            ),
        ),
        "ReadMemory" => McpToolDefinition::new(
            registry_name,
            "Retrieves an existing knowledge file by URI or title.",
            object_schema(
                vec![
                    (
                        "identifier",
                        string_property("Memory file identifier: URI or exact title"),
                    ),
                    (
                        "includeChecksum",
                        bool_property("Return checksum with content for safe editing", true),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["identifier"],
            ),
        ),
        "EditMemory" => McpToolDefinition::new(
            registry_name,
            "Modifies an existing knowledge file while preserving WikiLinks.",
            object_schema(
                vec![
                    (
                        "identifier",
                        string_property("Memory file identifier: URI or exact title"),
                    ),
                    (
                        "operation",
                        enum_string_property(
                            "Edit operation to apply",
                            &["append", "prepend", "find_replace", "replace_section"],
                        )
                        .into(),
                    ),
                    (
                        "content",
                        string_property(
                            "Content to add or replace; must include at least one [[concept]]",
                        ),
                    ),
                    (
                        "checksum",
                        string_property(
                            "File checksum from the last read for stale-edit protection",
                        ),
                    ),
                    (
                        "findText",
                        string_property("Text to find when operation is find_replace"),
                    ),
                    (
                        "sectionName",
                        string_property("Section name when operation is replace_section"),
                    ),
                    (
                        "expectedCount",
                        integer_property("Expected match count when operation is find_replace"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["identifier", "operation", "content"],
            ),
        ),
        "DeleteMemory" => McpToolDefinition::new(
            registry_name,
            "Deletes an existing knowledge file after explicit confirmation.",
            object_schema(
                vec![
                    (
                        "identifier",
                        string_property("Memory file identifier: URI or exact title"),
                    ),
                    (
                        "confirm",
                        bool_property("Confirm deletion of the file", false),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["identifier"],
            ),
        ),
        "MoveMemory" => McpToolDefinition::new(
            registry_name,
            "Moves or renames a knowledge file while preserving WikiLinks.",
            object_schema(
                vec![
                    (
                        "source",
                        string_property("Source memory file identifier: URI or exact title"),
                    ),
                    (
                        "destination",
                        string_property("Destination file name or relative path"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["source", "destination"],
            ),
        ),
        "SearchMemories" => McpToolDefinition::new(
            registry_name,
            "Searches knowledge files using hybrid, semantic, or full-text ranking.",
            object_schema(
                vec![
                    ("query", string_property("Search query over file contents")),
                    (
                        "mode",
                        enum_string_property("Search mode", &["Hybrid", "Semantic", "FullText"])
                            .with_default(json!("Hybrid")),
                    ),
                    (
                        "pageSize",
                        integer_property_with_default("Maximum files to return", json!(10)),
                    ),
                    (
                        "page",
                        integer_property_with_default("Page number for results", json!(1)),
                    ),
                    (
                        "folder",
                        string_property("Optional folder path filter under the memory root"),
                    ),
                    (
                        "tags",
                        string_array_property("Optional tag filter; all provided tags must match"),
                    ),
                    (
                        "minScore",
                        number_property_with_default("Minimum score threshold", json!(0.0)),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["query"],
            ),
        ),
        "ExtractConceptsFromFile" => McpToolDefinition::new(
            registry_name,
            "Extracts WikiLink concepts from a specific knowledge file.",
            object_schema(
                vec![
                    (
                        "identifier",
                        string_property("Memory file identifier: URI or exact title"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["identifier"],
            ),
        ),
        "Sync" => McpToolDefinition::new(
            registry_name,
            "Synchronizes WikiLink concepts from memory files into the graph database.",
            object_schema(
                vec![(
                    "learn",
                    bool_property("Return help documentation instead of executing", false),
                )],
                &[],
            ),
        ),
        "BuildContext" => McpToolDefinition::new(
            registry_name,
            "Builds graph context around a concept by traversing related knowledge.",
            object_schema(
                vec![
                    (
                        "conceptName",
                        string_property("Concept name to build context around"),
                    ),
                    (
                        "depth",
                        integer_property_with_default("How many graph hops to traverse", json!(2)),
                    ),
                    (
                        "maxEntities",
                        integer_property_with_default(
                            "Maximum related entities to return",
                            json!(20),
                        ),
                    ),
                    (
                        "includeContent",
                        bool_property("Include content previews in the context response", false),
                    ),
                ],
                &["conceptName"],
            ),
        ),
        "Visualize" => McpToolDefinition::new(
            registry_name,
            "Generates a Mermaid diagram for concept relationships.",
            object_schema(
                vec![
                    (
                        "conceptName",
                        string_property("Central concept for the visualization"),
                    ),
                    (
                        "depth",
                        integer_property_with_default("Graph traversal depth", json!(2)),
                    ),
                    (
                        "maxNodes",
                        integer_property_with_default(
                            "Maximum nodes to include in the diagram",
                            json!(30),
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["conceptName"],
            ),
        ),
        "SequentialThinking" => McpToolDefinition::new(
            registry_name,
            "Creates structured thinking sessions with persistent branching and conclusions.",
            object_schema(
                vec![
                    (
                        "response",
                        string_property(
                            "Main thought content; must include [[WikiLinks]] unless cancelling",
                        ),
                    ),
                    (
                        "nextThoughtNeeded",
                        bool_property("Whether another thought is needed after this one", false),
                    ),
                    (
                        "thoughtNumber",
                        integer_property_with_default("Current thought number", json!(0)),
                    ),
                    (
                        "totalThoughts",
                        integer_property_with_default(
                            "Current estimate for total thoughts",
                            json!(0),
                        ),
                    ),
                    (
                        "sessionId",
                        string_property("Existing sequential thinking session identifier"),
                    ),
                    (
                        "cancel",
                        bool_property("Cancel the session instead of adding a thought", false),
                    ),
                    (
                        "thoughts",
                        string_property(
                            "Optional meta-thoughts; include [[WikiLinks]] when provided",
                        ),
                    ),
                    (
                        "isRevision",
                        bool_property("Marks this thought as a revision of prior thinking", false),
                    ),
                    (
                        "revisesThought",
                        integer_property("Thought number that this revision updates"),
                    ),
                    (
                        "branchFromThought",
                        integer_property("Thought number where this branch diverges"),
                    ),
                    (
                        "branchId",
                        string_property("Branch identifier for multi-agent coordination"),
                    ),
                    (
                        "needsMoreThoughts",
                        bool_property(
                            "Whether more thoughts are needed than originally estimated",
                            false,
                        ),
                    ),
                    (
                        "analysisType",
                        string_property(
                            "Optional analysis type annotation such as bug or architecture",
                        ),
                    ),
                    (
                        "parentWorkflowId",
                        string_property("Optional parent workflow session identifier"),
                    ),
                    (
                        "conclusion",
                        string_property(
                            "Structured ConfessionReport used when completing the session",
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "Workflow" => McpToolDefinition::new(
            registry_name,
            "Runs structured workflows with optional sequential-thinking submachines.",
            object_schema(
                vec![
                    (
                        "sessionId",
                        string_property("Existing workflow session identifier to continue"),
                    ),
                    (
                        "workflowId",
                        string_or_string_array_property(
                            "Workflow identifier or list of workflow identifiers to start",
                        ),
                    ),
                    (
                        "response",
                        string_property("Response to the current step; include [[WikiLinks]]"),
                    ),
                    (
                        "thoughts",
                        string_property("Ambient or meta-thoughts; include [[WikiLinks]]"),
                    ),
                    (
                        "status",
                        enum_string_property(
                            "Terminal status for the workflow session",
                            &["completed", "cancelled"],
                        )
                        .into(),
                    ),
                    (
                        "conclusion",
                        string_property(
                            "Structured ConfessionReport used when completing the session",
                        ),
                    ),
                    (
                        "view",
                        bool_property("Display the workflow queue status", false),
                    ),
                    (
                        "append",
                        string_property(
                            "Workflow identifier or identifiers to append to the queue",
                        ),
                    ),
                    (
                        "submachineSessionId",
                        string_property(
                            "SequentialThinking session identifier registered as a submachine",
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "RecentActivity" => McpToolDefinition::new(
            registry_name,
            "Reports recently indexed files with optional content excerpts.",
            object_schema(
                vec![
                    (
                        "limit",
                        integer_property_with_default(
                            "Maximum number of results to return",
                            json!(10),
                        ),
                    ),
                    (
                        "filter",
                        enum_string_property(
                            "Activity filter",
                            &["thinking", "memory", "assumptions", "all"],
                        )
                        .with_default(json!("all")),
                    ),
                    (
                        "timespan",
                        string_property_with_pattern(
                            "Positive timespan such as 24.00:00:00",
                            r"^(\d+\.)?\d{2}:\d{2}:\d{2}(\.\d{1,7})?$",
                        ),
                    ),
                    (
                        "includeContent",
                        bool_property(
                            "Include full H2 section content instead of headers only",
                            false,
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "MemoryStatus" => McpToolDefinition::new(
            registry_name,
            "Provides high-level memory system statistics and health information.",
            object_schema(
                vec![(
                    "learn",
                    bool_property("Return help documentation instead of executing", false),
                )],
                &[],
            ),
        ),
        "GetConfig" => McpToolDefinition::new(
            registry_name,
            "Displays maenifold configuration settings and resolved paths.",
            object_schema(
                vec![(
                    "learn",
                    bool_property("Return help documentation instead of executing", false),
                )],
                &[],
            ),
        ),
        "GetHelp" => McpToolDefinition::new(
            registry_name,
            "Retrieves detailed help documentation for a specific tool.",
            object_schema(
                vec![
                    ("toolName", string_property("Tool name to load help for")),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["toolName"],
            ),
        ),
        "ListMemories" => McpToolDefinition::new(
            registry_name,
            "Lists knowledge files or folders relative to the memory root.",
            object_schema(
                vec![
                    (
                        "path",
                        string_property("Optional directory path relative to the memory root"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "UpdateAssets" => McpToolDefinition::new(
            registry_name,
            "Refreshes runtime assets from packaged assets with optional dry-run mode.",
            object_schema(
                vec![
                    (
                        "dryRun",
                        bool_property("Preview changes without modifying files", true),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "RepairConcepts" => McpToolDefinition::new(
            registry_name,
            "Repairs concept variants across memory files by replacing them with a canonical form.",
            object_schema(
                vec![
                    (
                        "conceptsToReplace",
                        string_property("Delimited concept variants to replace"),
                    ),
                    (
                        "canonicalConcept",
                        string_property("Canonical concept to keep after consolidation"),
                    ),
                    (
                        "folder",
                        string_property("Optional folder under the memory root to process"),
                    ),
                    (
                        "dryRun",
                        bool_property("Preview changes without modifying files", true),
                    ),
                    (
                        "createWikiLinks",
                        bool_property(
                            "Create WikiLinks from plain text instead of replacing existing ones",
                            false,
                        ),
                    ),
                    (
                        "minSemanticSimilarity",
                        number_property_with_default(
                            "Minimum semantic similarity required for safe consolidation",
                            json!(0.7),
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["conceptsToReplace", "canonicalConcept"],
            ),
        ),
        "AnalyzeConceptCorruption" => McpToolDefinition::new(
            registry_name,
            "Analyzes concept families to find duplicate or corrupted WikiLink variants.",
            object_schema(
                vec![
                    (
                        "conceptFamily",
                        string_property("Concept family to analyze, such as tool or agent"),
                    ),
                    (
                        "maxResults",
                        integer_property_with_default("Maximum variants to include", json!(50)),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["conceptFamily"],
            ),
        ),
        "FindSimilarConcepts" => McpToolDefinition::new(
            registry_name,
            "Finds semantically similar concepts using vector embeddings.",
            object_schema(
                vec![
                    (
                        "conceptName",
                        string_property("Concept name to search for related concepts"),
                    ),
                    (
                        "maxResults",
                        integer_property_with_default(
                            "Maximum number of similar concepts to return",
                            json!(10),
                        ),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["conceptName"],
            ),
        ),
        "StartWatcher" => McpToolDefinition::new(
            registry_name,
            "Starts background watching of the memory directory for incremental sync.",
            object_schema(
                vec![(
                    "debounceMs",
                    integer_property_with_default("Debounce interval in milliseconds", json!(150)),
                )],
                &[],
            ),
        ),
        "StopWatcher" => McpToolDefinition::new(
            registry_name,
            "Stops the background watcher used for incremental sync.",
            object_schema(vec![], &[]),
        ),
        "RunFullBenchmark" => McpToolDefinition::new(
            registry_name,
            "Runs system performance benchmarks across search, sync, and graph traversal.",
            object_schema(
                vec![
                    (
                        "iterations",
                        integer_property_with_default(
                            "Benchmark iterations per category",
                            json!(5),
                        ),
                    ),
                    (
                        "maxTestFiles",
                        integer_property_with_default(
                            "Maximum test files to use during benchmarks",
                            json!(1000),
                        ),
                    ),
                    (
                        "includeDeepTraversal",
                        bool_property("Include expensive deep traversal benchmarks", true),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "Adopt" => McpToolDefinition::new(
            registry_name,
            "Loads a role, color, or perspective asset as an adoption instruction.",
            object_schema(
                vec![
                    (
                        "type",
                        enum_string_property(
                            "Asset type to adopt",
                            &["role", "color", "perspective"],
                        )
                        .into(),
                    ),
                    (
                        "identifier",
                        string_property("Asset identifier or filename without the .json extension"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["type", "identifier"],
            ),
        ),
        "AssumptionLedger" => McpToolDefinition::new(
            registry_name,
            "Creates, updates, or reads assumptions linked to workflow or thinking context.",
            object_schema(
                vec![
                    (
                        "action",
                        enum_string_property(
                            "Ledger action to perform",
                            &["append", "update", "read"],
                        )
                        .into(),
                    ),
                    (
                        "assumption",
                        string_property("Assumption statement for append operations"),
                    ),
                    (
                        "context",
                        string_property("Workflow or thinking session reference"),
                    ),
                    (
                        "validationPlan",
                        string_property("Planned validation method"),
                    ),
                    (
                        "confidence",
                        string_property("Confidence level such as high, medium, or low"),
                    ),
                    (
                        "concepts",
                        string_array_property("Concept tags for knowledge graph integration"),
                    ),
                    (
                        "uri",
                        string_property("Memory URI for update or read actions"),
                    ),
                    (
                        "status",
                        enum_string_property(
                            "Updated assumption status",
                            &["active", "validated", "invalidated", "refined"],
                        )
                        .into(),
                    ),
                    (
                        "notes",
                        string_property("Additional notes for update operations"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["action"],
            ),
        ),
        "ListAssets" => McpToolDefinition::new(
            registry_name,
            "Lists available asset types or metadata for a specific asset type.",
            object_schema(
                vec![
                    (
                        "type",
                        enum_string_property(
                            "Optional asset type to list",
                            &["workflow", "role", "color", "perspective"],
                        )
                        .into(),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &[],
            ),
        ),
        "ReadMcpResource" => McpToolDefinition::new(
            registry_name,
            "Reads MCP resource content by URI.",
            object_schema(
                vec![
                    (
                        "uri",
                        string_property("Resource URI such as asset://catalog"),
                    ),
                    (
                        "learn",
                        bool_property("Return help documentation instead of executing", false),
                    ),
                ],
                &["uri"],
            ),
        ),
        other => {
            return Err(MaenifoldError::Message(format!(
                "Missing MCP tool schema mapping for registry tool {other}"
            )));
        }
    };

    Ok(tool)
}

fn object_schema(properties: Vec<(&str, Value)>, required: &[&str]) -> Value {
    let mut property_map = Map::new();
    for (name, schema) in properties {
        property_map.insert(name.to_string(), schema);
    }

    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), Value::Object(property_map));
    schema.insert("additionalProperties".to_string(), Value::Bool(false));
    if !required.is_empty() {
        schema.insert(
            "required".to_string(),
            Value::Array(required.iter().map(|name| json!(name)).collect()),
        );
    }
    Value::Object(schema)
}

fn string_property(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn string_property_with_pattern(description: &str, pattern: &str) -> Value {
    json!({ "type": "string", "description": description, "pattern": pattern })
}

fn integer_property(description: &str) -> Value {
    json!({ "type": "integer", "description": description })
}

fn integer_property_with_default(description: &str, default: Value) -> Value {
    json!({ "type": "integer", "description": description, "default": default })
}

fn number_property_with_default(description: &str, default: Value) -> Value {
    json!({ "type": "number", "description": description, "default": default })
}

fn bool_property(description: &str, default: bool) -> Value {
    json!({ "type": "boolean", "description": description, "default": default })
}

fn string_array_property(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": { "type": "string" }
    })
}

fn string_or_string_array_property(description: &str) -> Value {
    json!({
        "description": description,
        "anyOf": [
            { "type": "string" },
            {
                "type": "array",
                "items": { "type": "string" }
            }
        ]
    })
}

fn enum_string_property(description: &str, variants: &[&str]) -> SchemaProperty {
    SchemaProperty(json!({
        "type": "string",
        "description": description,
        "enum": variants,
    }))
}

struct SchemaProperty(Value);

impl SchemaProperty {
    fn with_default(mut self, default: Value) -> Value {
        if let Value::Object(ref mut object) = self.0 {
            object.insert("default".to_string(), default);
        }
        self.0
    }
}

impl From<SchemaProperty> for Value {
    fn from(property: SchemaProperty) -> Self {
        property.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::collections::BTreeSet;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    struct TestRoot {
        _guard: std::sync::MutexGuard<'static, ()>,
        _tmp: TempDir,
    }

    impl TestRoot {
        fn new() -> Self {
            let guard = TEST_LOCK.lock().expect("test lock");
            let tmp = TempDir::new().expect("temp dir");
            Config::override_root(tmp.path());
            initialize_runtime(&Config::current()).expect("runtime init should succeed");
            Self {
                _guard: guard,
                _tmp: tmp,
            }
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

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
        let handler = MaenifoldHandler::new().expect("handler should build");
        let info = handler.get_info();

        assert_eq!(info.server_info.name, "maenifold");
        assert!(!info.server_info.version.is_empty());
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
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
        let handler = MaenifoldHandler::new().expect("handler should build");
        assert_eq!(handler.tools.len(), 28);

        let names: BTreeSet<&str> = handler.tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains("WriteMemory"));
        assert!(names.contains("SequentialThinking"));
        assert!(names.contains("ReadMcpResource"));
    }

    // T-10.2: RTM FR-11 / RTM-069 — resources/list enumerates URIs for all supported asset families.
    #[test]
    fn t_10_2_list_asset_resources_covers_all_asset_types() {
        let _root = TestRoot::new();

        let resources = list_asset_resources(current_asset_manager().expect("asset manager"))
            .expect("resources should list");

        assert!(
            resources
                .iter()
                .any(|r| r.uri.starts_with("asset://workflows/"))
        );
        assert!(
            resources
                .iter()
                .any(|r| r.uri.starts_with("asset://roles/"))
        );
        assert!(
            resources
                .iter()
                .any(|r| r.uri.starts_with("asset://colors/"))
        );
        assert!(
            resources
                .iter()
                .any(|r| r.uri.starts_with("asset://perspectives/"))
        );
    }

    // T-10.2: RTM FR-11 / RTM-070 — resources/read returns the JSON body for a valid asset URI.
    #[test]
    fn t_10_2_current_asset_manager_reads_asset_uri() {
        let _root = TestRoot::new();

        let resource = current_asset_manager()
            .expect("asset manager")
            .read_resource("asset://workflows/deductive-reasoning")
            .expect("resource should read");

        assert_eq!(
            resource.get("id").and_then(Value::as_str),
            Some("deductive-reasoning")
        );
    }
}
