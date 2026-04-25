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
    /// Download a model from HuggingFace into the managed catalog (subdir layout)
    #[serde(rename = "download")]
    Download,
    /// Set the active model (writes config; daemon restart + sync required to apply)
    Activate,
    /// Delete an installed model from disk (refuses to delete the active model)
    Delete,
    /// Drop vec0 tables and clear embedding dimensions metadata (rebuild on next sync)
    #[serde(rename = "regenerate")]
    Regenerate,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingModelsParams {
    /// The operation to perform
    pub action: EmbeddingModelsAction,
    /// Model identifier (catalog ID, e.g. "bge-small-en-v1.5"). Required for activate/delete.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Explicitly trust and use an external `{assets_path}/models/catalog.json` override.
    #[serde(default)]
    pub trust_external_catalog: bool,
    /// Explicitly allow downloads/activation for non-MIT/Apache model licenses.
    #[serde(default)]
    pub allow_restricted_license: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}
