use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SamplingAction {
    AllowOnce,
    AllowAlways,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SamplingDecision {
    pub action: SamplingAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_agent: Option<String>,
}
