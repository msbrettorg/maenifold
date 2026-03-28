use crate::markdown::read_markdown;
use crate::sessions::{date_parts_from_session_id, workflow_session_uri};
use crate::workflow::handler;
use crate::workflow::runner_helpers::extract_workflow_session_id;
use crate::workflow::runner_test_support::{
    install_acpx_stub, read_stub_state, think_context, workflow_context, write_stub_config,
};
use crate::workflow::output_types::ThinkingOutput;
use crate::workflow::think_runner::{ThinkOptions, run_think, think_progress_payload_from_output};
use serde_json::json;
use serial_test::serial;

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

fn think_uri(session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/sequential/{year:04}/{month:02}/{day:02}/{session_id}")
}

#[tokio::test]
#[serial(config)]
async fn structured_output_parsing_works_in_the_loop() {
    let (_tmp, context, _reset) = think_context();
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!([
            thinking_xml("first [[one]]", false, false, None, None),
            thinking_xml("second [[two]]", true, false, None, Some("done [[three]]"))
        ]),
    );

    let result = run_think(&context, "codex", "Topic", ThinkOptions::default())
        .await
        .expect("think runner");
    let (_, content, _) = read_markdown(&think_uri(&result.session_id), &context.config)
        .expect("read thinking session");

    assert_eq!(result.thoughts_completed, 2);
    assert_eq!(result.conclusion.as_deref(), Some("done [[three]]"));
    assert!(content.contains("first [[one]]"));
    assert!(content.contains("second [[two]]"));
    assert!(content.contains("done [[three]]"));
    assert!(!content.contains("<output>"));
}

#[test]
fn needs_more_true_maps_to_payload() {
    let payload = think_progress_payload_from_output(
        "session-1",
        &ThinkingOutput {
            response: "more [[thought]]".to_string(),
            done: false,
            needs_more: true,
            revision_of: None,
            conclusion: None,
        },
        2,
        &ThinkOptions::default(),
    );
    assert_eq!(payload["nextThoughtNeeded"], json!(true));
    assert_eq!(payload["needsMoreThoughts"], json!(true));
}

#[test]
fn revision_of_maps_to_revision_payload_fields() {
    let payload = think_progress_payload_from_output(
        "session-1",
        &ThinkingOutput {
            response: "revise [[thought]]".to_string(),
            done: false,
            needs_more: false,
            revision_of: Some(2),
            conclusion: None,
        },
        3,
        &ThinkOptions::default(),
    );
    assert_eq!(payload["isRevision"], json!(true));
    assert_eq!(payload["revisesThought"], json!(2));
}

#[tokio::test]
#[serial(config)]
async fn done_true_with_conclusion_terminates_loop() {
    let (_tmp, context, _reset) = think_context();
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!([thinking_xml(
            "done response [[one]]",
            true,
            false,
            None,
            Some("final [[one]]"),
        )]),
    );

    let result = run_think(&context, "codex", "Topic", ThinkOptions::default())
        .await
        .expect("think runner");
    let state = read_stub_state(&context.config.maenifold_root);

    assert_eq!(result.thoughts_completed, 1);
    assert_eq!(result.conclusion.as_deref(), Some("final [[one]]"));
    assert_eq!(state["prompts"].as_array().expect("prompts").len(), 1);
}

#[tokio::test]
#[serial(config)]
async fn retry_on_parse_failure_uses_rewritten_output() {
    let (_tmp, context, _reset) = think_context();
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!([
            "bad response [[broken]]",
            thinking_xml("recovered [[ok]]", true, false, None, Some("fixed [[ok]]"))
        ]),
    );

    let result = run_think(&context, "codex", "Topic", ThinkOptions::default())
        .await
        .expect("think runner");
    let state = read_stub_state(&context.config.maenifold_root);

    assert_eq!(result.thoughts_completed, 1);
    assert_eq!(result.conclusion.as_deref(), Some("fixed [[ok]]"));
    assert_eq!(state["prompts"].as_array().expect("prompts").len(), 2);
}

#[tokio::test]
#[serial(config)]
async fn overflow_protection_stops_at_total_plus_three() {
    let (_tmp, context, _reset) = think_context();
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!(vec![
            thinking_xml(
                "keep going [[step]]",
                false,
                false,
                None,
                None
            );
            6
        ]),
    );

    let result = run_think(
        &context,
        "codex",
        "Topic",
        ThinkOptions {
            total_thoughts: 1,
            parent_workflow_id: None,
        },
    )
    .await
    .expect("think runner");

    assert_eq!(result.thoughts_completed, 4);
    assert_eq!(result.conclusion.as_deref(), Some("keep going [[step]]"));
}

#[tokio::test]
#[serial(config)]
async fn run_think_seeds_parent_workflow_linkage() {
    let (_tmp, context, _reset) = workflow_context("parent-link", 1);
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!([thinking_xml(
            "child response [[child]]",
            true,
            false,
            None,
            Some("child conclusion [[child]]"),
        )]),
    );

    let start = handler::workflow(&context, &json!({ "workflowId": "parent-link" }))
        .expect("start workflow");
    let workflow_session_id = extract_workflow_session_id(&start).expect("workflow session id");

    let result = run_think(
        &context,
        "codex",
        "Topic",
        ThinkOptions {
            total_thoughts: 1,
            parent_workflow_id: Some(workflow_session_id.clone()),
        },
    )
    .await
    .expect("think runner");

    let (frontmatter, _, _) =
        read_markdown(&think_uri(&result.session_id), &context.config).expect("read thinking");
    let expected_parent = format!("[[workflow/{workflow_session_id}]]");
    assert_eq!(
        frontmatter
            .as_ref()
            .and_then(|values| values.get("parent"))
            .and_then(|value| value.as_str()),
        Some(expected_parent.as_str())
    );

    let (workflow_frontmatter, _, _) =
        read_markdown(&workflow_session_uri(&workflow_session_id), &context.config)
            .expect("read workflow");
    let related_link = format!("[[sequential/{}]]", result.session_id);
    let related = workflow_frontmatter
        .as_ref()
        .and_then(|values| values.get("related"))
        .and_then(|value| value.as_sequence())
        .expect("workflow related links");
    assert!(
        related
            .iter()
            .any(|value| value.as_str() == Some(related_link.as_str()))
    );
}
