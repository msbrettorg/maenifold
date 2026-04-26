use crate::error::GateError;

pub use maenifold_gate_core::task_metadata::{TaskMetadata, is_linked_session_id};

#[derive(Debug, Clone)]
pub struct TaskMetadataParser;

impl TaskMetadataParser {
    pub fn parse(lines: &[&str]) -> Result<TaskMetadata, GateError> {
        Ok(maenifold_gate_core::task_metadata::TaskMetadataParser::parse(lines)?)
    }

    pub fn parse_task_id(text: &str) -> Result<Option<String>, GateError> {
        Ok(maenifold_gate_core::task_metadata::TaskMetadataParser::parse_task_id(text)?)
    }
}
