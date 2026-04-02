use std::fs;
use std::path::Path;

use crate::error::GateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityRow {
    pub requirement: String,
    pub component: String,
    pub test_case: String,
    pub test_type: String,
}

#[derive(Debug, Clone)]
pub struct RtmParser;

impl RtmParser {
    pub fn parse(path: &Path) -> Result<Vec<TraceabilityRow>, GateError> {
        let text = fs::read_to_string(path)?;
        let mut rows = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('|') || trimmed.contains("Requirement") || trimmed.contains("---") {
                continue;
            }

            let cells: Vec<String> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|value| value.trim().to_string())
                .collect();
            if cells.len() != 4 {
                continue;
            }

            rows.push(TraceabilityRow {
                requirement: cells[0].clone(),
                component: cells[1].clone(),
                test_case: cells[2]
                    .split(':')
                    .next()
                    .map(str::trim)
                    .unwrap_or(&cells[2])
                    .to_string(),
                test_type: cells[3].clone(),
            });
        }

        Ok(rows)
    }
}
