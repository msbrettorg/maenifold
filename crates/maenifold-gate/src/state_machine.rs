use crate::error::GateError;
use crate::gate_definition::{GateDefinition, GateState, TransitionResult};

#[derive(Debug, Clone)]
pub struct StateMachine;

impl StateMachine {
    pub fn acting_stage_name(state: &str, definition: &GateDefinition) -> Result<String, GateError> {
        let (acting_stage, _) = Self::acting_state(state, definition)?;
        Ok(acting_stage)
    }

    pub fn next_on_pass(state: &str, definition: &GateDefinition) -> Result<TransitionResult, GateError> {
        Self::transition(state, "pass", definition)
    }

    pub fn next_on_fail(state: &str, definition: &GateDefinition) -> Result<TransitionResult, GateError> {
        Self::transition(state, "fail", definition)
    }

    pub fn validate_outcome(state: &str, outcome: &str, definition: &GateDefinition) -> Result<(), GateError> {
        let (_, gate_state) = Self::acting_state(state, definition)?;
        if gate_state.allows_outcome(outcome) {
            Ok(())
        } else {
            Err(GateError::IllegalTransition {
                state: state.to_string(),
                outcome: outcome.to_string(),
            })
        }
    }

    fn transition(state: &str, outcome: &str, definition: &GateDefinition) -> Result<TransitionResult, GateError> {
        let (acting_stage, gate_state) = Self::acting_state(state, definition)?;
        let next_state = gate_state
            .on
            .get(outcome)
            .cloned()
            .ok_or_else(|| GateError::IllegalTransition {
                state: state.to_string(),
                outcome: outcome.to_string(),
            })?;

        Ok(TransitionResult {
            acting_stage,
            outcome: outcome.to_string(),
            next_state,
        })
    }

    fn acting_state<'a>(
        state: &str,
        definition: &'a GateDefinition,
    ) -> Result<(String, &'a GateState), GateError> {
        let initial = definition.initial_state()?;
        let normalized_state = if state.is_empty() { initial } else { state };
        let gate_state = definition
            .find_state(normalized_state)
            .ok_or_else(|| GateError::UnknownState(normalized_state.to_string()))?;

        if gate_state.terminal {
            return Err(GateError::IllegalTransition {
                state: normalized_state.to_string(),
                outcome: "terminal".to_string(),
            });
        }

        if normalized_state == initial {
            let entry_state = gate_state
                .on
                .get("pass")
                .ok_or_else(|| {
                    GateError::TaskMetadataParse(format!(
                        "initial state '{initial}' must declare an on.pass transition"
                    ))
                })?
                .clone();
            let entry = definition
                .find_state(&entry_state)
                .ok_or_else(|| GateError::UnknownState(entry_state.clone()))?;
            if entry.terminal {
                return Err(GateError::TaskMetadataParse(format!(
                    "initial state '{initial}' cannot dispatch directly to terminal state '{entry_state}'"
                )));
            }
            return Ok((entry_state, entry));
        }

        Ok((normalized_state.to_string(), gate_state))
    }
}
