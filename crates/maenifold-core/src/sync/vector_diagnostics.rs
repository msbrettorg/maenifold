use crate::config::env_bool;
use crate::error::Result;
use crate::markdown::extract_wikilinks;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

// T-QUAL-002.6b: RTM FR-25.2
pub struct VectorFailureReasonCodes;

impl VectorFailureReasonCodes {
    pub const CONCEPT_LOOKUP_FAILED: &'static str = "CONCEPT_LOOKUP_FAILED";
    pub const FILE_EMBEDDING_FAILED: &'static str = "FILE_EMBEDDING_FAILED";
    pub const CONCEPT_EMBEDDING_FAILED: &'static str = "CONCEPT_EMBEDDING_FAILED";
    pub const CONCEPT_BLOB_SIZE_MISMATCH: &'static str = "CONCEPT_BLOB_SIZE_MISMATCH";
    pub const FILE_BLOB_SIZE_MISMATCH: &'static str = "FILE_BLOB_SIZE_MISMATCH";
}

// T-QUAL-002.6b: RTM FR-25.2
pub struct VectorUnavailableReasonCodes;

impl VectorUnavailableReasonCodes {
    pub const DISABLED_BY_CONFIG: &'static str = "VECTOR_DISABLED_BY_CONFIG";
    pub const UNKNOWN: &'static str = "VECTOR_UNAVAILABLE_UNKNOWN";
}

// T-QUAL-002.6b: RTM FR-25.2
#[derive(Debug, Clone, Default)]
pub struct VectorSyncDiagnostics {
    concept_embedding_failures: usize,
    file_embedding_failures: usize,
    concept_embedding_skipped: usize,
    file_embedding_skipped: usize,
    failure_reasons: BTreeMap<String, usize>,
}

impl VectorSyncDiagnostics {
    pub fn record_concept_failure(&mut self, reason: &str) {
        self.concept_embedding_failures += 1;
        *self.failure_reasons.entry(reason.to_string()).or_default() += 1;
    }

    pub fn record_file_failure(&mut self, reason: &str) {
        self.file_embedding_failures += 1;
        *self.failure_reasons.entry(reason.to_string()).or_default() += 1;
    }

    pub fn record_concept_skipped(&mut self, count: usize) {
        self.concept_embedding_skipped += count;
    }

    pub fn record_file_skipped(&mut self) {
        self.file_embedding_skipped += 1;
    }

    pub fn concept_embedding_failures(&self) -> usize {
        self.concept_embedding_failures
    }

    pub fn file_embedding_failures(&self) -> usize {
        self.file_embedding_failures
    }

    pub fn total_failures(&self) -> usize {
        self.concept_embedding_failures + self.file_embedding_failures
    }

    pub fn concept_embedding_skipped(&self) -> usize {
        self.concept_embedding_skipped
    }

    pub fn file_embedding_skipped(&self) -> usize {
        self.file_embedding_skipped
    }

    pub fn total_skipped(&self) -> usize {
        self.concept_embedding_skipped + self.file_embedding_skipped
    }

    pub fn failure_reasons(&self) -> &BTreeMap<String, usize> {
        &self.failure_reasons
    }
}

// T-QUAL-002.6b: RTM FR-25.2
pub fn build_vector_failure_report(diagnostics: &VectorSyncDiagnostics) -> Vec<String> {
    let mut lines = vec![
        "Vector sync diagnostics:".to_string(),
        "- STATUS: failed".to_string(),
        format!(
            "- COUNTERS_FAILED: concepts={}, memory={}, total={}",
            diagnostics.concept_embedding_failures(),
            diagnostics.file_embedding_failures(),
            diagnostics.total_failures()
        ),
        format!(
            "- COUNTERS_SKIPPED: concepts={}, memory={}, total={}",
            diagnostics.concept_embedding_skipped(),
            diagnostics.file_embedding_skipped(),
            diagnostics.total_skipped()
        ),
    ];

    if !diagnostics.failure_reasons().is_empty() {
        lines.push("- FAILURES:".to_string());
        for (reason, count) in diagnostics.failure_reasons() {
            lines.push(format!("- REASON {reason}: {count}"));
        }
    }

    lines
}

// T-QUAL-002.6b: RTM FR-25.2
pub fn build_vector_unavailable_report(
    diagnostics: &VectorSyncDiagnostics,
    reason: &str,
) -> Vec<String> {
    vec![
        "Vector sync diagnostics:".to_string(),
        "- STATUS: unavailable".to_string(),
        format!(
            "- COUNTERS_FAILED: concepts={}, memory={}, total={}",
            diagnostics.concept_embedding_failures(),
            diagnostics.file_embedding_failures(),
            diagnostics.total_failures()
        ),
        format!(
            "- COUNTERS_SKIPPED: concepts={}, memory={}, total={}",
            diagnostics.concept_embedding_skipped(),
            diagnostics.file_embedding_skipped(),
            diagnostics.total_skipped()
        ),
        format!(
            "- REASON_CODE {}",
            normalize_vector_unavailable_reason(reason)
        ),
    ]
}

pub(crate) fn vector_sync_disabled_by_config() -> bool {
    // T-11.2: RTM FR-15
    env_bool("MAENIFOLD_DISABLE_VECTOR", false)
}

pub(crate) fn collect_vector_skip_diagnostics_for_paths(
    file_paths: &[PathBuf],
) -> Result<VectorSyncDiagnostics> {
    let mut diagnostics = VectorSyncDiagnostics::default();
    let mut concepts = BTreeSet::new();

    for file_path in file_paths {
        let content = fs::read_to_string(file_path)?;
        diagnostics.record_file_skipped();
        concepts.extend(extract_wikilinks(&content));
    }

    diagnostics.record_concept_skipped(concepts.len());
    Ok(diagnostics)
}

fn normalize_vector_unavailable_reason(reason: &str) -> &'static str {
    let normalized = reason.trim().to_ascii_uppercase();
    if normalized == VectorUnavailableReasonCodes::DISABLED_BY_CONFIG {
        VectorUnavailableReasonCodes::DISABLED_BY_CONFIG
    } else {
        VectorUnavailableReasonCodes::UNKNOWN
    }
}
