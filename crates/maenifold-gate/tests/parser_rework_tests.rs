use std::{fs, path::Path};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use maenifold_gate::{TaskStage, TaskStatus, parse_task_content};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::tempdir;

fn base_task() -> String {
    "# T-54.1 — Parser Rework Tolerance\n\n**Requirement**: FR-41\n".to_string()
}

fn parse_task(location: &str, body: &str) -> maenifold_gate::TaskState {
    let path = Path::new(location).join("T-54.1-parser-rework-tolerance.md");
    parse_task_content(path, body).expect("task should parse")
}

#[test]
fn duplicate_swe_headings_keep_last_attempt() {
    let content = format!(
        "{}\n## SWE Remediation\n\nInitial fix.\n\n## SWE Remediation (Attempt 2)\n\nSecond fix.\n",
        base_task()
    );

    let state = parse_task("todo", &content);

    assert_eq!(state.status, TaskStatus::Red);
    assert_eq!(state.attempts, 0);
    assert_eq!(state.stages.len(), 1);
    assert_eq!(state.stages[0].stage, TaskStage::Swe);
    assert_eq!(state.stages[0].heading_line, 9);
    assert_eq!(state.stages[0].verdict, None);
}

#[test]
fn re_audit_without_initial_verdict_becomes_blue_ready() {
    let content = format!(
        "{}\n## SWE Remediation\n\nFixed issues.\n\n## Red-team Audit\n\nNeeds another pass.\n\n## Red-team Audit (Re-audit)\n\n**Verdict**: PASS\n\nLooks good.\n",
        base_task()
    );

    let state = parse_task("todo", &content);

    assert_eq!(state.status, TaskStatus::Blue);
    assert_eq!(state.attempts, 1);
    assert_eq!(state.stages.len(), 2);
    assert_eq!(state.stages[1].stage, TaskStage::Red);
    assert_eq!(state.stages[1].verdict.map(|v| v.as_str()), Some("PASS"));
}

#[test]
fn out_of_order_swe_after_red_is_treated_as_rework_cycle() {
    let content = format!(
        "{}\n## SWE Remediation\n\nInitial fix.\n\n## Red-team Audit\n\n**Verdict**: REJECT\n\nFound regressions.\n\n## SWE Remediation (Attempt 2)\n\nAddressed red-team feedback.\n",
        base_task()
    );

    let state = parse_task("todo", &content);

    assert_eq!(state.status, TaskStatus::Red);
    assert_eq!(state.attempts, 1);
    assert_eq!(state.stages.len(), 1);
    assert_eq!(state.stages[0].stage, TaskStage::Swe);
    assert_eq!(state.stages[0].heading_line, 15);
}

#[test]
fn round_suffix_red_heading_is_recognized() {
    let content = format!(
        "{}\n## SWE Remediation\n\nImplemented fix.\n\n## Red-team Audit — Round 2\n\n**Verdict**: PASS\n\nApproved.\n",
        base_task()
    );

    let state = parse_task("todo", &content);

    assert_eq!(state.status, TaskStatus::Blue);
    assert_eq!(state.stages.len(), 2);
    assert_eq!(state.stages[1].stage, TaskStage::Red);
    assert_eq!(state.stages[1].verdict.map(|v| v.as_str()), Some("PASS"));
}

#[test]
fn emoji_prefixed_verdict_is_accepted() {
    let content = format!(
        "{}\n## SWE Remediation\n\nImplemented fix.\n\n## Red-team Audit\n\n**Verdict**: ❌ REJECT\n\nNeeds follow-up.\n",
        base_task()
    );

    let state = parse_task("todo", &content);

    assert_eq!(state.status, TaskStatus::Pending);
    assert_eq!(state.attempts, 1);
    assert_eq!(state.stages[1].verdict.map(|v| v.as_str()), Some("REJECT"));
}

#[test]
fn stale_blue_section_before_reaudit_pass_is_ignored() {
    let content = format!(
        "{}\n## SWE Remediation\n\nInitial fix.\n\n## Red-team Audit\n\n**Verdict**: REJECT\n\nNeeds changes.\n\n## Blue-team Verification\n\n**Verdict**: PASS\n\nStale verification from prior cycle.\n\n## SWE Remediation (Attempt 2)\n\nAddressed feedback.\n\n## Red-team Audit (Re-audit)\n\n**Verdict**: PASS\n\nReady for blue.\n",
        base_task()
    );

    let state = parse_task("todo", &content);

    assert_eq!(state.status, TaskStatus::Blue);
    assert_eq!(state.attempts, 1);
    assert_eq!(state.stages.len(), 2);
    assert_eq!(state.stages[1].stage, TaskStage::Red);
    assert_eq!(state.stages[1].verdict.map(|v| v.as_str()), Some("PASS"));
}

#[test]
fn done_file_missing_last_blue_verdict_gets_implicit_pass() {
    let content = format!(
        "{}\n## SWE Remediation\n\nImplemented fix.\n\n## Red-team Audit\n\n**Verdict**: PASS\n\nApproved.\n\n## Blue-team Verification\n\nFinal verification complete.\n",
        base_task()
    );

    let state = parse_task("done", &content);

    assert_eq!(state.status, TaskStatus::Done);
    assert_eq!(state.stages.len(), 3);
    assert_eq!(state.stages[2].stage, TaskStage::Blue);
    assert_eq!(state.stages[2].verdict.map(|v| v.as_str()), Some("PASS"));
}

#[test]
fn todo_file_missing_last_red_verdict_emits_warning() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("todo")).expect("create todo");
    fs::create_dir_all(root.path().join("done")).expect("create done");

    let file = "T-54.1-parser-rework-tolerance.md";
    let path = root.path().join("todo").join(file);
    let content = format!(
        "{}\n## SWE Remediation\n\nImplemented fix.\n\n## Red-team Audit\n\nAwaiting verdict.\n",
        base_task()
    );
    fs::write(&path, content).expect("write task");

    let gate_bin = std::env::var_os("CARGO_BIN_EXE_gate");
    let mut sync_command: Command = if gate_bin.is_some() {
        Command::new(gate_bin.clone().expect("gate binary"))
    } else {
        cargo_bin_cmd!("gate")
    };
    sync_command.arg("sync").arg(root.path()).assert().success();

    let mut command: Command = if let Some(path) = gate_bin {
        Command::new(path)
    } else {
        cargo_bin_cmd!("gate")
    };

    command
        .args(["status", "--root"])
        .arg(root.path())
        .arg("T-54.1")
        .assert()
        .success()
        .stderr(contains("warning:").and(contains("missing verdict for red section")));
}

#[test]
fn done_file_with_only_red_blue_sections_parses_as_done() {
    let content = format!(
        "{}\n## Red-team Audit\n\n**Verdict**: PASS\n\nApproved.\n\n## Blue-team Verification\n\n**Verdict**: PASS\n\nVerified.\n",
        base_task()
    );

    let state = parse_task("done", &content);

    assert_eq!(state.status, TaskStatus::Done);
    assert_eq!(state.stages.len(), 2);
    assert_eq!(state.stages[0].stage, TaskStage::Red);
    assert_eq!(state.stages[0].verdict.map(|v| v.as_str()), Some("PASS"));
    assert_eq!(state.stages[1].stage, TaskStage::Blue);
    assert_eq!(state.stages[1].verdict.map(|v| v.as_str()), Some("PASS"));
}
