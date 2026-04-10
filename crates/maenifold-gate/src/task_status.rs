use crate::error::GateError;
use crate::gate_definition::GateDefinition;
use crate::state_machine::StateMachine;
use crate::task_file::TaskDocument;
use std::collections::HashSet;

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
                    maybe_seed_recorded_stage(&mut state, record.stage.as_str(), definition)?;
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

/// Tasks may have work-log histories that omit one or more PASS transitions
/// before the next recorded outcome. This happens for several reasons:
///
/// * Legacy tasks recorded before every transition was written explicitly.
/// * Human-edited backlog histories that summarize a later review/result
///   without the intermediate PASS record.
/// * Historical tooling bugs that skipped stage-advance records.
///
/// To keep those histories replayable, seed the cursor forward along the
/// definition's PASS path until it reaches the recorded stage, if reachable.
fn maybe_seed_recorded_stage(
    state: &mut String,
    recorded_stage: &str,
    definition: &GateDefinition,
) -> Result<(), GateError> {
    if *state == recorded_stage {
        return Ok(());
    }

    if definition.find_state(recorded_stage).is_none() {
        return Ok(());
    }

    let mut cursor = state.clone();
    let mut visited = HashSet::new();
    while visited.insert(cursor.clone()) {
        let Ok(transition) = StateMachine::next_on_pass(&cursor, definition) else {
            break;
        };
        if transition.next_state == recorded_stage {
            *state = recorded_stage.to_string();
            break;
        }
        cursor = transition.next_state;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::TaskStatusDeriver;
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
        .expect("valid definition")
    }

    fn task_document(records: Vec<TaskStageRecord>) -> TaskDocument {
        TaskDocument {
            metadata: TaskMetadata {
                id: Some("T-90.1".to_string()),
                title: "Legacy replay".to_string(),
                pipeline: "sdd".to_string(),
                state: Some("pending".to_string()),
            },
            records,
        }
    }

    fn outcome(stage: &str, outcome: &str, recorded_at: &str) -> TaskStageRecord {
        TaskStageRecord {
            stage: stage.to_string(),
            record_kind: "outcome".to_string(),
            outcome: Some(outcome.to_string()),
            agent: "test-agent".to_string(),
            model: None,
            recorded_at: recorded_at.to_string(),
            notes: "regression coverage".to_string(),
        }
    }

    #[test]
    fn legacy_done_history_without_pending_bootstrap_replays_to_done() {
        let document = task_document(vec![
            outcome("swe", "pass", "2025-01-01T00:00:00Z"),
            outcome("red-team", "pass", "2025-01-01T00:05:00Z"),
            outcome("blue-team", "pass", "2025-01-01T00:10:00Z"),
        ]);

        let effective = TaskStatusDeriver::derive(&document, &sdd_definition(), "done")
            .expect("legacy replay should succeed");

        assert_eq!(effective.state, "done");
        assert_eq!(effective.attempts, 0);
    }

    #[test]
    fn legacy_entry_fail_then_pass_tracks_retry_and_progress() {
        let document = task_document(vec![
            outcome("swe", "fail", "2025-01-01T00:00:00Z"),
            outcome("swe", "pass", "2025-01-01T00:05:00Z"),
        ]);

        let effective = TaskStatusDeriver::derive(&document, &sdd_definition(), "todo")
            .expect("legacy retry replay should succeed");

        assert_eq!(effective.state, "red-team");
        assert_eq!(effective.attempts, 1);
    }

    #[test]
    fn replay_tolerates_missing_intermediate_pass_record() {
        let document = task_document(vec![
            outcome("swe", "pass", "2025-01-01T00:00:00Z"),
            outcome("blue-team", "pass", "2025-01-01T00:10:00Z"),
        ]);

        let effective = TaskStatusDeriver::derive(&document, &sdd_definition(), "done")
            .expect("forward seeding should tolerate missing red-team pass");

        assert_eq!(effective.state, "done");
        assert_eq!(effective.attempts, 0);
    }
}
