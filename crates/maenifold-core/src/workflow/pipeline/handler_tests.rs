pub(super) use super::session_io::{frontmatter_value, write_pipeline_session};
pub(super) use super::test_support::*;
pub(super) use super::{NodeState, NodeStatus, PipelineDefinition, pipeline};
pub(super) use crate::config::Config;
pub(super) use crate::markdown::{delete_file, file_exists, read_markdown, write_markdown};
pub(super) use crate::registry::ToolRegistry;
pub(super) use crate::sessions::{pipeline_session_uri, workflow_session_uri};
pub(super) use crate::tools::ToolContext;
pub(super) use crate::util::json::OutputFormat;
pub(super) use serde_json::{Value, json};
pub(super) use serde_yaml::Value as YamlValue;
pub(super) use serial_test::serial;
pub(super) use std::collections::{BTreeMap, HashMap};
pub(super) use std::fs;

#[path = "handler_tests_create.rs"]
mod create;
#[path = "handler_tests_create_misc.rs"]
mod create_misc;
#[path = "handler_tests_crud.rs"]
mod crud;
#[path = "handler_tests_fork_a.rs"]
mod fork_a;
#[path = "handler_tests_fork_b.rs"]
mod fork_b;
#[path = "handler_tests_fork_context.rs"]
mod fork_context;
#[path = "handler_tests_graft_a.rs"]
mod graft_a;
#[path = "handler_tests_graft_b.rs"]
mod graft_b;
#[path = "handler_tests_graft_c.rs"]
mod graft_c;
#[path = "handler_tests_graft_d.rs"]
mod graft_d;
#[path = "handler_tests_join.rs"]
mod join;
#[path = "handler_tests_join_errors.rs"]
mod join_errors;
#[path = "handler_tests_rewind.rs"]
mod rewind;
#[path = "handler_tests_routing.rs"]
mod routing;

pub(super) fn set_pipeline_status(context: &ToolContext, session_id: &str, status: &str) {
    let uri = pipeline_session_uri(session_id);
    let (frontmatter, content, _) = read_markdown(&uri, &context.config).expect("read pipeline");
    let mut frontmatter = frontmatter.expect("pipeline frontmatter");
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    write_markdown(&uri, Some(&frontmatter), &content, &context.config).expect("write pipeline");
}
