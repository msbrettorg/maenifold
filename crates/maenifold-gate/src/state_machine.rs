use crate::{error::GateError, gate_definition::GateDefinition};

pub use maenifold_gate_core::gate_definition::TransitionResult;

#[derive(Debug, Clone)]
pub struct StateMachine;

impl StateMachine {
    pub fn acting_stage_name(
        state: &str,
        definition: &GateDefinition,
    ) -> Result<String, GateError> {
        Ok(
            maenifold_gate_core::state_machine::StateMachine::acting_stage_name(
                state,
                definition.as_core(),
            )?,
        )
    }

    pub fn next_on_pass(
        state: &str,
        definition: &GateDefinition,
    ) -> Result<TransitionResult, GateError> {
        Ok(maenifold_gate_core::state_machine::StateMachine::next_on_pass(
            state,
            definition.as_core(),
        )?)
    }

    pub fn next_on_fail(
        state: &str,
        definition: &GateDefinition,
    ) -> Result<TransitionResult, GateError> {
        Ok(maenifold_gate_core::state_machine::StateMachine::next_on_fail(
            state,
            definition.as_core(),
        )?)
    }
}
