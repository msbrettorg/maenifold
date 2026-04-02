use std::fs;
use std::path::Path;

use crate::error::GateError;
use crate::task_metadata::{TaskMetadata, TaskMetadataParser};
use crate::task_sections::{TaskSectionParser, TaskStageRecord};

#[derive(Debug, Clone)]
pub struct TaskDocument {
    pub metadata: TaskMetadata,
    pub records: Vec<TaskStageRecord>,
}

#[derive(Debug, Clone)]
pub struct TaskFileReader;

impl TaskFileReader {
    pub fn read(path: &Path) -> Result<TaskDocument, GateError> {
        let text = fs::read_to_string(path)?;
        let lines: Vec<&str> = text.lines().collect();
        let metadata = TaskMetadataParser::parse(&lines)?;
        let records = TaskSectionParser::parse(&lines)?;
        Ok(TaskDocument { metadata, records })
    }
}

#[derive(Debug, Clone)]
pub struct TaskFileWriter;

impl TaskFileWriter {
    pub fn append_section(path: &Path, section: &str) -> Result<(), GateError> {
        let mut text = fs::read_to_string(path)?;
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
        text.push_str(section.trim_end());
        text.push('\n');
        fs::write(path, text)?;
        Ok(())
    }
}
