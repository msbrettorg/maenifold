#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadTextFileParams {
    /// File path to read. Absolute paths are used directly; relative paths require cwd.
    pub path: String,
    /// Optional current working directory used to resolve relative paths.
    #[serde(default)]
    pub cwd: Option<String>,
}

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "ReadTextFile" => derived_tool_definition::<ReadTextFileParams>(
            registry_name,
            "Reads a UTF-8 text file from an absolute path or a cwd-relative path.",
        ),
        _ => return None,
    };

    Some(tool)
}
