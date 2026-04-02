use std::fs;
use std::path::Path;

use regex::Regex;

use crate::error::GateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct PrdParser;

impl PrdParser {
    pub fn parse(path: &Path) -> Result<Vec<Requirement>, GateError> {
        let text = fs::read_to_string(path)?;
        let pattern = Regex::new(r"^\s*-\s+\*\*(?P<id>(?:FR|NFR)-\d+)\*\*:\s*(?P<description>.+?)\s*$")
            .map_err(|error| GateError::TaskMetadataParse(format!("invalid PRD regex: {error}")))?;
        let mut requirements = Vec::new();

        for line in text.lines() {
            let Some(captures) = pattern.captures(line) else {
                continue;
            };
            requirements.push(Requirement {
                id: captures["id"].to_string(),
                description: captures["description"].to_string(),
            });
        }

        Ok(requirements)
    }
}
