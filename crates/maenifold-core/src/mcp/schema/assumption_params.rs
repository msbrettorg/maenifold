#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AssumptionLedgerAction {
    Append,
    Update,
    Read,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AssumptionStatus {
    Active,
    Validated,
    Invalidated,
    Refined,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssumptionLedgerParams {
    /// Ledger action to perform
    pub action: AssumptionLedgerAction,
    /// Assumption statement for append operations
    #[serde(default)]
    pub assumption: Option<String>,
    /// Workflow or thinking session reference
    #[serde(default)]
    pub context: Option<String>,
    /// Planned validation method
    #[serde(default)]
    pub validation_plan: Option<String>,
    /// Confidence level such as high, medium, or low
    #[serde(default)]
    pub confidence: Option<String>,
    /// Concept tags for knowledge graph integration
    #[serde(default)]
    pub concepts: Option<Vec<String>>,
    /// Memory URI for update or read actions
    #[serde(default)]
    pub uri: Option<String>,
    /// Updated assumption status
    #[serde(default)]
    pub status: Option<AssumptionStatus>,
    /// Additional notes for update operations
    #[serde(default)]
    pub notes: Option<String>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}
