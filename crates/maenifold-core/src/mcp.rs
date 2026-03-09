use crate::assets::AssetManager;
use crate::config::Config;
use crate::db::GraphDatabase;
use crate::error::{MaenifoldError, Result};
use crate::json::OutputFormat;
use crate::registry::ToolRegistry;
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::io::{self, BufRead, BufReader, Read, Write};

pub struct MaenifoldMcpServer;

// T-5.8: RTM NFR-3, NFR-4 hardens stdio framing against oversized MCP requests.
const MAX_MCP_FRAME_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug)]
enum StdioReadResult {
    Eof,
    Request(Value),
    ProtocolError { code: i64, message: String },
}

impl MaenifoldMcpServer {
    pub fn list_tools() -> Result<Vec<McpToolDefinition>> {
        list_mcp_tools(ToolRegistry::global())
    }

    // T-5.1: RTM FR-22.1
    pub fn serve_stdio() -> Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = stdout.lock();

        loop {
            let request = match read_stdio_message(&mut reader)? {
                StdioReadResult::Eof => break,
                StdioReadResult::ProtocolError { code, message } => {
                    write_stdio_message(&mut writer, &error_response(Value::Null, code, message))?;
                    writer.flush()?;
                    continue;
                }
                StdioReadResult::Request(request) => request,
            };

            let request_id = request.get("id").cloned().unwrap_or(Value::Null);
            match handle_stdio_request(request) {
                Ok(Some(response)) => {
                    write_stdio_message(&mut writer, &response)?;
                    writer.flush()?;
                }
                Ok(None) => {}
                Err(error) => {
                    // T-5.8: RTM NFR-3, NFR-4 keeps the MCP server alive on malformed requests.
                    let response = json_rpc_error_for_request_error(request_id, error);
                    write_stdio_message(&mut writer, &response)?;
                    writer.flush()?;
                }
            }
        }

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
                    integer_property_with_default("Debounce interval in milliseconds", json!(500)),
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

fn read_stdio_message(reader: &mut BufReader<impl Read>) -> Result<StdioReadResult> {
    let mut content_length = None;
    let mut header_error = None;

    loop {
        let mut header_line = String::new();
        match reader.read_line(&mut header_line)? {
            0 if content_length.is_none() && header_error.is_none() => {
                return Ok(StdioReadResult::Eof);
            }
            0 => {
                return Err(MaenifoldError::Message(
                    "Unexpected EOF while reading MCP headers".to_string(),
                ));
            }
            _ => {}
        }

        let trimmed = header_line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            match value.trim().parse::<usize>() {
                Ok(length) => content_length = Some(length),
                Err(err) => {
                    header_error = Some(format!("Invalid Content-Length header: {err}"));
                }
            }
        }
    }

    if let Some(message) = header_error {
        return Ok(StdioReadResult::ProtocolError {
            code: -32600,
            message,
        });
    }

    let Some(content_length) = content_length else {
        return Ok(StdioReadResult::ProtocolError {
            code: -32600,
            message: "Missing Content-Length header".to_string(),
        });
    };

    if content_length > MAX_MCP_FRAME_SIZE {
        discard_stdio_body(reader, content_length)?;
        return Ok(StdioReadResult::ProtocolError {
            code: -32600,
            message: format!(
                "MCP frame exceeds max size of {MAX_MCP_FRAME_SIZE} bytes: {content_length}"
            ),
        });
    }

    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body)?;
    match serde_json::from_slice(&body) {
        Ok(request) => Ok(StdioReadResult::Request(request)),
        Err(err) => Ok(StdioReadResult::ProtocolError {
            code: -32700,
            message: format!("Parse error: {err}"),
        }),
    }
}

fn write_stdio_message(writer: &mut impl Write, response: &Value) -> Result<()> {
    let body = serde_json::to_vec(response)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    Ok(())
}

fn handle_stdio_request(request: Value) -> Result<Option<Value>> {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| MaenifoldError::InvalidPayload("Missing JSON-RPC method".to_string()))?;
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    let response = match method {
        "initialize" => Some(success_response(id, initialize_result())),
        "notifications/initialized" => None,
        "tools/list" => Some(success_response(
            id,
            json!({
                "tools": MaenifoldMcpServer::list_tools()?,
            }),
        )),
        "tools/call" => Some(success_response(id, call_tool(params))),
        _ => Some(error_response(
            id,
            -32601,
            format!("Method not found: {method}"),
        )),
    };

    Ok(response)
}

fn discard_stdio_body(reader: &mut BufReader<impl Read>, content_length: usize) -> Result<()> {
    let mut remaining = content_length;
    let mut buffer = [0_u8; 8192];

    while remaining > 0 {
        let read_len = remaining.min(buffer.len());
        reader.read_exact(&mut buffer[..read_len])?;
        remaining -= read_len;
    }

    Ok(())
}

fn json_rpc_error_for_request_error(id: Value, error: MaenifoldError) -> Value {
    let (code, message) = match error {
        MaenifoldError::InvalidPayload(message) => (-32600, message),
        MaenifoldError::Json(error) => (-32700, format!("Parse error: {error}")),
        other => (-32603, other.to_string()),
    };
    error_response(id, code, message)
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "serverInfo": {
            "name": "maenifold",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "capabilities": {
            "tools": {},
        },
        "instructions": "Use maenifold tools to read, write, search, and organize memory files, inspect graph relationships, and run workflow or thinking sessions. Prefer exact tool names from tools/list and pass JSON arguments that match each tool schema.",
    })
}

fn call_tool(params: Value) -> Value {
    let Some(tool_name) = params.get("name").and_then(Value::as_str) else {
        return tool_error("Missing tools/call parameter: name");
    };

    let arguments = match params.get("arguments") {
        Some(Value::Object(_)) => params["arguments"].clone(),
        Some(_) => return tool_error("Invalid tools/call parameter: arguments must be an object"),
        None => json!({}),
    };

    match ToolRegistry::global().try_invoke(tool_name, &arguments, OutputFormat::Json) {
        Ok(output) => json!({
            "content": [
                {
                    "type": "text",
                    "text": output,
                }
            ]
        }),
        Err(err) => json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Error invoking {tool_name}: {err}"),
                }
            ],
            "isError": true,
        }),
    }
}

fn tool_error(message: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": message,
            }
        ],
        "isError": true,
    })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        }
    })
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
    use std::collections::BTreeSet;
    use std::io::Cursor;

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

    #[test]
    fn t_5_2_registry_names_match_canonical_names() {
        let tools = list_mcp_tools(&ToolRegistry::new()).expect("registry mapping should succeed");
        let names = tools.into_iter().map(|tool| tool.name).collect::<Vec<_>>();

        assert!(names.contains(&"WriteMemory".to_string()));
        assert!(names.contains(&"ReadMcpResource".to_string()));
        assert!(names.contains(&"RunFullBenchmark".to_string()));
    }

    #[test]
    fn t_5_8_read_stdio_message_rejects_oversized_frames() {
        let body = vec![b'a'; MAX_MCP_FRAME_SIZE + 1];
        let input = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        let mut framed = input;
        framed.extend_from_slice(&body);
        let mut reader = BufReader::new(Cursor::new(framed));

        let result = read_stdio_message(&mut reader).expect("oversized frame should be handled");

        match result {
            StdioReadResult::ProtocolError { code, message } => {
                assert_eq!(code, -32600);
                assert!(message.contains("MCP frame exceeds max size"));
            }
            other => panic!("expected protocol error, got {other:?}"),
        }
    }

    #[test]
    fn t_5_8_read_stdio_message_reports_parse_errors() {
        let input = b"Content-Length: 8\r\n\r\nnot-json".to_vec();
        let mut reader = BufReader::new(Cursor::new(input));

        let result = read_stdio_message(&mut reader).expect("invalid JSON should be handled");

        match result {
            StdioReadResult::ProtocolError { code, message } => {
                assert_eq!(code, -32700);
                assert!(message.contains("Parse error"));
            }
            other => panic!("expected protocol error, got {other:?}"),
        }
    }

    #[test]
    fn t_5_8_handle_stdio_request_errors_map_to_invalid_request() {
        let response = json_rpc_error_for_request_error(
            json!(7),
            MaenifoldError::InvalidPayload("bad".into()),
        );

        assert_eq!(response["id"], json!(7));
        assert_eq!(response["error"]["code"], json!(-32600));
        assert_eq!(response["error"]["message"], json!("bad"));
    }
}
