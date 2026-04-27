//! Prompt-injection scanner for untrusted Teams content.
//!
//! Port of `agency-cowork/scripts/prompt_guard.py` (536 LOC). The pattern
//! table lives in [`super::prompt_guard_patterns`]; this module owns the
//! types and the `scan()` entrypoint.
//!
//! Threshold policy mirrors agency-cowork
//! `skills/teams/scripts/monitor/message_handler.py` L997-1034: any
//! `Critical` or `High` finding on an external message blocks dispatch.
//! `Medium`/`Low` are advisory only.
//!
//! Self-chat (`conversation_id == "48:notes"`) bypasses scanning at the
//! caller — agency-cowork "user trusts themselves" model.
//!
//! Allowlist semantics (py L398-415): allowlisted text DEMOTES findings to
//! `Low`; never suppresses them. Prevents the
//! "embed `threatmodel.md` to disable the scanner" bypass.

use unicode_normalization::UnicodeNormalization;

use super::handler_helpers::decode_html_entities;
use super::prompt_guard_patterns::{ALLOWLIST, INVISIBLE_RE, PATTERNS};

/// Severity levels. Derived `Ord` gives `Low < Medium < High < Critical`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InjectionFinding {
    pub pattern_name: &'static str,
    pub description: &'static str,
    pub severity: Severity,
    pub matched_text: String,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct InjectionScanResult {
    pub clean: bool,
    pub findings: Vec<InjectionFinding>,
    pub max_severity: Option<Severity>,
    pub scanned_length: usize,
}

impl InjectionScanResult {
    /// Block dispatch when any High or Critical finding is present.
    pub fn is_blocking(&self) -> bool {
        self.max_severity.is_some_and(|s| s >= Severity::High)
    }

    pub fn pattern_names_csv(&self) -> String {
        self.findings
            .iter()
            .map(|f| f.pattern_name)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// First N chars of the first finding's matched text, for log preview.
    pub fn preview(&self, max: usize) -> String {
        let Some(first) = self.findings.first() else {
            return String::new();
        };
        first.matched_text.chars().take(max).collect()
    }
}

/// Normalize untrusted text per prompt_guard.py L346-374:
/// 1. NFKD decomposition (collapses fullwidth chars).
/// 2. Strip invisible / zero-width characters.
/// 3. Decode HTML character references.
fn normalize_text(text: &str) -> String {
    let nfkd: String = text.nfkd().collect();
    let stripped = INVISIBLE_RE.replace_all(&nfkd, "");
    decode_html_entities(&stripped)
}

fn is_allowlisted(text: &str) -> bool {
    ALLOWLIST.iter().any(|p| p.is_match(text))
}

/// Truncate matched text to <=120 chars with "..." suffix when over,
/// per prompt_guard.py L408-412.
fn truncate_match(matched: &str) -> String {
    let count = matched.chars().count();
    if count > 120 {
        let head: String = matched.chars().take(117).collect();
        format!("{head}...")
    } else {
        matched.to_string()
    }
}

/// Scan text for prompt-injection patterns. See module docs for policy.
pub fn scan(text: &str) -> InjectionScanResult {
    if text.is_empty() {
        return InjectionScanResult {
            clean: true,
            ..Default::default()
        };
    }
    let normalized = normalize_text(text);
    if normalized.trim().is_empty() {
        return InjectionScanResult {
            clean: true,
            ..Default::default()
        };
    }

    let allowlisted = is_allowlisted(&normalized);
    let mut findings = Vec::new();

    for entry in PATTERNS.iter() {
        if let Some(m) = entry.regex.find(&normalized) {
            // Allowlist demotes to Low; never suppresses (py L398-415).
            let severity = if allowlisted {
                Severity::Low
            } else {
                entry.severity
            };
            findings.push(InjectionFinding {
                pattern_name: entry.name,
                description: entry.description,
                severity,
                matched_text: truncate_match(m.as_str()),
            });
        }
    }

    let max_severity = findings.iter().map(|f| f.severity).max();
    InjectionScanResult {
        clean: findings.is_empty(),
        findings,
        max_severity,
        scanned_length: normalized.chars().count(),
    }
}

#[cfg(test)]
#[path = "prompt_guard_tests.rs"]
mod tests;
