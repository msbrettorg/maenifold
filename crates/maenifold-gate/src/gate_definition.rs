use std::collections::HashSet;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::GateError;

#[derive(Debug, Clone)]
pub struct GateDefinition {
    pub initial: String,
    pub states: IndexMap<String, GateState>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GateDefinitionFile {
    schema_version: u32,
    id: String,
    name: String,
    initial: String,
    states: IndexMap<String, GateState>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GateState {
    #[serde(default)]
    pub terminal: bool,
    #[serde(default)]
    pub on: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionResult {
    pub acting_stage: String,
    pub outcome: String,
    pub next_state: String,
}

#[derive(Debug, Clone)]
pub struct GateDefinitionValidator;

impl GateDefinition {
    pub fn from_json_text(text: &str) -> Result<Self, GateError> {
        let file: GateDefinitionFile = serde_json::from_str(text)?;
        if file.schema_version != 1 {
            return Err(GateError::TaskMetadataParse(format!(
                "unsupported gate schemaVersion {}",
                file.schema_version
            )));
        }
        if file.id.trim().is_empty() {
            return Err(GateError::TaskMetadataParse(
                "gate definition id must not be empty".to_string(),
            ));
        }
        if file.name.trim().is_empty() {
            return Err(GateError::TaskMetadataParse(
                "gate definition name must not be empty".to_string(),
            ));
        }

        let definition = Self {
            initial: file.initial,
            states: file.states,
        };
        GateDefinitionValidator::validate(&definition)?;
        Ok(definition)
    }

    pub fn initial_state(&self) -> Result<&str, GateError> {
        if self.states.contains_key(self.initial.as_str()) {
            Ok(self.initial.as_str())
        } else {
            Err(GateError::UnknownState(self.initial.clone()))
        }
    }

    pub fn find_state(&self, id: &str) -> Option<&GateState> {
        self.states.get(id)
    }

    pub fn ordered_stage_ids(&self) -> Result<Vec<String>, GateError> {
        self.initial_state()?;
        Ok(self.states.keys().cloned().collect())
    }
}

impl GateDefinitionValidator {
    pub fn validate(definition: &GateDefinition) -> Result<(), GateError> {
        let mut seen = HashSet::new();
        if !definition.states.contains_key(definition.initial.as_str()) {
            return Err(GateError::UnknownState(definition.initial.clone()));
        }

        for (state_id, state) in &definition.states {
            if !seen.insert(state_id.as_str()) {
                return Err(GateError::DuplicateStageName(state_id.clone()));
            }

            if state.terminal && !state.on.is_empty() {
                return Err(GateError::TaskMetadataParse(format!(
                    "terminal state '{state_id}' cannot declare transitions"
                )));
            }

            for target in state.on.values() {
                if !definition.states.contains_key(target.as_str()) {
                    return Err(GateError::UnknownTransitionTarget {
                        stage: state_id.clone(),
                        target: target.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::GateDefinition;

    #[test]
    fn ordered_stage_ids_follow_pass_path() {
        let definition = GateDefinition::from_json_text(
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
        .expect("valid definition");

        assert_eq!(
            definition.ordered_stage_ids().expect("stage order"),
            vec!["pending", "swe", "red-team", "blue-team", "done"]
        );
    }
}
