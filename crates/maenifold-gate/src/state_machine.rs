use crate::error::GateError;
use crate::gate_definition::{GateDefinition, GateStage, TransitionResult};

#[derive(Debug, Clone)]
pub struct StateMachine;

impl StateMachine {
    pub fn acting_stage_name(state: &str, definition: &GateDefinition) -> Result<String, GateError> {
        let (acting_stage, _) = Self::acting_stage(state, definition)?;
        Ok(acting_stage)
    }

    pub fn next_on_pass(state: &str, definition: &GateDefinition) -> Result<TransitionResult, GateError> {
        Self::transition(state, "pass", definition)
    }

    pub fn next_on_reject(state: &str, definition: &GateDefinition) -> Result<TransitionResult, GateError> {
        Self::transition(state, "reject", definition)
    }

    pub fn next_on_fail(state: &str, definition: &GateDefinition) -> Result<TransitionResult, GateError> {
        Self::transition(state, "fail", definition)
    }

    pub fn validate_verdict(state: &str, verdict: &str, definition: &GateDefinition) -> Result<(), GateError> {
        let (_, stage) = Self::acting_stage(state, definition)?;
        if stage.allows_verdict(verdict) {
            return Ok(());
        }

        Err(GateError::IllegalTransition {
            state: state.to_string(),
            verdict: verdict.to_string(),
        })
    }

    fn transition(
        state: &str,
        verdict: &str,
        definition: &GateDefinition,
    ) -> Result<TransitionResult, GateError> {
        let (acting_state, stage) = Self::acting_stage(state, definition)?;
        let next_state = stage
            .transitions
            .get(verdict)
            .cloned()
            .ok_or_else(|| GateError::IllegalTransition {
                state: state.to_string(),
                verdict: verdict.to_string(),
            })?;

        Ok(TransitionResult {
            acting_stage: acting_state,
            verdict: verdict.to_string(),
            next_state,
        })
    }

    fn acting_stage<'a>(
        state: &str,
        definition: &'a GateDefinition,
    ) -> Result<(String, &'a GateStage), GateError> {
        if state == "done" {
            return Err(GateError::IllegalTransition {
                state: "done".to_string(),
                verdict: "terminal".to_string(),
            });
        }

        if state == "pending" {
            let entry = definition.entry_stage()?;
            let stage = definition
                .find_stage(entry)
                .ok_or_else(|| GateError::UnknownState(entry.to_string()))?;
            return Ok((entry.to_string(), stage));
        }

        let stage = definition
            .find_stage(state)
            .ok_or_else(|| GateError::UnknownState(state.to_string()))?;
        Ok((state.to_string(), stage))
    }
}
