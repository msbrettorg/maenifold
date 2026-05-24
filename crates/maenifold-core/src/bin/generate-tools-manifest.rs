mod mcp {
    use rmcp::model::ToolExecution;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct McpToolDefinition {
        pub name: String,
        pub description: String,
        pub input_schema: Value,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pub execution: Option<ToolExecution>,
    }
}

#[path = "../mcp/schema/assumption_params.rs"]
mod assumption_params;
#[path = "../mcp/schema/embedding_models_params.rs"]
mod embedding_models_params;
#[path = "../mcp/schema/files_params.rs"]
mod files_params;
#[path = "../mcp/schema/graph_params.rs"]
mod graph_params;
#[path = "../mcp/schema/memory_params.rs"]
mod memory_params;
#[path = "../mcp/schema/params.rs"]
mod params;
#[path = "../mcp/schema/pipeline_node_schema.rs"]
mod pipeline_node_schema;
#[path = "../mcp/schema/pipeline_node_schema_data_variants.rs"]
mod pipeline_node_schema_data_variants;
#[path = "../mcp/schema/pipeline_node_schema_workflow_variants.rs"]
mod pipeline_node_schema_workflow_variants;
#[path = "../mcp/schema/pipeline_node_types.rs"]
mod pipeline_node_types;
#[path = "../mcp/schema/pipeline_params.rs"]
mod pipeline_params;
#[path = "../mcp/schema/support.rs"]
mod support;
#[path = "../mcp/schema/system_params.rs"]
mod system_params;
#[path = "../mcp/schema/thinking_workflow_params.rs"]
mod thinking_workflow_params;

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use mcp::McpToolDefinition;
use rmcp::model::{TaskSupport, ToolExecution};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use files_params::ReadTextFileParams;
use params::*;
use support::{
    derived_tool_definition, schema_value, schema_value_no_null_options, tool_definition,
};

const OUTPUT_PATH: &str = "assets/tools_manifest.json";
const GENERATED_BY: &str = "generate-tools-manifest";
const RECENT_ACTIVITY_PATTERN: &str = r"^(\d+\.)?\d{2}:\d{2}:\d{2}(\.\d{1,7})?$";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("--verify") => verify_manifest()?,
        Some(flag) => return Err(format!("unknown flag: {flag}").into()),
        None => write_manifest()?,
    }
    Ok(())
}

fn write_manifest() -> Result<(), Box<dyn std::error::Error>> {
    let path = output_path();
    let manifest = Manifest {
        version: 1,
        generated_by: GENERATED_BY.to_string(),
        tools: legacy_tools().into_iter().map(ManifestTool::from).collect(),
    };
    let json = serde_json::to_string_pretty(&sort_manifest_value(serde_json::to_value(manifest)?))?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

fn verify_manifest() -> Result<(), Box<dyn std::error::Error>> {
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(output_path())?)?;
    let actual: HashMap<_, _> = manifest
        .tools
        .into_iter()
        .map(ManifestTool::into_definition)
        .map(|tool| (tool.name.clone(), canonical_bytes(&tool)))
        .collect();
    let expected = legacy_tools();
    if actual.len() != expected.len() {
        return Err(format!("expected {} tools, found {}", expected.len(), actual.len()).into());
    }
    for tool in expected {
        let Some(current) = actual.get(&tool.name) else {
            return Err(format!("missing tool {}", tool.name).into());
        };
        let expected_bytes = canonical_bytes(&tool);
        if current != &expected_bytes {
            return Err(format!("manifest mismatch for {}", tool.name).into());
        }
    }
    println!("verified {} tools", actual.len());
    Ok(())
}

fn legacy_tools() -> Vec<McpToolDefinition> {
    let mut tools: Vec<_> = tool_specs()
        .into_iter()
        .map(|spec| (spec.build)(spec.name, spec.description))
        .collect();
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

#[rustfmt::skip]
fn tool_specs() -> Vec<ToolSpec> {
    vec![
        spec::<WriteMemoryParams>("WriteMemory", "Creates new knowledge files with [[WikiLinks]] for graph integration."),
        spec::<ReadMemoryParams>("ReadMemory", "Retrieves an existing knowledge file by URI or title."),
        spec::<EditMemoryParams>("EditMemory", "Modifies an existing knowledge file while preserving WikiLinks."),
        spec::<DeleteMemoryParams>("DeleteMemory", "Deletes an existing knowledge file after explicit confirmation."),
        spec::<MoveMemoryParams>("MoveMemory", "Moves or renames a knowledge file while preserving WikiLinks."),
        spec::<SearchMemoriesParams>("SearchMemories", "Searches knowledge files using hybrid, semantic, or full-text ranking."),
        spec::<ExtractConceptsFromFileParams>("ExtractConceptsFromFile", "Extracts WikiLink concepts from a specific knowledge file."),
        spec::<ListMemoriesParams>("ListMemories", "Lists knowledge files or folders relative to the memory root."),
        spec::<SyncParams>("Sync", "Synchronizes WikiLink concepts from memory files into the graph database."),
        spec::<BuildContextParams>("BuildContext", "Builds graph context around a concept by traversing related knowledge."),
        spec::<VisualizeParams>("Visualize", "Generates Mermaid or graph JSON output for concept relationships."),
        spec::<RepairConceptsParams>("RepairConcepts", "Repairs concept variants across memory files by replacing them with a canonical form."),
        spec::<AnalyzeConceptCorruptionParams>("AnalyzeConceptCorruption", "Analyzes concept families to find duplicate or corrupted WikiLink variants."),
        spec::<FindSimilarConceptsParams>("FindSimilarConcepts", "Finds semantically similar concepts using vector embeddings."),
        spec::<SequentialThinkingParams>("SequentialThinking", "Creates structured thinking sessions with persistent branching, conclusions, and child submachines."),
        ToolSpec { name: "Pipeline", description: "Creates and manages pipeline sessions composed of DAG nodes.", build: pipeline_tool },
        spec::<WorkflowParams>("Workflow", "Runs structured workflows with optional nested submachines."),
        spec::<DeleteWorkflowAssetParams>("DeleteWorkflowAsset", "Deletes one workflow asset by name when no running pipeline references it."),
        spec::<SaveWorkflowAssetParams>("SaveWorkflowAsset", "Saves (creates or overwrites) a workflow asset by id."),
        ToolSpec { name: "RecentActivity", description: "Reports recently indexed files with optional content excerpts.", build: recent_activity_tool },
        spec::<AssumptionLedgerParams>("AssumptionLedger", "Creates, updates, or reads assumptions linked to workflow or thinking context."),
        spec::<ReadTextFileParams>("ReadTextFile", "Reads a UTF-8 text file from an absolute path or a cwd-relative path."),
        spec::<MemoryStatusParams>("MemoryStatus", "Provides high-level memory system statistics and health information."),
        spec::<GetConfigParams>("GetConfig", "Displays maenifold configuration settings and resolved paths."),
        spec::<GetCopilotModelsParams>("GetCopilotModels", "Fetches Copilot /models capabilities through the daemon cache and returns exact per-model reasoning-effort options."),
        spec::<GetHelpParams>("GetHelp", "Loads canonical help for a named public MCP tool or gateway command. Use it after you know which tool or command needs deeper usage details. It complements GatewayIndex; it is not the first cold-start domain discovery call."),
        spec::<UpdateAssetsParams>("UpdateAssets", "Refreshes runtime assets from packaged assets with optional dry-run mode."),
        spec::<RunFullBenchmarkParams>("RunFullBenchmark", "Runs system performance benchmarks across search, sync, and graph traversal."),
        spec::<AdoptParams>("Adopt", "Loads a role, color, or perspective asset as an adoption instruction."),
        spec::<ListAssetsParams>("ListAssets", "Lists available asset types or metadata for a specific asset type."),
        spec::<ReadMcpResourceParams>("ReadMcpResource", "Reads a concrete MCP resource URI such as asset://catalog or asset://.../id. Use it when you already have a resource URI and need packaged resource content. It does not discover gateway domains or read memory:// notes; use GatewayIndex for domains."),
        spec::<ListSessionsParams>("ListSessions", "Lists resumable ACP sessions across configured agents."),
        spec::<AcpExecParams>("AcpExec", "Unified ACP agent invocation. Supports prompt (single message), think (sequential thinking), and workflow (structured workflow) modes. All modes share a global agent pool for process reuse."),
        spec::<EmbeddingModelsParams>("EmbeddingModels", "Manages embedding models and ONNX Runtime downloads. List curated models, check status, and download runtime assets."),
    ]
}

fn spec<T: JsonSchema>(name: &'static str, description: &'static str) -> ToolSpec {
    ToolSpec {
        name,
        description,
        build: derived::<T>,
    }
}

fn derived<T: JsonSchema>(name: &str, description: &str) -> McpToolDefinition {
    derived_tool_definition::<T>(name, description)
}

fn pipeline_tool(name: &str, description: &str) -> McpToolDefinition {
    let mut tool = tool_definition(
        name,
        description,
        schema_value_no_null_options::<PipelineParams>(),
    );
    tool.execution = Some(ToolExecution::new().with_task_support(TaskSupport::Optional));
    tool
}

fn recent_activity_tool(name: &str, description: &str) -> McpToolDefinition {
    let mut schema = schema_value::<RecentActivityParams>();
    if let Some(timespan) = schema
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .and_then(|properties| properties.get_mut("timespan"))
        .and_then(Value::as_object_mut)
    {
        timespan.insert(
            "pattern".to_string(),
            Value::String(RECENT_ACTIVITY_PATTERN.to_string()),
        );
    }
    tool_definition(name, description, schema)
}

fn canonical_bytes(tool: &McpToolDefinition) -> Vec<u8> {
    serde_json::to_vec(&sort_manifest_value(
        serde_json::to_value(ManifestTool::from(tool.clone())).expect("serialize"),
    ))
    .expect("json bytes")
}

fn sort_manifest_value(mut value: Value) -> Value {
    sort_value(&mut value);
    value
}

fn sort_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (key, mut nested) in std::mem::take(map) {
                sort_value(&mut nested);
                sorted.insert(key, nested);
            }
            for (key, nested) in sorted {
                map.insert(key, nested);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(sort_value),
        _ => {}
    }
}

fn output_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(OUTPUT_PATH)
}

#[derive(Clone, Copy)]
struct ToolSpec {
    name: &'static str,
    description: &'static str,
    build: fn(&str, &str) -> McpToolDefinition,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    generated_by: String,
    tools: Vec<ManifestTool>,
}

#[derive(Clone, Serialize, Deserialize)]
struct ManifestTool {
    name: String,
    description: String,
    input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    execution: Option<ToolExecution>,
}

impl From<McpToolDefinition> for ManifestTool {
    fn from(tool: McpToolDefinition) -> Self {
        Self {
            name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
            execution: tool.execution,
        }
    }
}

impl ManifestTool {
    fn into_definition(self) -> McpToolDefinition {
        McpToolDefinition {
            name: self.name,
            description: self.description,
            input_schema: self.input_schema,
            execution: self.execution,
        }
    }
}
