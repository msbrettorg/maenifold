use crate::error::GateError;
use crate::gate_definition::GateDefinition;
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
        let mut state = String::from("pending");
        let mut attempts = 0;

        for record in &document.records {
            match record.record_kind.as_str() {
                "append" => {}
                "verdict" => {
                    let verdict = record.verdict.as_deref().ok_or_else(|| {
                        GateError::TaskMetadataParse("verdict record missing Verdict".to_string())
                    })?;
                    let transition = match verdict {
                        "pass" => crate::state_machine::StateMachine::next_on_pass(&state, definition)?,
                        "reject" => crate::state_machine::StateMachine::next_on_reject(&state, definition)?,
                        "fail" => crate::state_machine::StateMachine::next_on_fail(&state, definition)?,
                        other => {
                            return Err(GateError::IllegalTransition {
                                state: state.clone(),
                                verdict: other.to_string(),
                            })
                        }
                    };

                    if transition.acting_stage != record.stage {
                        return Err(GateError::IllegalTransition {
                            state: state.clone(),
                            verdict: verdict.to_string(),
                        });
                    }

                    if transition.next_state == "pending" {
                        attempts += 1;
                    }
                    state = transition.next_state;
                }
                other => {
                    return Err(GateError::TaskMetadataParse(format!(
                        "unknown record kind: {other}"
                    )))
                }
            }
        }

        if location == "done" {
            if state != "done" {
                return Err(GateError::UnknownState(format!(
                    "task archived in done/ but effective state is {state}"
                )));
            }
        } else if state == "done" {
            return Err(GateError::UnknownState(
                "task remains in todo/ but effective state is done".to_string(),
            ));
        }

        Ok(EffectiveTaskState {
            state,
            attempts,
            location: location.to_string(),
        })
    }
}
