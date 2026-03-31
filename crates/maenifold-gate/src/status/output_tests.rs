use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::{
    db::TaskRow,
    types::{StageState, StageVerdict, TaskLocation},
};

fn state(status: &str, stages: Vec<StageState>) -> TaskState {
    TaskState::new(
        PathBuf::from("todo/T-42.1-sample.md"),
        TaskLocation::Todo,
        status.to_string(),
        3,
        stages,
    )
}

fn task_row(status: &str, location: &str) -> TaskRow {
    TaskRow {
        id: "T-42.1".into(),
        file: "T-42.1-sample.md".into(),
        title: Some("Sample".into()),
        requirement: "FR-42".into(),
        status: status.into(),
        location: location.into(),
        attempts: 3,
    }
}

#[test]
fn json_output_serializes_expected_structure() {
    let state = state(
        "red",
        vec![
            StageState {
                stage: "swe".into(),
                heading_line: 1,
                verdict: Some(StageVerdict::Pass),
            },
            StageState {
                stage: "red".into(),
                heading_line: 7,
                verdict: None,
            },
        ],
    );

    let output = json_output(
        "T-42.1-sample.md",
        "todo",
        "sdd",
        Some(&state),
        Some("# T-42.1 — Sample Task\n\n**Requirement**: FR-42\n\n## SWE\n\nDone.\n"),
        None,
        vec!["db status=pending but file status=red".into()],
    );

    let value = serde_json::to_value(output).expect("status output serializes");
    assert_eq!(value["id"], "T-42.1");
    assert_eq!(value["title"], "Sample Task");
    assert_eq!(value["file"], "T-42.1-sample.md");
    assert_eq!(value["location"], "todo");
    assert_eq!(value["status"], "red");
    assert_eq!(value["pipeline"], "sdd");
    assert_eq!(value["attempts"], 3);
    assert_eq!(value["requirement"], "FR-42");
    assert_eq!(
        value["stages"],
        json!([
            {"name": "swe", "verdict": "PASS"},
            {"name": "red", "verdict": null}
        ])
    );
    assert_eq!(value["sections"][0]["heading"], "SWE");
    assert_eq!(
        value["db_drift"],
        json!(["db status=pending but file status=red"])
    );
}

#[test]
fn print_summary_handles_zero_tasks() {
    assert_eq!(
        render_summary(&BTreeMap::new()),
        "STATUS   COUNT\n---\ntotal        0\n"
    );
}

#[test]
fn print_summary_formats_multiple_statuses_and_total() {
    let counts = BTreeMap::from([
        ("blue".to_string(), 12usize),
        ("pending".to_string(), 3usize),
        ("red".to_string(), 1usize),
    ]);

    assert_eq!(
        render_summary(&counts),
        "STATUS   COUNT\nblue        12\npending      3\nred          1\n---\ntotal       16\n"
    );
}

#[test]
fn print_summary_lists_errors_last() {
    let counts = BTreeMap::from([
        ("error".to_string(), 2usize),
        ("pending".to_string(), 1usize),
    ]);

    assert_eq!(
        render_summary(&counts),
        "STATUS   COUNT\npending      1\nerror        2\n---\ntotal        3\n"
    );
}

#[test]
fn discovered_stages_preserves_first_seen_order() {
    let state = state(
        "blue",
        vec![
            StageState {
                stage: "swe".into(),
                heading_line: 1,
                verdict: Some(StageVerdict::Pass),
            },
            StageState {
                stage: "red".into(),
                heading_line: 7,
                verdict: Some(StageVerdict::Reject),
            },
            StageState {
                stage: "swe".into(),
                heading_line: 12,
                verdict: Some(StageVerdict::Pass),
            },
        ],
    );

    assert_eq!(discovered_stages(&state), vec!["swe", "red"]);
}

#[test]
fn maybe_drift_returns_empty_when_check_disabled() {
    let state = state("red", vec![]);
    let row = task_row("pending", "done");

    assert!(maybe_drift(false, &state, Some(&row)).is_empty());
}

#[test]
fn maybe_drift_detects_status_and_location_changes() {
    let state = state("red", vec![]);
    let row = task_row("pending", "done");

    assert_eq!(
        maybe_drift(true, &state, Some(&row)),
        vec![
            "db status=pending but file status=red".to_string(),
            "db location=done but file location=todo".to_string()
        ]
    );
}

#[test]
fn maybe_drift_reports_missing_db_row() {
    let state = state("pending", vec![]);
    assert_eq!(
        maybe_drift(true, &state, None),
        vec!["missing from db cache".to_string()]
    );
}

#[test]
fn stage_value_returns_verdict_when_present() {
    let state = state(
        "red",
        vec![StageState {
            stage: "red".into(),
            heading_line: 3,
            verdict: Some(StageVerdict::Reject),
        }],
    );

    assert_eq!(stage_value(&state, "red"), Some(Some("REJECT")));
    assert_eq!(stage_text(&state, "red"), "REJECT");
}

#[test]
fn stage_value_distinguishes_pending_stage_from_missing_stage() {
    let state = state(
        "red",
        vec![StageState {
            stage: "swe".into(),
            heading_line: 3,
            verdict: None,
        }],
    );

    assert_eq!(stage_value(&state, "swe"), Some(None));
    assert_eq!(stage_text(&state, "swe"), "yes");
    assert_eq!(stage_value(&state, "blue"), None);
    assert_eq!(stage_text(&state, "blue"), "no");
}
