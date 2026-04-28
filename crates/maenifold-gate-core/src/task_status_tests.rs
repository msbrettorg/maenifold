use super::*;
use crate::gate_definition::GateDefinition;
use crate::task_metadata::TaskMetadata;

#[derive(Debug, Clone)]
struct TaskDocument {
    metadata: TaskMetadata,
    records: Vec<TaskStageRecord>,
}

#[derive(Debug, Clone)]
struct TaskStageRecord {
    stage: String,
    record_kind: String,
    outcome: Option<String>,
}

impl TaskStageRecordLike for TaskStageRecord {
    fn stage(&self) -> &str {
        &self.stage
    }

    fn record_kind(&self) -> &str {
        &self.record_kind
    }

    fn outcome(&self) -> Option<&str> {
        self.outcome.as_deref()
    }
}

impl TaskDocumentLike for TaskDocument {
    type Record = TaskStageRecord;

    fn metadata_state(&self) -> Option<&str> {
        self.metadata.state.as_deref()
    }

    fn records(&self) -> &[Self::Record] {
        &self.records
    }
}

fn sdd_definition() -> GateDefinition {
    GateDefinition::from_json_text(
        r#"{
        "schemaVersion": 1,
        "id": "sdd",
        "name": "Spec Driven Development",
        "initial": "pending",
        "states": {
            "pending": { "on": { "pass": "swe" } },
            "swe": { "on": { "fail": "pending", "pass": "red-team" } },
            "red-team": { "on": { "fail": "pending", "pass": "blue-team" } },
            "blue-team": { "on": { "fail": "pending", "pass": "done" } },
            "done": { "terminal": true }
        }
    }"#,
    )
    .expect("valid sdd definition")
}

fn document(state: Option<&str>, records: Vec<TaskStageRecord>) -> TaskDocument {
    TaskDocument {
        metadata: TaskMetadata {
            id: None,
            title: "Test task".to_string(),
            pipeline: "sdd".to_string(),
            state: state.map(str::to_string),
            session_id: None,
            depends_on: Vec::new(),
        },
        records,
    }
}

fn outcome(stage: &str, outcome_value: &str) -> TaskStageRecord {
    TaskStageRecord {
        stage: stage.to_string(),
        record_kind: "outcome".to_string(),
        outcome: Some(outcome_value.to_string()),
    }
}

fn append(stage: &str) -> TaskStageRecord {
    TaskStageRecord {
        stage: stage.to_string(),
        record_kind: "append".to_string(),
        outcome: None,
    }
}

#[test]
fn empty_records_yields_initial_state() {
    let derived = TaskStatusDeriver::derive(&document(None, vec![]), &sdd_definition(), "todo")
        .expect("derive should succeed");
    assert_eq!(derived.state, "pending");
    assert_eq!(derived.attempts, 0);
}

#[test]
fn single_pass_advances_to_next_stage() {
    let derived = TaskStatusDeriver::derive(
        &document(None, vec![outcome("pending", "pass")]),
        &sdd_definition(),
        "todo",
    )
    .expect("derive should succeed");
    assert_eq!(derived.state, "swe");
    assert_eq!(derived.attempts, 0);
}

#[test]
fn full_pass_chain_reaches_done() {
    let records = vec![
        outcome("pending", "pass"),
        outcome("swe", "pass"),
        outcome("red-team", "pass"),
        outcome("blue-team", "pass"),
    ];
    let derived = TaskStatusDeriver::derive(&document(None, records), &sdd_definition(), "done")
        .expect("derive should succeed");
    assert_eq!(derived.state, "done");
    assert_eq!(derived.attempts, 0);
}

#[test]
fn fail_returns_to_initial() {
    let records = vec![outcome("pending", "pass"), outcome("swe", "fail")];
    let derived = TaskStatusDeriver::derive(&document(None, records), &sdd_definition(), "todo")
        .expect("derive should succeed");
    assert_eq!(derived.state, "pending");
    assert_eq!(derived.attempts, 1);
}

#[test]
fn append_does_not_change_state() {
    let records = vec![outcome("pending", "pass"), append("swe")];
    let derived = TaskStatusDeriver::derive(&document(None, records), &sdd_definition(), "todo")
        .expect("derive should succeed");
    assert_eq!(derived.state, "swe");
    assert_eq!(derived.attempts, 0);
}

#[test]
fn attempt_increments_on_fail_from_non_initial() {
    let records = vec![
        outcome("pending", "pass"),
        outcome("swe", "fail"),
        outcome("pending", "pass"),
        outcome("swe", "fail"),
    ];
    let derived = TaskStatusDeriver::derive(&document(None, records), &sdd_definition(), "todo")
        .expect("derive should succeed");
    assert_eq!(derived.state, "pending");
    assert_eq!(derived.attempts, 2);
}

#[test]
fn done_location_requires_terminal_state() {
    let result = TaskStatusDeriver::derive(
        &document(None, vec![outcome("pending", "pass")]),
        &sdd_definition(),
        "done",
    );
    assert!(result.is_err());
}

#[test]
fn todo_location_rejects_terminal_state() {
    let records = vec![
        outcome("pending", "pass"),
        outcome("swe", "pass"),
        outcome("red-team", "pass"),
        outcome("blue-team", "pass"),
    ];
    let result = TaskStatusDeriver::derive(&document(None, records), &sdd_definition(), "todo");
    assert!(result.is_err());
}

#[test]
fn reopen_from_terminal_increments_attempts() {
    let records = vec![
        outcome("pending", "pass"),
        outcome("swe", "pass"),
        outcome("red-team", "pass"),
        outcome("blue-team", "pass"),
        outcome("done", "fail"),
    ];
    let derived = TaskStatusDeriver::derive(&document(None, records), &sdd_definition(), "todo")
        .expect("derive should succeed");
    assert_eq!(derived.state, "pending");
    assert_eq!(derived.attempts, 1);
}
