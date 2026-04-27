//! Credential scanner for outbound Teams messages.
//!
//! Port of agency-cowork
//! `skills/teams/scripts/rich/credential_scanner.py` (389 LOC). The pattern
//! table lives in [`super::credential_guard_patterns`]; this module owns the
//! types and the `scan_for_credentials()` entrypoint.
//!
//! The scanner checks multiple decoded variants of the input to defeat
//! evasion via HTML attributes, URL-encoding, and HTML entity encoding
//! (py L296-302):
//!   1. Raw text (catches credentials in HTML attributes/comments)
//!   2. HTML-stripped + entity-decoded
//!   3. URL-decoded
//!   4. HTML entity-decoded raw
//!
//! Wired into [`super::reply`] in a follow-up sub-dispatch.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::credential_guard_patterns::{ALLOWLIST, PATTERNS};
use super::handler_helpers::decode_html_entities;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CredentialFinding {
    pub pattern_name: &'static str,
    pub description: &'static str,
    /// First 8 chars of the match + `***[REDACTED]`. py L236-240.
    pub match_preview: String,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CredentialScanResult {
    pub is_clean: bool,
    pub findings: Vec<CredentialFinding>,
}

/// True when the scan produced any credential findings.
#[allow(dead_code)]
pub fn has_findings(result: &CredentialScanResult) -> bool {
    !result.findings.is_empty()
}

/// Redact a match — first 8 chars + `***[REDACTED]`. py L236-240.
fn redact(match_text: &str) -> String {
    let chars: Vec<char> = match_text.chars().collect();
    if chars.len() <= 8 {
        return "***".to_string();
    }
    let head: String = chars.iter().take(8).collect();
    format!("{head}***[REDACTED]")
}

fn is_allowlisted(match_text: &str) -> bool {
    ALLOWLIST.iter().any(|p| p.is_match(match_text))
}

static HTML_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]+>").expect("html tag regex must compile"));

/// Strip HTML tags then decode entities. py L219-225.
fn strip_html(text: &str) -> String {
    let no_tags = HTML_TAG_RE.replace_all(text, " ");
    decode_html_entities(&no_tags)
}

/// Scan a single text variant, mutating findings/seen. py L257-284.
fn scan_variant(text: &str, findings: &mut Vec<CredentialFinding>, seen: &mut HashSet<String>) {
    for entry in PATTERNS.iter() {
        for m in entry.regex.find_iter(text) {
            let matched = m.as_str();
            if seen.contains(matched) {
                continue;
            }
            if is_allowlisted(matched) {
                continue;
            }
            seen.insert(matched.to_string());
            findings.push(CredentialFinding {
                pattern_name: entry.name,
                description: entry.description,
                match_preview: redact(matched),
            });
        }
    }
}

/// Scan text for credential patterns across multiple decoded variants.
/// Port of py `scan_for_credentials` (L287-335).
#[allow(dead_code)]
pub fn scan_for_credentials(text: &str) -> CredentialScanResult {
    if text.is_empty() {
        return CredentialScanResult {
            is_clean: true,
            findings: Vec::new(),
        };
    }

    let mut findings: Vec<CredentialFinding> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Variant 1: raw text.
    scan_variant(text, &mut findings, &mut seen);

    // Variant 2: HTML-stripped + entity-decoded.
    if text.contains('<') && text.contains('>') {
        let stripped = strip_html(text);
        scan_variant(&stripped, &mut findings, &mut seen);
    }

    // Variant 3: URL-decoded (catches %3D, %3B encoded connection strings).
    if let Ok(url_decoded) = urlencoding::decode(text) {
        if url_decoded != text {
            scan_variant(&url_decoded, &mut findings, &mut seen);
        }
    }

    // Variant 4: HTML entity-decoded raw text.
    let entity_decoded = decode_html_entities(text);
    if entity_decoded != text {
        scan_variant(&entity_decoded, &mut findings, &mut seen);
    }

    CredentialScanResult {
        is_clean: findings.is_empty(),
        findings,
    }
}
