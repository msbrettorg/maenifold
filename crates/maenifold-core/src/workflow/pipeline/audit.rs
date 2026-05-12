use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub(super) const PIPELINE_DELETE_AUDIT_FILE: &str = "pipeline-delete-audit.jsonl";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PipelineDeleteAuditRecord {
    #[serde(skip_deserializing, default = "pipeline_delete_op")]
    pub(super) op: &'static str,
    pub(super) session_id: String,
    pub(super) force: bool,
    pub(super) files: Vec<String>,
    pub(super) requester_agent: String,
    pub(super) requester_pid: u32,
    pub(super) ts: String,
}

pub(super) trait AuditWriter {
    fn write_pipeline_delete(&self, record: &PipelineDeleteAuditRecord) -> Result<()>;
}

pub(super) struct JsonlAuditWriter<'a> {
    config: &'a Config,
}

impl<'a> JsonlAuditWriter<'a> {
    pub(super) fn new(config: &'a Config) -> Self {
        Self { config }
    }
}

impl AuditWriter for JsonlAuditWriter<'_> {
    fn write_pipeline_delete(&self, record: &PipelineDeleteAuditRecord) -> Result<()> {
        let path = pipeline_delete_audit_path(self.config);
        let mut line = serde_json::to_vec(record).map_err(|err| audit_error(&path, err))?;
        line.push(b'\n');
        let parent = path.parent().ok_or_else(|| {
            MaenifoldError::Message(format!(
                "audit write failed for {}: missing parent directory",
                path.display()
            ))
        })?;
        fs::create_dir_all(parent).map_err(|err| audit_error(&path, err))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|err| audit_error(&path, err))?;
        file.write_all(&line)
            .map_err(|err| audit_error(&path, err))?;
        file.flush().map_err(|err| audit_error(&path, err))?;
        file.sync_all().map_err(|err| audit_error(&path, err))?;
        Ok(())
    }
}

pub(super) fn pipeline_delete_audit_path(config: &Config) -> PathBuf {
    config
        .maenifold_root
        .join("logs")
        .join(PIPELINE_DELETE_AUDIT_FILE)
}

fn pipeline_delete_op() -> &'static str {
    "pipeline_delete"
}

fn audit_error(path: &PathBuf, err: impl std::fmt::Display) -> MaenifoldError {
    MaenifoldError::Message(format!("audit write failed for {}: {err}", path.display()))
}
