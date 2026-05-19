use super::super::conditional_output::extract_conditional_json_value;
use super::*;

const ACP_OUTPUT_SCHEMA_FAILURE: &str =
    "ACP node did not produce valid structured output matching output_schema after 1 repair turn";

pub(super) async fn repair_structured_output_if_needed(
    session: &AgentSession,
    session_config: &AgentSessionConfig,
    agent: &str,
    first: Result<super::super::super::agent_session::AgentResponse>,
) -> Result<super::super::super::agent_session::AgentResponse> {
    let Some(schema) = session_config.output_schema.as_ref() else {
        return first;
    };
    let mut errors = Vec::new();
    let response = first?;
    if let Some(args) = structured_args_from_response(&response, schema, &mut errors) {
        return Ok(response_with_structured_args(response, args));
    }
    let prompt = repair_prompt(schema, &errors);
    let repaired = executor_run_exec::await_with_budget_and_kill(
        agent,
        std::time::Duration::from_secs(executor_run_exec::PROMPT_BUDGET_SECS),
        session.prompt(&prompt),
        || async {
            session.force_kill_child().await;
        },
    )
    .await?;
    let mut repair_errors = Vec::new();
    if let Some(args) = structured_args_from_response(&repaired, schema, &mut repair_errors) {
        return Ok(response_with_structured_args(repaired, args));
    }
    Ok(response_with_hard_failure(repaired))
}

fn repair_prompt(schema: &JsonValue, errors: &[String]) -> String {
    format!(
        "Your previous response did not produce a valid complete_node call. The node's output_schema requires: {}. Validation errors so far: {}. Return ONLY a complete_node tool call with valid arguments. No other content.",
        schema,
        if errors.is_empty() {
            "no complete_node call was made".to_string()
        } else {
            errors.join("; ")
        }
    )
}

fn response_with_structured_args(
    mut response: super::super::super::agent_session::AgentResponse,
    args: JsonValue,
) -> super::super::super::agent_session::AgentResponse {
    response.text = args.to_string();
    response.error = None;
    response
}

fn response_with_hard_failure(
    mut response: super::super::super::agent_session::AgentResponse,
) -> super::super::super::agent_session::AgentResponse {
    response.text = encode_prompt_hard_failure(ACP_OUTPUT_SCHEMA_FAILURE);
    response.error = None;
    response
}

fn structured_args_from_response(
    response: &super::super::super::agent_session::AgentResponse,
    schema: &JsonValue,
    errors: &mut Vec<String>,
) -> Option<JsonValue> {
    let candidate = response
        .complete_node_args
        .clone()
        .or_else(|| extract_conditional_json_value(&response.text));
    let Some(args) = candidate else {
        return None;
    };
    match validate_args_against_schema(&args, schema) {
        Ok(()) => Some(args),
        Err(error) => {
            errors.push(error);
            None
        }
    }
}

fn validate_args_against_schema(
    args: &JsonValue,
    schema: &JsonValue,
) -> std::result::Result<(), String> {
    let validator = jsonschema::validator_for(schema).map_err(|error| error.to_string())?;
    validator.validate(args).map_err(|error| error.to_string())
}
