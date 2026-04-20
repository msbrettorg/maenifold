use crate::error::GateError;
use crate::gate_definition::GateDefinition;
use crate::state_machine::StateMachine;
use crate::task_file::TaskDocument;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveTaskState {
    pub state: String,
    pub attempts: u32,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct TaskStatusDeriver;

impl TaskStatusDeriver {
    pub fn derive(
        document: &TaskDocument,
        definition: &GateDefinition,
        location: &str,
    ) -> Result<EffectiveTaskState, GateError> {
        let initial = definition.initial_state()?.to_string();
        let mut state = if document.records.is_empty() {
            document
                .metadata
                .state
                .clone()
                .unwrap_or_else(|| initial.clone())
        } else {
            initial.clone()
        };
        let mut attempts = 0;

        for record in &document.records {
            match record.record_kind.as_str() {
                "append" => {}
                "outcome" => {
                    let outcome = record.outcome.as_deref().ok_or_else(|| {
                        GateError::TaskMetadataParse("outcome record missing Outcome".to_string())
                    })?;
                    let current = definition
                        .find_state(&state)
                        .ok_or_else(|| GateError::UnknownState(state.clone()))?;
                    if current.terminal {
                        if outcome != "fail" || record.stage != state {
                            return Err(GateError::IllegalTransition {
                                state: state.clone(),
                                outcome: outcome.to_string(),
                            });
                        }
                        attempts += 1;
                        state = initial.clone();
                        continue;
                    }
                    let transition = match outcome {
                        "pass" => StateMachine::next_on_pass(&state, definition)?,
                        "fail" => StateMachine::next_on_fail(&state, definition)?,
                        other => {
                            return Err(GateError::IllegalTransition {
                                state: state.clone(),
                                outcome: other.to_string(),
                            });
                        }
                    };

                    if transition.acting_stage != record.stage {
                        return Err(GateError::IllegalTransition {
                            state: state.clone(),
                            outcome: outcome.to_string(),
                        });
                    }

                    if transition.next_state == initial && state != initial {
                        attempts += 1;
                    }
                    state = transition.next_state;
                }
                other => {
                    return Err(GateError::TaskMetadataParse(format!(
                        "unknown record kind: {other}"
                    )));
                }
            }
        }

        if location == "done" {
            let done = definition
                .find_state(&state)
                .ok_or_else(|| GateError::UnknownState(state.clone()))?;
            if !done.terminal {
                return Err(GateError::UnknownState(format!(
                    "task archived in done/ but effective state is {state}"
                )));
            }
        } else if definition
            .find_state(&state)
            .ok_or_else(|| GateError::UnknownState(state.clone()))?
            .terminal
        {
            return Err(GateError::UnknownState(
                "task remains in todo/ but effective state is terminal".to_string(),
            ));
        }

        Ok(EffectiveTaskState {
            state,
            attempts,
            location: location.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate_definition::GateDefinition;
    use crate::task_file::TaskDocument;
    use crate::task_metadata::TaskMetadata;
    use crate::task_sections::TaskStageRecord;

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
            },
            records,
        }
    }

    fn outcome(stage: &str, outcome_value: &str) -> TaskStageRecord {
        TaskStageRecord {
            stage: stage.to_string(),
            record_kind: "outcome".to_string(),
            outcome: Some(outcome_value.to_string()),
            agent: "test".to_string(),
            model: None,
            recorded_at: "2026-01-01T00:00:00Z".to_string(),
            notes: "test".to_string(),
        }
    }

    fn append(stage: &str) -> TaskStageRecord {
        TaskStageRecord {
            stage: stage.to_string(),
            record_kind: "append".to_string(),
            outcome: None,
            agent: "test".to_string(),
            model: None,
            recorded_at: "2026-01-01T00:00:00Z".to_string(),
            notes: "progress".to_string(),
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
}
