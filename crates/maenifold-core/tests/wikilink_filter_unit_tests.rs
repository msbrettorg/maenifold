use maenifold_core::markdown::WikiLinkFilter;
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

// T-2.4: RTM FR-11.1 ports C# WikiLinkFilter unit coverage to Rust's line-based dotfile filter.
fn check_filter(
    filter_contents: Option<&str>,
    content: &str,
) -> std::result::Result<(), Vec<(String, String)>> {
    let tmp = TempDir::new().expect("tmp");

    if let Some(filter_contents) = filter_contents {
        fs::write(tmp.path().join(".wikilink-filter"), filter_contents).expect("write filter");
    }

    WikiLinkFilter::new(tmp.path())
        .try_check(content)
        .expect("filter evaluation")
}

// T-2.4: RTM FR-11.1 normalizes blocked concepts into an assertion-friendly map.
fn blocked_map(result: std::result::Result<(), Vec<(String, String)>>) -> BTreeMap<String, String> {
    match result {
        Ok(()) => BTreeMap::new(),
        Err(blocked) => blocked.into_iter().collect(),
    }
}

// T-2.4: RTM FR-11.1
#[test]
fn check_filter_with_filter_file_blocks_matching_concepts() {
    let blocked = blocked_map(check_filter(
        Some("tool | Generic hub\n"),
        "Testing [[tool]] and [[react]] together.",
    ));

    assert_eq!(blocked.get("tool").map(String::as_str), Some("Generic hub"));
    assert!(!blocked.contains_key("react"));
}

// T-2.4: RTM FR-11.1
#[test]
fn check_filter_missing_file_returns_empty() {
    let result = check_filter(None, "Testing [[tool]] and [[agent]] together.");

    assert_eq!(result, Ok(()));
}

// T-2.4: RTM FR-11.1
#[test]
fn check_filter_case_insensitive_matching() {
    let blocked = blocked_map(check_filter(Some("Tool | Hub\n"), "Testing [[tool]]."));

    assert_eq!(blocked.get("tool").map(String::as_str), Some("Hub"));
}

// T-2.4: RTM FR-11.1
#[test]
fn check_filter_mixed_filtered_and_unfiltered() {
    let blocked = blocked_map(check_filter(
        Some("tool | Generic hub\nagent | Generic hub\n"),
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

// T-2.4: RTM FR-11.1
#[test]
fn check_filter_empty_concepts_list_returns_empty() {
    let result = check_filter(Some("tool | Generic hub\nagent | Generic hub\n"), "");

    assert_eq!(result, Ok(()));
}

// T-2.4: RTM FR-11.1 current Rust parsing treats malformed JSON text as a non-matching line entry.
#[test]
fn check_filter_malformed_json_returns_empty() {
    let result = check_filter(Some("not valid json {{{"), "Testing [[tool]].");

    assert_eq!(result, Ok(()));
}
