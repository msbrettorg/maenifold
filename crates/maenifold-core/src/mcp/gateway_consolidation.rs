use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_schema_validation::{compile_validator, compute_schema_hash};
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConsolidatedServerTool {
    pub server_name: String,
    pub tool_catalog: Vec<Tool>,
    pub tool_validators: HashMap<String, CachedToolValidator>,
}

#[derive(Clone)]
pub struct CachedToolValidator {
    pub schema_hash: String,
    pub validator: Option<Arc<jsonschema::Validator>>,
}

impl fmt::Debug for ConsolidatedServerTool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConsolidatedServerTool")
            .field("server_name", &self.server_name)
            .field("tool_catalog", &self.tool_catalog)
            .field("tool_validators", &self.tool_validators)
            .finish()
    }
}

impl fmt::Debug for CachedToolValidator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CachedToolValidator")
            .field("schema_hash", &self.schema_hash)
            .field("has_validator", &self.validator.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GatewayToolCounts {
    pub total: usize,
    pub facades: usize,
    pub passthrough: usize,
}

pub fn build_consolidated_tools(
    gateway_sources: Vec<(String, Vec<Tool>)>,
    min_tools_for_consolidation: usize,
) -> Result<(Vec<ConsolidatedServerTool>, Vec<Tool>)> {
    assert!(
        min_tools_for_consolidation >= 2,
        "min_tools_for_consolidation must be at least 2"
    );

    let mut consolidated = Vec::new();
    let mut passthrough = Vec::new();

    for (server_name, mut tools) in gateway_sources {
        validate_server_name(&server_name)?;
        if tools.is_empty() {
            continue;
        }

        tools.sort_by(|left, right| tool_name(left).cmp(&tool_name(right)));

        consolidated.push(ConsolidatedServerTool {
            server_name: server_name.clone(),
            tool_validators: cached_tool_validators(&tools),
            tool_catalog: tools,
        });
    }

    consolidated.sort_by(|left, right| left.server_name.cmp(&right.server_name));
    passthrough.sort_by(|left, right| tool_name(left).cmp(&tool_name(right)));
    Ok((consolidated, passthrough))
}

pub(super) fn gateway_tool_counts(
    consolidated: &[ConsolidatedServerTool],
    passthrough: &[Tool],
    native_tools: &[Tool],
) -> GatewayToolCounts {
    GatewayToolCounts {
        total: consolidated.len() + passthrough.len() + native_tools.len(),
        facades: consolidated.len(),
        passthrough: passthrough.len(),
    }
}

fn validate_server_name(server_name: &str) -> Result<()> {
    if server_name.trim().is_empty() {
        return Err(MaenifoldError::Message(
            "Gateway server name cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn cached_tool_validators(tools: &[Tool]) -> HashMap<String, CachedToolValidator> {
    tools
        .iter()
        .map(|tool| {
            let schema = Value::Object((*tool.input_schema).clone());
            (
                tool_name(tool),
                CachedToolValidator {
                    schema_hash: compute_schema_hash(&schema),
                    validator: compile_validator(&schema),
                },
            )
        })
        .collect()
}

fn tool_name(tool: &Tool) -> String {
    tool.name.to_string()
}

#[cfg(test)]
#[path = "gateway_consolidation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gateway_consolidation_counts_tests.rs"]
mod counts_tests;
