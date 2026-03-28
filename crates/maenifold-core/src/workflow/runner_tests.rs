use crate::workflow::runner::{WorkflowRunStatus, run_workflow};
use crate::workflow::runner_test_support::{
    install_acpx_stub, read_stub_state, workflow_context, write_stub_config,
};
use serde_json::json;
use serial_test::serial;
use std::fs;

fn thinking_xml(
    response: &str,
    done: bool,
    needs_more: bool,
    revision_of: Option<u32>,
    conclusion: Option<&str>,
) -> String {
    format!(
        "<output><response>{response}</response><done>{done}</done><needs_more>{needs_more}</needs_more><revision_of>{}</revision_of><conclusion>{}</conclusion></output>",
        revision_of
            .map(|value| value.to_string())
            .unwrap_or_default(),
        conclusion.unwrap_or_default()
    )
}

fn workflow_xml(response: &str, done: bool, conclusion: Option<&str>) -> String {
    format!(
        "<output><response>{response}</response><done>{done}</done><conclusion>{}</conclusion></output>",
        conclusion.unwrap_or_default()
    )
}

#[tokio::test]
#[serial(config)]
async fn workflow_runner_completes_multi_step_workflow() {
    let (_tmp, context, _reset) = workflow_context("happy", 2);
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!({
            "runner-agent": {
                "responses": [
                    { "text": "First [[response]]" },
                    { "text": "Final [[conclusion]]" }
                ]
            }
        }),
    );

    let result = run_workflow(&context, "happy", "runner-agent", "System prompt")
        .await
        .expect("workflow runner");

    assert!(result.session_id.starts_with("workflow-"));
    assert_eq!(result.steps_completed, 2);
    assert_eq!(result.conclusion.as_deref(), Some("Final [[conclusion]]"));
    assert_eq!(result.status, WorkflowRunStatus::Completed);
}

#[tokio::test]
#[serial(config)]
async fn workflow_runner_stops_on_terminal_success() {
    let (_tmp, context, _reset) = workflow_context("single", 1);
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!({ "runner-agent": { "responses": [{ "text": "Done [[conclusion]]" }] } }),
    );

    let result = run_workflow(&context, "single", "runner-agent", "Prompt")
        .await
        .expect("workflow runner");
    let state = read_stub_state(&context.config.maenifold_root);

    assert_eq!(result.steps_completed, 1);
    assert_eq!(state["counts"]["runner-agent"], 1);
    assert_eq!(result.status, WorkflowRunStatus::Completed);
}

#[tokio::test]
#[serial(config)]
async fn workflow_runner_propagates_agent_failures() {
    let (_tmp, context, _reset) = workflow_context("failure", 1);
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!({ "runner-agent": { "stderr": "agent exploded", "exitCode": 2 } }),
    );

    let error = run_workflow(&context, "failure", "runner-agent", "Prompt")
        .await
        .expect_err("workflow runner should fail");
    assert!(error.to_string().contains("agent exploded"));
}

#[tokio::test]
#[serial(config)]
async fn workflow_runner_tags_response_without_wikilinks() {
    let (_tmp, context, _reset) = workflow_context("tagging", 1);
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!({ "runner-agent": { "responses": [{ "text": "plain text no links" }] } }),
    );

    let result = run_workflow(&context, "tagging", "runner-agent", "Prompt")
        .await
        .expect("workflow runner");

    assert_eq!(result.steps_completed, 1);
    assert_eq!(
        result.conclusion.as_deref(),
        Some("plain text no links [[step]]"),
        "fallback_step_tag should append [[step]] to wikilink-free agent responses"
    );
}

#[tokio::test]
#[serial(config)]
async fn workflow_runner_thinks_before_agent_prompt_when_required() {
    let (_tmp, context, _reset) = workflow_context("thinking", 1);
    fs::write(
        context.config.assets_path.join("workflows/thinking.json"),
        serde_json::to_vec_pretty(&json!({
            "steps": [{
                "id": "step-1",
                "description": "Analyze [[step]]",
                "metadata": {
                    "requiresSequentialThinking": true,
                    "toolHints": { "SequentialThinking": { "estimated_thoughts": 1 } }
                }
            }]
        }))
        .expect("workflow json"),
    )
    .expect("write workflow");
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!({
            "runner-agent": {
                "responses": [
                    {
                        "text": thinking_xml(
                            "DONE [[thought]]",
                            true,
                            false,
                            None,
                            Some("DONE [[thought]]"),
                        )
                    },
                    {
                        "text": workflow_xml(
                            "Done [[conclusion]]",
                            true,
                            Some("Done [[conclusion]]"),
                        )
                    }
                ]
            }
        }),
    );

    run_workflow(&context, "thinking", "runner-agent", "Prompt")
        .await
        .expect("workflow runner");
    let state = read_stub_state(&context.config.maenifold_root);
    let prompts = state["prompts"].as_array().expect("prompts");
    let first_workflow_prompt = prompts
        .iter()
        .position(|prompt| {
            prompt["session"]
                .as_str()
                .map(|session| session.starts_with("workflow-"))
                .unwrap_or(false)
        })
        .expect("workflow prompt");

    assert!(
        prompts[0]["session"]
            .as_str()
            .unwrap()
            .starts_with("think-")
    );
    assert!(
        first_workflow_prompt > 0,
        "workflow prompt should be preceded by at least one thinking prompt"
    );
    assert!(prompts[..first_workflow_prompt].iter().all(|prompt| {
        prompt["session"]
            .as_str()
            .map(|session| session.starts_with("think-"))
            .unwrap_or(false)
    }));
    assert!(
        prompts[first_workflow_prompt]["session"]
            .as_str()
            .unwrap()
            .starts_with("workflow-")
    );
    assert_eq!(
        state["observations"][0]["registeredBeforePrompt"].as_bool(),
        Some(true),
        "workflow should register the submachine before driving the first thought"
    );
    assert_eq!(
        state["observations"][0]["activeSubmachineSessionId"].as_str(),
        state["prompts"][0]["session"]
            .as_str()
            .map(|session| session.trim_start_matches("think-"))
    );
    assert_eq!(
        state["observations"][0]["parentWorkflowSession"].as_str(),
        prompts[first_workflow_prompt]["session"].as_str()
    );
}

#[tokio::test]
#[serial(config)]
async fn workflow_runner_skips_thinking_without_flag() {
    let (_tmp, context, _reset) = workflow_context("no-thinking", 1);
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!({ "runner-agent": { "responses": [{ "text": "Done [[conclusion]]" }] } }),
    );

    run_workflow(&context, "no-thinking", "runner-agent", "Prompt")
        .await
        .expect("workflow runner");
    let state = read_stub_state(&context.config.maenifold_root);

    assert_eq!(state["prompts"].as_array().map(Vec::len), Some(1));
    assert!(
        state["prompts"][0]["session"]
            .as_str()
            .unwrap()
            .starts_with("workflow-")
    );
}
