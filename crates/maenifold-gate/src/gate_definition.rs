use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use crate::error::GateError;

#[derive(Debug, Clone)]
pub struct GateDefinition {
    pub name: String,
    pub digest: String,
    pub stages: Vec<GateStage>,
}

#[derive(Debug, Clone, Deserialize)]
struct GateDefinitionFile {
    name: String,
    stages: Vec<GateStage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GateStage {
    pub name: String,
    #[serde(default)]
    pub verdicts: Vec<String>,
    #[serde(default)]
    pub transitions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionResult {
    pub acting_stage: String,
    pub verdict: String,
    pub next_state: String,
}

#[derive(Debug, Clone)]
pub struct GateDefinitionValidator;

impl GateDefinition {
    pub fn from_json_text(text: &str, digest: String) -> Result<Self, GateError> {
        let file: GateDefinitionFile = serde_json::from_str(text)?;
        let definition = Self {
            name: file.name,
            digest,
            stages: file.stages,
        };
        GateDefinitionValidator::validate(&definition)?;
        Ok(definition)
    }

    pub fn entry_stage(&self) -> Result<&str, GateError> {
        self.stages
            .first()
            .map(|stage| stage.name.as_str())
            .ok_or_else(|| GateError::TaskMetadataParse("pipeline has no stages".to_string()))
    }

    pub fn find_stage(&self, name: &str) -> Option<&GateStage> {
        self.stages.iter().find(|stage| stage.name == name)
    }
}

impl GateStage {
    pub fn allows_verdict(&self, verdict: &str) -> bool {
        self.transitions.contains_key(verdict) || self.verdicts.iter().any(|value| value == verdict)
    }
}

impl GateDefinitionValidator {
    pub fn validate(definition: &GateDefinition) -> Result<(), GateError> {
        let mut seen = HashSet::new();
        let valid_targets: HashSet<&str> = definition
            .stages
            .iter()
            .map(|stage| stage.name.as_str())
            .chain(std::iter::once("pending"))
            .chain(std::iter::once("done"))
            .collect();

        for stage in &definition.stages {
            if !seen.insert(stage.name.as_str()) {
                return Err(GateError::DuplicateStageName(stage.name.clone()));
            }

            for target in stage.transitions.values() {
                if !valid_targets.contains(target.as_str()) {
                    return Err(GateError::UnknownTransitionTarget {
                        stage: stage.name.clone(),
                        target: target.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}
