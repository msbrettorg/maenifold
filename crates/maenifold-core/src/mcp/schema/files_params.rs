#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

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
