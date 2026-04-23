#![cfg(test)]

use super::submachine::{
    HostSessionType, ResumeSubmachineResult, SubmachineType, parse_submachine_type,
};
use super::submachine_gate::find_gate_task_status;
use super::submachine_register::{maybe_resume_submachine, register_host_submachine};
use crate::config::Config;
use crate::markdown::{Frontmatter, read_markdown, write_markdown};
use crate::sessions::frontmatter_string;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use tempfile::TempDir;

fn test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    (
        tmp,
        ToolContext::new(config, OutputFormat::Markdown),
        ConfigResetGuard,
    )
}

fn write_session(
    context: &ToolContext,
    session_type: HostSessionType,
    session_id: &str,
    parent: Option<&str>,
) {
    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String(format!("{} {}", session_type.as_str(), session_id)),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "phase".to_string(),
        YamlValue::String("running".to_string()),
    );
    frontmatter.insert(
        "activeSubmachineType".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineSessionId".to_string(),
        YamlValue::String(String::new()),
    );
    frontmatter.insert(
        "activeSubmachineTarget".to_string(),
        YamlValue::String(String::new()),
    );
    if let Some(parent) = parent {
        frontmatter.insert("parent".to_string(), YamlValue::String(parent.to_string()));
    }
    write_markdown(
        &session_type.session_uri(session_id),
        Some(&frontmatter),
        "# Session\n",
        &context.config,
    )
    .expect("write session");
}

#[test]
fn parse_submachine_type_defaults_to_sequential() {
    assert_eq!(
        parse_submachine_type(None).expect("default type"),
        SubmachineType::Sequential
    );
}

#[test]
fn gate_task_status_uses_done_directory_as_terminal() {
    let (tmp, context, _guard) = test_context();
    let done_dir = tmp.path().join("done");
    std::fs::create_dir_all(&done_dir).expect("create done dir");
    std::fs::create_dir_all(tmp.path().join("todo")).expect("create todo dir");
    std::fs::write(
        done_dir.join("T-108.1-example.md"),
        "---\nstate: blue-team\n---\n",
    )
    .expect("write gate task");

    let status = find_gate_task_status(&context, "T-108.1", None)
        .expect("gate status")
        .expect("task found");
    assert!(status.terminal);
    assert_eq!(status.effective_state, "done");
    assert!(status.path.ends_with("done/T-108.1-example.md"));
}

#[test]
fn register_host_submachine_rejects_workflow_cycles() {
    let (_tmp, context, _guard) = test_context();
    let host_session_id = "workflow-1735689600001";
    let child_session_id = "workflow-1735689600000";
    write_session(&context, HostSessionType::Workflow, child_session_id, None);
    write_session(
        &context,
        HostSessionType::Workflow,
        host_session_id,
        Some(&HostSessionType::Workflow.link(child_session_id)),
    );

    let mut host_frontmatter = read_markdown(
        &HostSessionType::Workflow.session_uri(host_session_id),
        &context.config,
    )
    .expect("read host")
    .0
    .expect("host frontmatter");
    let error = register_host_submachine(
        &context,
        HostSessionType::Workflow,
        host_session_id,
        &HostSessionType::Workflow.session_uri(host_session_id),
        &mut host_frontmatter,
        child_session_id,
        SubmachineType::Workflow,
    )
    .expect_err("cycle should be rejected");
    assert!(error.to_string().contains("create a cycle"));
}

#[test]
fn gate_submachine_resume_uses_persisted_target_path() {
    let (tmp, context, _guard) = test_context();
    std::fs::create_dir_all(tmp.path().join("todo")).expect("create todo dir");
    std::fs::create_dir_all(tmp.path().join("done")).expect("create done dir");
    std::fs::create_dir_all(tmp.path().join("nested/todo")).expect("create nested todo dir");
    std::fs::create_dir_all(tmp.path().join("nested/done")).expect("create nested done dir");

    let authoritative_task = tmp.path().join("todo/T-108.1-authoritative.md");
    std::fs::write(&authoritative_task, "---\nstate: blue-team\n---\n").expect("write task");
    std::fs::write(
        tmp.path().join("nested/done/T-108.1-conflict.md"),
        "---\nstate: done\n---\n",
    )
    .expect("write conflicting task");

    let host_session_id = "workflow-1735689600001";
    write_session(&context, HostSessionType::Workflow, host_session_id, None);
    let host_uri = HostSessionType::Workflow.session_uri(host_session_id);
    let (frontmatter, _, _) = read_markdown(&host_uri, &context.config).expect("read host");
    let mut frontmatter = frontmatter.expect("host frontmatter");

    register_host_submachine(
        &context,
        HostSessionType::Workflow,
        host_session_id,
        &host_uri,
        &mut frontmatter,
        "T-108.1",
        SubmachineType::Gate,
    )
    .expect("register gate");

    let moved_task = tmp.path().join("done/T-108.1-authoritative.md");
    std::fs::rename(&authoritative_task, &moved_task).expect("move authoritative task");

    let (frontmatter, existing_content, _) =
        read_markdown(&host_uri, &context.config).expect("read host after register");
    let frontmatter = frontmatter.expect("frontmatter after register");
    assert_eq!(
        frontmatter_string(&frontmatter, "activeSubmachineTarget"),
        authoritative_task.to_string_lossy()
    );

    let resumed = maybe_resume_submachine(
        &context,
        HostSessionType::Workflow,
        &host_uri,
        frontmatter,
        &existing_content,
    )
    .expect("resume");

    match resumed {
        ResumeSubmachineResult::Ready(updated_frontmatter) => {
            assert_eq!(frontmatter_string(&updated_frontmatter, "phase"), "running");
            assert_eq!(
                frontmatter_string(&updated_frontmatter, "activeSubmachineTarget"),
                ""
            );
        }
        ResumeSubmachineResult::Waiting(message) => {
            panic!("expected resumed state, got waiting: {message}");
        }
    }
}
