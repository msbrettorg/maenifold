use crate::mcp::elicitation_validation::validate_elicitation_response;
use rmcp::model::{CreateElicitationRequestParams, ElicitationAction};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UserPromptAction {
    Accept,
    Cancel,
    Dismiss,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ElicitationResponse {
    pub(crate) action: UserPromptAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<Value>,
}

impl ElicitationResponse {
    pub(crate) fn to_mcp_action(&self) -> ElicitationAction {
        match self.action {
            UserPromptAction::Accept => ElicitationAction::Accept,
            UserPromptAction::Cancel => ElicitationAction::Cancel,
            UserPromptAction::Dismiss => ElicitationAction::Decline,
        }
    }
}

pub(super) fn validate_response_content(
    params: &CreateElicitationRequestParams,
    content: &Value,
) -> Result<(), Vec<String>> {
    if let Some(schema) = requested_schema_value(params) {
        validate_elicitation_response(&schema, content)?;
    }
    Ok(())
}

pub(super) fn requested_schema_value(params: &CreateElicitationRequestParams) -> Option<Value> {
    match params {
        CreateElicitationRequestParams::FormElicitationParams {
            requested_schema, ..
        } => serde_json::to_value(requested_schema).ok(),
        CreateElicitationRequestParams::UrlElicitationParams { .. } => None,
    }
}
