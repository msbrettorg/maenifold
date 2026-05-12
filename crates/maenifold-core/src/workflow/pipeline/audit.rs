use crate::config::Config;
use crate::error::Result;
use std::path::PathBuf;

pub(super) const PIPELINE_DELETE_AUDIT_FILE: &str = "pipeline-delete-audit.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PipelineDeleteAuditRecord {
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

pub(super) fn pipeline_delete_audit_path(config: &Config) -> PathBuf {
    config
        .maenifold_root
        .join("logs")
        .join(PIPELINE_DELETE_AUDIT_FILE)
}
