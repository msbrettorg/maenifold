mod assumption_params;
pub(crate) mod config_tools;
#[cfg(test)]
mod config_tools_tests;
mod embedding_models_params;
mod files_params;
pub(crate) mod gateway_tools;
#[cfg(test)]
mod gateway_tools_schema_tests;
#[cfg(test)]
mod generic_tool_tests;
mod graph_params;
#[cfg(test)]
mod graph_params_tests;
mod memory_params;
mod params;
mod pipeline_node_schema;
mod pipeline_node_schema_data_variants;
mod pipeline_node_schema_workflow_variants;
mod pipeline_node_types;
mod pipeline_params;
#[cfg(test)]
mod pipeline_params_tests;
mod support;
mod system_params;
mod thinking_workflow_params;

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::Result;
use crate::mcp::McpToolDefinition;
use serde::Deserialize;
use serde_json::{Map, Value, json};

pub(crate) use params::{
    AcpExecContextBlock, AcpExecImageContext, AcpExecResourceContext, AcpExecTextContext,
    EmbeddingModelsAction, EmbeddingModelsParams,
};

pub(crate) fn mcp_tool_from_registry_name(registry_name: &str) -> Result<McpToolDefinition> {
    Ok(manifest_tools()
        .get(registry_name)
        .cloned()
        .unwrap_or_else(|| {
            tracing::warn!("No schema mapping for tool {registry_name}, using generic schema");
            McpToolDefinition {
                name: registry_name.to_string(),
                description: "Gateway tool (use learn=true for details)".to_string(),
                input_schema: json!({ "type": "object" }),
                execution: None,
                meta: None,
            }
        }))
}

fn manifest_tools() -> &'static HashMap<String, McpToolDefinition> {
    static TOOLS: OnceLock<HashMap<String, McpToolDefinition>> = OnceLock::new();
    TOOLS.get_or_init(|| {
        let manifest: ToolManifest =
            serde_json::from_str(include_str!("../../../assets/tools_manifest.json"))
                .expect("tools_manifest.json should be valid");
        manifest
            .tools
            .into_iter()
            .map(|tool| (tool.name.clone(), tool.into_definition()))
            .collect()
    })
}

#[derive(Deserialize)]
struct ToolManifest {
    tools: Vec<ToolManifestEntry>,
}

#[derive(Deserialize)]
struct ToolManifestEntry {
    name: String,
    description: String,
    input_schema: serde_json::Value,
    #[serde(default)]
    execution: Option<rmcp::model::ToolExecution>,
    #[serde(rename = "_meta", default)]
    meta: Option<Map<String, Value>>,
}

impl ToolManifestEntry {
    fn into_definition(self) -> McpToolDefinition {
        let meta = self.meta.or_else(|| default_tool_meta(&self.name));
        McpToolDefinition {
            name: self.name,
            description: self.description,
            input_schema: self.input_schema,
            execution: self.execution,
            meta,
        }
    }
}

fn default_tool_meta(_name: &str) -> Option<Map<String, Value>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{TaskSupport, ToolExecution};
    use std::collections::BTreeMap;

    fn parsed_manifest() -> ToolManifest {
        serde_json::from_str(include_str!("../../../assets/tools_manifest.json"))
            .expect("tools_manifest.json fixture should parse")
    }

    fn expected_descriptions() -> BTreeMap<String, String> {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/expected_tool_descriptions.json"
        ))
        .expect("expected tool descriptions fixture should parse")
    }

    #[test]
    fn tools_manifest_loads_successfully() {
        let tools = manifest_tools();
        assert!(!tools.is_empty(), "manifest registry should not be empty");
        assert_eq!(tools.len(), 34, "manifest registry should contain 34 tools");
    }

    #[test]
    fn every_manifest_tool_resolves_via_registry_lookup() {
        for entry in parsed_manifest().tools {
            let resolved =
                mcp_tool_from_registry_name(&entry.name).expect("manifest tool lookup should work");
            assert_eq!(resolved.name, entry.name);
            assert_eq!(resolved.description, entry.description);
            assert_eq!(resolved.input_schema, entry.input_schema);
            assert_eq!(resolved.execution, entry.execution);
        }
    }

    #[test]
    fn pipeline_tool_has_execution_task_support_optional() {
        let pipeline = mcp_tool_from_registry_name("Pipeline").expect("Pipeline tool should load");
        assert_eq!(
            pipeline.execution.as_ref(),
            Some(&ToolExecution::new().with_task_support(TaskSupport::Optional))
        );
    }

    #[test]
    fn recent_activity_tool_has_timespan_regex_pattern() {
        let recent_activity =
            mcp_tool_from_registry_name("RecentActivity").expect("RecentActivity should load");
        let pattern = recent_activity
            .input_schema
            .get("properties")
            .and_then(|properties| properties.get("timespan"))
            .and_then(|timespan| timespan.get("pattern"))
            .and_then(serde_json::Value::as_str);

        assert_eq!(pattern, Some(r"^(\d+\.)?\d{2}:\d{2}:\d{2}(\.\d{1,7})?$"));
    }

    #[test]
    fn manifest_parity_vs_deleted_shims() {
        let expected = expected_descriptions();
        let actual = parsed_manifest()
            .tools
            .into_iter()
            .map(|tool| (tool.name, tool.description))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(
            expected.len(),
            34,
            "fixture should contain 34 tool descriptions"
        );
        assert_eq!(
            actual.len(),
            34,
            "manifest should contain 34 tool descriptions"
        );
        assert_eq!(actual, expected);
    }
}
