use crate::markdown::read_markdown;
use crate::sessions::{date_parts_from_session_id, workflow_session_uri};
use crate::workflow::handler;
use crate::workflow::runner_helpers::extract_workflow_session_id;
use crate::workflow::runner_test_support::{
    install_acpx_stub, think_context, workflow_context, write_stub_config,
};
use crate::workflow::think_runner::{ThinkOptions, run_think};
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial(config)]
async fn happy_path_stops_when_agent_says_done() {
    let (_tmp, context, _reset) = think_context();
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!(["first [[one]]", "second [[two]]", "DONE [[three]]"]),
    );

    let result = run_think(&context, "codex", "Topic", ThinkOptions::default())
        .await
        .expect("think runner");
    assert_eq!(result.thoughts_completed, 3);
    assert_eq!(result.conclusion.as_deref(), Some("DONE [[three]]"));
}

#[tokio::test]
#[serial(config)]
async fn overflow_protection_stops_at_total_plus_three() {
    let (_tmp, context, _reset) = think_context();
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!(vec!["keep going [[step]]"; 6]),
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
}

#[tokio::test]
#[serial(config)]
async fn injects_step_wikilink_when_agent_response_has_none() {
    let (_tmp, context, _reset) = think_context();
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(
        &context.config.maenifold_root,
        json!(["plain text", "DONE plain text"]),
    );

    let result = run_think(&context, "codex", "Topic", ThinkOptions::default())
        .await
        .expect("think runner");
    let (year, month, day) = date_parts_from_session_id(&result.session_id);
    let uri = format!(
        "memory://thinking/sequential/{year:04}/{month:02}/{day:02}/{}",
        result.session_id
    );
    let (_, content, _) = read_markdown(&uri, &context.config).expect("read thinking session");
    assert!(content.contains("plain text [[step]]"));
    assert!(content.contains("DONE plain text [[step]]"));
}

#[tokio::test]
#[serial(config)]
async fn run_think_seeds_parent_workflow_linkage() {
    let (_tmp, context, _reset) = workflow_context("parent-link", 1);
    let _path = install_acpx_stub(&context.config.maenifold_root);
    write_stub_config(&context.config.maenifold_root, json!(["DONE [[child]]"]));

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

    let (year, month, day) = date_parts_from_session_id(&result.session_id);
    let uri = format!(
        "memory://thinking/sequential/{year:04}/{month:02}/{day:02}/{}",
        result.session_id
    );
    let (frontmatter, _, _) = read_markdown(&uri, &context.config).expect("read thinking");
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
            .any(|value| value.as_str() == Some(related_link.as_str())),
        "workflow frontmatter should link the seeded sequential session"
    );
}
