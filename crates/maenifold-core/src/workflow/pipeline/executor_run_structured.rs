#[allow(deprecated)]
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
    if let Ok(valid) =
        early_return_if_valid(response, schema, session.fenced_json_fallback, &mut errors)
    {
        return Ok(valid);
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
    if let Some(args) = structured_args_from_response(
        &repaired,
        schema,
        session.fenced_json_fallback,
        &mut repair_errors,
    ) {
        return Ok(response_with_structured_args(repaired, args));
    }
    Ok(response_with_hard_failure(repaired))
}

fn early_return_if_valid(
    response: super::super::super::agent_session::AgentResponse,
    schema: &JsonValue,
    fenced_json_fallback: bool,
    errors: &mut Vec<String>,
) -> std::result::Result<
    super::super::super::agent_session::AgentResponse,
    super::super::super::agent_session::AgentResponse,
> {
    if let Some(args) =
        structured_args_from_response(&response, schema, fenced_json_fallback, errors)
    {
        Ok(response_with_structured_args(response, args))
    } else {
        Err(response)
    }
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
    fenced_json_fallback: bool,
    errors: &mut Vec<String>,
) -> Option<JsonValue> {
    let candidate = response.complete_node_args.clone().or_else(|| {
        if fenced_json_fallback {
            #[allow(deprecated)]
            extract_conditional_json_value(&response.text)
        } else {
            None
        }
    });
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

#[cfg(test)]
mod acp_output_schema_repair_tests {
    use serde_json::{Value as JsonValue, json};

    use super::super::{NodeOutcome, NodeTaskResult, PromptExecutionResult, classify_node_result};
    use super::*;
    use crate::workflow::agent_session::AgentResponse;

    fn output_schema() -> JsonValue {
        json!({"type":"object","required":["verdict"],"properties":{"verdict":{"enum":["PASS","FAIL"]}}})
    }

    fn response(text: &str, complete_node_args: Option<JsonValue>) -> AgentResponse {
        AgentResponse {
            text: text.to_string(),
            error: None,
            input_tokens: None,
            output_tokens: None,
            model: Some("test-model".to_string()),
            complete_node_args,
        }
    }

    #[test]
    fn tool_capable_agent_without_complete_node_call_hard_fails_after_repair() {
        let mut errors = Vec::new();
        let repaired = response("```json\n{\"verdict\":\"PASS\"}\n```", None);
        let args = structured_args_from_response(&repaired, &output_schema(), false, &mut errors);

        assert_eq!(args, None);
        let failure = response_with_hard_failure(repaired);
        let outcome = classify_node_result(
            "source",
            NodeTaskResult::Prompt(Ok(PromptExecutionResult {
                response: failure.text,
                acp_session_id: Some("acp-1".to_string()),
                input_tokens: None,
                output_tokens: None,
                model: Some("test-model".to_string()),
            })),
        );
        let NodeOutcome::Abandoned { reason, .. } = outcome else {
            panic!("missing complete_node args after repair must hard-fail the node");
        };
        assert_eq!(reason, ACP_OUTPUT_SCHEMA_FAILURE);
    }

    #[test]
    fn wrapper_gates_fallback_when_session_flag_false() {
        let mut errors = Vec::new();
        let first = response("```json\n{\"verdict\":\"PASS\"}\n```", None);
        let result = early_return_if_valid(first, &output_schema(), false, &mut errors);

        assert_eq!(
            result,
            Err(response("```json\n{\"verdict\":\"PASS\"}\n```", None))
        );
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn tool_incapable_agent_with_fenced_json_succeeds() {
        let mut errors = Vec::new();
        let repaired = response("```json\n{\"verdict\":\"PASS\"}\n```", None);
        let args = structured_args_from_response(&repaired, &output_schema(), true, &mut errors);

        assert_eq!(args, Some(json!({"verdict":"PASS"})));
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn tool_capable_agent_with_valid_complete_node_succeeds() {
        let mut errors = Vec::new();
        let repaired = response(
            "prose that must not be parsed",
            Some(json!({"verdict":"PASS"})),
        );
        let args = structured_args_from_response(&repaired, &output_schema(), false, &mut errors);

        assert_eq!(args, Some(json!({"verdict":"PASS"})));
        assert_eq!(errors, Vec::<String>::new());
    }
}

fn validate_args_against_schema(
    args: &JsonValue,
    schema: &JsonValue,
) -> std::result::Result<(), String> {
    let validator = jsonschema::validator_for(schema).map_err(|error| error.to_string())?;
    validator.validate(args).map_err(|error| error.to_string())
}
