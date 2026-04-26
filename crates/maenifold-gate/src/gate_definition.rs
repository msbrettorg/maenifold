use std::ops::Deref;

use crate::error::GateError;

pub use maenifold_gate_core::gate_definition::GateState;

#[derive(Debug, Clone)]
pub struct GateDefinition(maenifold_gate_core::gate_definition::GateDefinition);

impl GateDefinition {
    pub fn from_json_text(text: &str) -> Result<Self, GateError> {
        Ok(Self(
            maenifold_gate_core::gate_definition::GateDefinition::from_json_text(text)?,
        ))
    }

    pub fn initial_state(&self) -> Result<&str, GateError> {
        Ok(self.0.initial_state()?)
    }

    pub fn find_state(&self, id: &str) -> Option<&GateState> {
        self.0.find_state(id)
    }

    pub fn ordered_stage_ids(&self) -> Result<Vec<String>, GateError> {
        Ok(self.0.ordered_stage_ids()?)
    }

    pub fn as_core(&self) -> &maenifold_gate_core::gate_definition::GateDefinition {
        &self.0
    }
}

impl Deref for GateDefinition {
    type Target = maenifold_gate_core::gate_definition::GateDefinition;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
