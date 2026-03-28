use crate::markdown::{Frontmatter, read_markdown};
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::tools::ToolContext;
use serde_json::{Value, json};
use std::collections::HashMap;
use tempfile::TempDir;

use super::super::NodeState;
use super::super::handler::pipeline;
use super::super::session_io::frontmatter_value;
use super::super::test_context::ConfigResetGuard;
use super::super::test_support::pipeline_test_context;

pub(super) fn pipeline_context() -> (TempDir, ToolContext, ConfigResetGuard) {
    pipeline_test_context(crate::util::json::OutputFormat::Json)
}

pub(super) fn dispatch_data(context: &ToolContext, pipeline_id: &str) -> Value {
    let output = pipeline(
        context,
        &json!({ "sessionId": pipeline_id, "dispatch": true }),
    )
    .expect("dispatch pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    response.get("data").cloned().expect("response data")
}

pub(super) fn read_pipeline_states(
    context: &ToolContext,
    pipeline_id: &str,
) -> HashMap<String, NodeState> {
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    frontmatter_value(&frontmatter.expect("pipeline frontmatter"), "nodeStates")
        .expect("node states")
}

pub(super) fn read_session(context: &ToolContext, session_id: &str) -> (Frontmatter, String) {
    let (frontmatter, content, _) =
        read_markdown(&workflow_session_uri(session_id), &context.config).expect("read session");
    (frontmatter.expect("session frontmatter"), content)
}

pub(super) use super::super::test_support::seed_pipeline_session_with_node_sessions;
