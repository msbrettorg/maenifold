use crate::{error::GateError, gate_definition::GateDefinition, state_machine::StateMachine};

pub trait TaskStageRecordLike {
    fn stage(&self) -> &str;
    fn record_kind(&self) -> &str;
    fn outcome(&self) -> Option<&str>;
}

pub trait TaskDocumentLike {
    type Record: TaskStageRecordLike;

    fn metadata_state(&self) -> Option<&str>;
    fn records(&self) -> &[Self::Record];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveTaskState {
    pub state: String,
    pub attempts: u32,
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct TaskStatusDeriver;

impl TaskStatusDeriver {
    pub fn derive<D>(
        document: &D,
        definition: &GateDefinition,
        location: &str,
    ) -> Result<EffectiveTaskState, GateError>
    where
        D: TaskDocumentLike,
    {
        let initial = definition.initial_state()?.to_string();
        let records = document.records();
        let mut state = if records.is_empty() {
            document
                .metadata_state()
                .map(str::to_string)
                .unwrap_or_else(|| initial.clone())
        } else {
            initial.clone()
        };
        let mut attempts = 0;
        for record in records {
            match record.record_kind() {
                "append" => {}
                "cancel" => {
                    // cancel bypasses the state machine and terminally marks the task as cancelled.
                    state = "cancelled".to_string();
                }
                "reopen" => {
                    attempts += 1;
                    state = initial.clone();
                }
                "outcome" => {
                    let outcome = record.outcome().ok_or_else(|| {
                        GateError::TaskMetadataParse("outcome record missing Outcome".to_string())
                    })?;
                    let current = definition
                        .find_state(&state)
                        .ok_or_else(|| GateError::UnknownState(state.clone()))?;
                    if current.terminal {
                        if outcome != "fail" || record.stage() != state {
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

                    if transition.acting_stage != record.stage() {
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

        if state == "cancelled" {
            if location != "done" {
                return Err(GateError::UnknownState(
                    "cancelled task must reside in done/".to_string(),
                ));
            }
            return Ok(EffectiveTaskState {
                state,
                attempts,
                location: location.to_string(),
            });
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
#[path = "task_status_tests.rs"]
mod tests;
