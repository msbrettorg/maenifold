use crate::config::env_bool;
use rusqlite::Connection;
use std::collections::BTreeMap;

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
    concept_failure_details: BTreeMap<String, Vec<String>>,
}
impl VectorSyncDiagnostics {
    pub fn record_concept_failure(&mut self, reason: &str) {
        self.concept_embedding_failures += 1;
        *self.failure_reasons.entry(reason.to_string()).or_default() += 1;
    }

    pub fn record_concept_failure_with_detail(&mut self, reason: &str, detail: &str) {
        self.record_concept_failure(reason);
        self.record_concept_failure_detail(reason, detail);
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

    pub fn concept_failure_details(&self) -> &BTreeMap<String, Vec<String>> {
        &self.concept_failure_details
    }

    fn record_concept_failure_detail(&mut self, reason: &str, detail: &str) {
        // T-20.1: RTM FR-7 — capture compact sampled [[embedding]] error detail so
        // [[sync]] surfaces actionable tokenizer / ONNX runtime failures without changing control flow.
        let sanitized = sanitize_failure_detail(detail);
        if sanitized.is_empty() {
            return;
        }

        let entry = self
            .concept_failure_details
            .entry(reason.to_string())
            .or_default();
        if entry.iter().any(|existing| existing == &sanitized)
            || entry.len() >= MAX_CONCEPT_FAILURE_DETAILS
        {
            return;
        }

        entry.push(sanitized);
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

    for (reason, details) in diagnostics.concept_failure_details() {
        if details.is_empty() {
            continue;
        }

        lines.push(format!("- FAILURE_DETAILS {reason}:"));
        for detail in details {
            lines.push(format!("- DETAIL {detail}"));
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

// T-21.3: RTM-150 — query actual vec0 embedding counts after [[sync]] commit so the
// verification report matches the C# parity behavior instead of rendering hardcoded zero rows.
pub fn build_vector_verification_report(connection: &Connection) -> Vec<String> {
    let report = (|| -> rusqlite::Result<Vec<String>> {
        let concept_count = connection.query_row(
            "SELECT COUNT(*) FROM vec_concepts WHERE embedding IS NOT NULL",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        let file_count = connection.query_row(
            "SELECT COUNT(*) FROM vec_memory_files WHERE embedding IS NOT NULL",
            [],
            |row| row.get::<_, i64>(0),
        )?;

        if concept_count == 0 && file_count == 0 {
            return Ok(vec![
                "- WARNING_CODE VECTOR_TABLES_EMPTY_AFTER_SYNC".to_string(),
            ]);
        }

        let mut lines = vec!["Vector index verification:".to_string()];
        lines.push(format_embedding_verification_line(
            connection,
            "vec_concepts",
            "CONCEPT",
            concept_count,
        )?);
        lines.push(format_embedding_verification_line(
            connection,
            "vec_memory_files",
            "MEMORY",
            file_count,
        )?);
        Ok(lines)
    })();

    match report {
        Ok(lines) => lines,
        Err(_) => vec!["- WARNING_CODE VECTOR_VERIFICATION_SKIPPED".to_string()],
    }
}

pub(crate) fn vector_sync_disabled_by_config() -> bool {
    // T-11.2: RTM FR-15
    env_bool("MAENIFOLD_DISABLE_VECTOR", false)
}
fn normalize_vector_unavailable_reason(reason: &str) -> &'static str {
    let normalized = reason.trim().to_ascii_uppercase();
    if normalized == VectorUnavailableReasonCodes::DISABLED_BY_CONFIG {
        VectorUnavailableReasonCodes::DISABLED_BY_CONFIG
    } else {
        VectorUnavailableReasonCodes::UNKNOWN
    }
}

const MAX_CONCEPT_FAILURE_DETAILS: usize = 3;
const MAX_FAILURE_DETAIL_CHARS: usize = 240;

fn format_embedding_verification_line(
    connection: &Connection,
    table_name: &str,
    label: &str,
    count: i64,
) -> rusqlite::Result<String> {
    if count == 0 {
        return Ok(format!("- {label} embeddings: none"));
    }

    // T-21.3: RTM-150 — derive dimensions and self-distance from stored vec0 blobs so
    // verification reflects the actual persisted embeddings for each table.
    let embedding_bytes: i64 = connection.query_row(
        &format!("SELECT length(embedding) FROM {table_name} WHERE embedding IS NOT NULL LIMIT 1"),
        [],
        |row| row.get(0),
    )?;
    let self_distance: Option<f64> = connection.query_row(
        &format!(
            "SELECT vec_distance_cosine(embedding, embedding) FROM {table_name} WHERE embedding IS NOT NULL LIMIT 1"
        ),
        [],
        |row| row.get(0),
    )?;
    let dims = embedding_bytes / std::mem::size_of::<f32>() as i64;

    Ok(format!(
        "- {label} embeddings: {count} rows ({dims} dims, self-distance {})",
        self_distance
            .map(|distance| format!("{distance:.6}"))
            .unwrap_or_else(|| "n/a".to_string())
    ))
}

fn sanitize_failure_detail(detail: &str) -> String {
    let compact = detail.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= MAX_FAILURE_DETAIL_CHARS {
        return compact;
    }

    let truncated = compact
        .chars()
        .take(MAX_FAILURE_DETAIL_CHARS.saturating_sub(3))
        .collect::<String>();
    format!("{truncated}...")
}
