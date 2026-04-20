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
    /// Set the active model (writes config; daemon restart + sync required to apply)
    Activate,
    /// Delete an installed model from disk (refuses to delete the active model)
    Delete,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingModelsParams {
    /// The operation to perform
    pub action: EmbeddingModelsAction,
    /// Model identifier (catalog ID, e.g. "bge-small-en-v1.5"). Required for activate/delete.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}
