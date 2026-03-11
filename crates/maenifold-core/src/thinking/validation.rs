use crate::error::Result;
use crate::markdown::extract_wikilinks;
use crate::tools::ToolContext;
use serde_json::Value;

use super::render_failure;
use super::session::generate_sequential_session_id;

pub(crate) struct SequentialThinkingInput {
    pub(crate) cancel: bool,
    pub(crate) response: Option<String>,
    pub(crate) thoughts: Option<String>,
    pub(crate) thought_number: u64,
    pub(crate) total_thoughts: u64,
    pub(crate) next_thought_needed: bool,
    pub(crate) needs_more_thoughts: bool,
    pub(crate) is_revision: bool,
    pub(crate) revises_thought: Option<u64>,
    pub(crate) branch_from_thought: Option<u64>,
    pub(crate) branch_id: Option<String>,
    pub(crate) analysis_type: Option<String>,
    pub(crate) parent_workflow_id: Option<String>,
    pub(crate) conclusion: Option<String>,
    pub(crate) session_id_provided: bool,
    pub(crate) session_id: String,
}

impl SequentialThinkingInput {
    pub(crate) fn from_payload(payload: &Value) -> Self {
        let session_id_provided = payload.get("sessionId").and_then(Value::as_str).is_some();
        let session_id = payload
            .get("sessionId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(generate_sequential_session_id);

        Self {
            cancel: payload
                .get("cancel")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            response: payload
                .get("response")
                .and_then(Value::as_str)
                .map(str::to_string),
            thoughts: payload
                .get("thoughts")
                .and_then(Value::as_str)
                .map(str::to_string),
            thought_number: payload
                .get("thoughtNumber")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            total_thoughts: payload
                .get("totalThoughts")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            next_thought_needed: payload
                .get("nextThoughtNeeded")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            needs_more_thoughts: payload
                .get("needsMoreThoughts")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            is_revision: payload
                .get("isRevision")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            revises_thought: payload.get("revisesThought").and_then(Value::as_u64),
            branch_from_thought: payload.get("branchFromThought").and_then(Value::as_u64),
            branch_id: payload
                .get("branchId")
                .and_then(Value::as_str)
                .map(str::to_string),
            analysis_type: payload
                .get("analysisType")
                .and_then(Value::as_str)
                .map(str::to_string),
            parent_workflow_id: payload
                .get("parentWorkflowId")
                .and_then(Value::as_str)
                .map(str::to_string),
            conclusion: payload
                .get("conclusion")
                .and_then(Value::as_str)
                .map(str::to_string),
            session_id_provided,
            session_id,
        }
    }
}

// T-12.0: RTM NFR-1 - preserve SequentialThinking request validation after splitting helpers.
pub(crate) fn validate_payload(
    context: &ToolContext,
    input: &SequentialThinkingInput,
) -> Result<Option<String>> {
    if !input.cancel {
        // PARITY-T07: RTM NFR-1 — treat missing response as empty string so WikiLink
        // validation produces the same "Must include [[WikiLinks]]" message as C#.
        let response_text = input.response.as_deref().unwrap_or("");

        let total_concepts = extract_wikilinks(response_text).len()
            + input
                .thoughts
                .as_deref()
                .map(extract_wikilinks)
                .unwrap_or_default()
                .len();
        if total_concepts == 0 {
            let message =
                "Must include [[WikiLinks]]. Example: 'Analyzing [[Machine Learning]] algorithms'";
            return Ok(Some(render_failure(context, "WIKILINK_REQUIRED", message)));
        }
    }

    if input.branch_from_thought.is_some() && input.branch_id.is_none() {
        let message =
            "branchId required when branchFromThought is specified for multi-agent coordination";
        return Ok(Some(render_failure(context, "BRANCH_ID_REQUIRED", message)));
    }

    Ok(None)
}

pub(crate) fn validate_completion(
    context: &ToolContext,
    input: &SequentialThinkingInput,
) -> Result<Option<String>> {
    if input.cancel || input.next_thought_needed {
        return Ok(None);
    }

    let Some(conclusion) = input.conclusion.as_deref() else {
        let message = "Conclusion required when completing session. Must synthesize findings with [[WikiLinks]].";
        return Ok(Some(render_failure(
            context,
            "CONCLUSION_REQUIRED",
            message,
        )));
    };

    if extract_wikilinks(conclusion).is_empty() {
        let message = "Conclusion must include [[WikiLinks]] for knowledge graph integration.";
        return Ok(Some(render_failure(
            context,
            "CONCLUSION_WIKILINK_REQUIRED",
            message,
        )));
    }

    Ok(None)
}
