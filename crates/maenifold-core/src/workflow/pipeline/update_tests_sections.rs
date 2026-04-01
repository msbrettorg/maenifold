use super::executor::predecessor_context_content;
use super::test_support::*;
use super::update_context::replace_h2_section;
use super::{NodeState, NodeStatus};
use crate::markdown::{read_markdown, write_markdown};
use crate::sessions::workflow_session_uri;
use crate::util::json::OutputFormat;
use serial_test::serial;
use std::collections::HashMap;

#[test]
#[serial(config)]
fn predecessor_context_returns_none_when_completed_upstream_has_no_last_field() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600850";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600850-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600850-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    let definition = seed_dispatch_pipeline(&context, pipeline_id, node_states.clone(), 8);
    let root_uri = workflow_session_uri("workflow-1735689600850-root");
    let (root_frontmatter, root_content, _) =
        read_markdown(&root_uri, &context.config).expect("read root");
    let mut root_frontmatter = root_frontmatter.expect("root frontmatter");
    root_frontmatter.remove("last");
    write_markdown(
        &root_uri,
        Some(&root_frontmatter),
        &root_content,
        &context.config,
    )
    .expect("rewrite root workflow without last");

    let content = predecessor_context_content(&context, &definition, &node_states, "child")
        .expect("predecessor context lookup");
    assert_eq!(content, None);
}

#[test]
fn pipeline_replace_h2_section_missing_heading_errors() {
    let content = "## Existing Section\n\nSome content\n";
    let result = replace_h2_section(content, "Nonexistent", "new content");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Section not found")
    );
}
