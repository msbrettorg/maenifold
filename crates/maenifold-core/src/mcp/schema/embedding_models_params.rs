#![allow(dead_code)]

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingModelsAction {
    /// Get current loaded model info
    Status,
    /// List curated models with install/active status
    List,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingModelsParams {
    /// The operation to perform
    pub action: EmbeddingModelsAction,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}
