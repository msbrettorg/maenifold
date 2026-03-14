use maenifold_core::markdown::WikiLinkFilter;
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

// T-10.16: RTM FR-19 / RTM-102 / RTM-103 / RTM-104 ports C# WikiLinkFilter unit coverage to Rust's JSON dotfile filter.
//
// NOTE: check_filter() calls try_check() and panics on parse errors.
// Tests that require check()'s fail-open semantics must call WikiLinkFilter directly.
// See check_filter_invalid_json_returns_empty for an example.
fn check_filter(
    filter_contents: Option<&str>,
    content: &str,
) -> std::result::Result<(), Vec<(String, String)>> {
    let tmp = TempDir::new().expect("tmp");

    if let Some(filter_contents) = filter_contents {
        fs::write(tmp.path().join(".wikilink-filter.json"), filter_contents).expect("write filter");
    }

    WikiLinkFilter::new(tmp.path())
        .try_check(content)
        .expect("filter evaluation")
}

// T-10.16: RTM FR-19 / RTM-103 normalizes blocked concepts into an assertion-friendly map.
fn blocked_map(result: std::result::Result<(), Vec<(String, String)>>) -> BTreeMap<String, String> {
    match result {
        Ok(()) => BTreeMap::new(),
        Err(blocked) => blocked.into_iter().collect(),
    }
}

// T-10.16: RTM FR-19 / RTM-102 / RTM-103 (includes coverage formerly in
// check_filter_with_filter_file_blocks_matching_concepts, merged here).
#[test]
fn check_filter_mixed_filtered_and_unfiltered() {
    let blocked = blocked_map(check_filter(
        Some(r#"{"tool":"Generic hub","agent":"Generic hub"}"#),
        "Testing [[tool]], [[react]], [[agent]], and [[typescript]].",
    ));

    assert_eq!(blocked.len(), 2);
    assert_eq!(blocked.get("tool").map(String::as_str), Some("Generic hub"));
    assert_eq!(
        blocked.get("agent").map(String::as_str),
        Some("Generic hub")
    );
    assert!(!blocked.contains_key("react"));
    assert!(!blocked.contains_key("typescript"));
}

// T-10.16: RTM FR-19 / RTM-103 / RTM-104
// JSON key "Tool" (capitalized) must normalize to "tool" during filter load.
#[test]
fn check_filter_json_key_is_normalized_to_lowercase() {
    let blocked = blocked_map(check_filter(Some(r#"{"Tool":"Hub"}"#), "Testing [[tool]]."));

    assert_eq!(blocked.get("tool").map(String::as_str), Some("Hub"));
}

// T-10.16: RTM FR-19 / RTM-103
// Populated filter, empty content string — no wikilinks extracted → Ok(()).
#[test]
fn check_filter_no_wikilinks_in_content_returns_ok() {
    let result = check_filter(Some(r#"{"tool":"Generic hub","agent":"Generic hub"}"#), "");

    assert_eq!(result, Ok(()));
}

// T-10.16: RTM FR-19 preserves `check()`'s fail-open behavior when `.wikilink-filter.json` is invalid.
// check() — not try_check() — is called here intentionally.
// check() has fail-open semantics: parse errors return Ok(()) rather than propagating.
// See also: wikilink_filter_returns_error_for_invalid_json_in_try_check (filter.rs)
// for the fail-strict counterpart.
#[test]
fn check_filter_invalid_json_returns_empty() {
    let tmp = TempDir::new().expect("tmp");
    fs::write(
        tmp.path().join(".wikilink-filter.json"),
        "not valid json {{{",
    )
    .expect("write filter");
    let result = WikiLinkFilter::new(tmp.path()).check("Testing [[tool]].");

    assert_eq!(result, Ok(()));
}

// T-22.1: RTM FR-19 (RTM-151, RTM-152, RTM-155) — is_concept_blocked() single-concept API.
#[test]
fn is_concept_blocked_returns_true_for_exact_match() {
    let tmp = TempDir::new().expect("tmp");
    fs::write(
        tmp.path().join(".wikilink-filter.json"),
        r#"{"tool":"Generic hub"}"#,
    )
    .expect("write filter");

    let filter = WikiLinkFilter::new(tmp.path());
    assert!(filter.is_concept_blocked("tool"));
}

// T-22.1: RTM FR-19 (RTM-152, RTM-155) — prefix-pattern match via is_concept_blocked().
#[test]
fn is_concept_blocked_returns_true_for_prefix_match() {
    let tmp = TempDir::new().expect("tmp");
    fs::write(
        tmp.path().join(".wikilink-filter.json"),
        r#"{"t-*":"Process metadata"}"#,
    )
    .expect("write filter");

    let filter = WikiLinkFilter::new(tmp.path());
    assert!(filter.is_concept_blocked("t-14.1"));
}

// T-22.1: RTM FR-19 (RTM-151) — no match returns false.
#[test]
fn is_concept_blocked_returns_false_when_no_match() {
    let tmp = TempDir::new().expect("tmp");
    fs::write(
        tmp.path().join(".wikilink-filter.json"),
        r#"{"tool":"Generic hub"}"#,
    )
    .expect("write filter");

    let filter = WikiLinkFilter::new(tmp.path());
    assert!(!filter.is_concept_blocked("react"));
}

// T-22.1: RTM FR-19 (RTM-151) — missing filter file → false (fail-open).
#[test]
fn is_concept_blocked_returns_false_when_filter_file_missing() {
    let tmp = TempDir::new().expect("tmp");

    let filter = WikiLinkFilter::new(tmp.path());
    assert!(!filter.is_concept_blocked("tool"));
}
