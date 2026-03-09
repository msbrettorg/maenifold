use maenifold_core::markdown::WikiLinkFilter;
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

// T-10.16: RTM FR-19 / RTM-102 / RTM-103 / RTM-104 ports C# WikiLinkFilter unit coverage to Rust's JSON dotfile filter.
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

// T-10.16: RTM FR-19 / RTM-102 / RTM-103
#[test]
fn check_filter_with_filter_file_blocks_matching_concepts() {
    let blocked = blocked_map(check_filter(
        Some(r#"{"tool":"Generic hub"}"#),
        "Testing [[tool]] and [[react]] together.",
    ));

    assert_eq!(blocked.get("tool").map(String::as_str), Some("Generic hub"));
    assert!(!blocked.contains_key("react"));
}

// T-10.16: RTM FR-19 / RTM-102
#[test]
fn check_filter_missing_file_returns_empty() {
    let result = check_filter(None, "Testing [[tool]] and [[agent]] together.");

    assert_eq!(result, Ok(()));
}

// T-10.16: RTM FR-19 / RTM-103 / RTM-104
#[test]
fn check_filter_case_insensitive_matching() {
    let blocked = blocked_map(check_filter(Some(r#"{"Tool":"Hub"}"#), "Testing [[tool]]."));

    assert_eq!(blocked.get("tool").map(String::as_str), Some("Hub"));
}

// T-10.16: RTM FR-19 / RTM-103
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

// T-10.16: RTM FR-19 / RTM-103
#[test]
fn check_filter_empty_concepts_list_returns_empty() {
    let result = check_filter(Some(r#"{"tool":"Generic hub","agent":"Generic hub"}"#), "");

    assert_eq!(result, Ok(()));
}

// T-10.16: RTM FR-19 preserves `check()`'s fail-open behavior when `.wikilink-filter.json` is invalid.
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
