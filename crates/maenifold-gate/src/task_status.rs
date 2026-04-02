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
        let mut state = initial.clone();
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

                    if transition.next_state == initial {
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
