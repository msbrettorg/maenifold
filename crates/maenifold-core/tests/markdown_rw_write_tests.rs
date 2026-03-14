#[path = "markdown_rw_support/mod.rs"]
mod markdown_rw_support;

use chrono::DateTime;
use maenifold_core::markdown::{Frontmatter, append_h2_section, read_markdown, write_markdown};
use markdown_rw_support::{TEST_MUTEX, isolated_env, memory_uri, scalar};
use serde_yaml::Value as YamlValue;
use std::thread;
use std::time::Duration;

// T-12.7.1: RTM NFR-1 — Helper: assert a frontmatter value is a valid RFC 3339 timestamp.
fn assert_valid_rfc3339(frontmatter: &Frontmatter, key: &str) -> String {
    let value = scalar(frontmatter, key)
        .unwrap_or_else(|| panic!("frontmatter key '{key}' must be present"));
    assert!(!value.is_empty(), "'{key}' timestamp must be non-empty");
    DateTime::parse_from_rfc3339(&value)
        .unwrap_or_else(|e| panic!("'{key}' value '{value}' is not valid RFC 3339: {e}"));
    value
}

// T-12.7.1: RTM NFR-1 — Split markdown write/append regressions by mutation behavior.

#[test]
fn write_markdown_null_frontmatter_creates_envelope_with_created_and_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/null-frontmatter.md");

    write_markdown(&uri, None, "# Test\n\nContent", &env.cfg).expect("write markdown");

    let (frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    // T-12.7.1: RTM NFR-1 — Assert timestamps are valid RFC 3339, not just present.
    let created = assert_valid_rfc3339(&frontmatter, "created");
    let modified = assert_valid_rfc3339(&frontmatter, "modified");
    assert!(created <= modified, "created ({created}) must be <= modified ({modified})");
}

#[test]
fn write_markdown_existing_created_preserves_created_and_updates_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/preserve-created.md");
    let original_created = "2020-01-01T00:00:00.0000000+00:00";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(original_created.to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Test\n\nContent", &env.cfg).expect("write");

    let (read_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read");
    let read_frontmatter = read_frontmatter.expect("frontmatter");
    assert_eq!(
        scalar(&read_frontmatter, "created").as_deref(),
        Some(original_created)
    );
    // T-12.7.1: RTM NFR-1 — Assert modified is a valid timestamp and > original_created.
    let modified = assert_valid_rfc3339(&read_frontmatter, "modified");
    assert!(
        modified.as_str() > original_created,
        "modified ({modified}) must be > original created ({original_created})"
    );
}

#[test]
fn write_markdown_missing_created_adds_created_and_sets_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/add-created.md");
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("My Note".to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# My Note\n\nContent", &env.cfg).expect("write");

    let (read_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read");
    let read_frontmatter = read_frontmatter.expect("frontmatter");
    // T-12.7.1: RTM NFR-1 — Assert timestamps are valid RFC 3339, not just present.
    let created = assert_valid_rfc3339(&read_frontmatter, "created");
    let modified = assert_valid_rfc3339(&read_frontmatter, "modified");
    assert!(created <= modified, "created ({created}) must be <= modified ({modified})");
    // Verify custom frontmatter key is preserved alongside timestamps.
    assert_eq!(
        scalar(&read_frontmatter, "title").as_deref(),
        Some("My Note")
    );
}

#[test]
fn write_markdown_completed_and_cancelled_not_overwritten() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/lifecycle.md");
    let completed_time = "2025-06-01T12:00:00.0000000+00:00";
    let cancelled_time = "2025-06-02T08:00:00.0000000+00:00";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    frontmatter.insert(
        "completed".to_string(),
        YamlValue::String(completed_time.to_string()),
    );
    frontmatter.insert(
        "cancelled".to_string(),
        YamlValue::String(cancelled_time.to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Session\n\nContent", &env.cfg).expect("write");

    let (read_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read");
    let read_frontmatter = read_frontmatter.expect("frontmatter");
    assert_eq!(
        scalar(&read_frontmatter, "completed").as_deref(),
        Some(completed_time)
    );
    assert_eq!(
        scalar(&read_frontmatter, "cancelled").as_deref(),
        Some(cancelled_time)
    );
}

#[test]
fn write_markdown_does_not_mutate_caller_dict() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/no-mutate.md");
    let stale_modified = "2020-01-01T00:00:00.0000000+00:00";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "modified".to_string(),
        YamlValue::String(stale_modified.to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Test\n\nContent", &env.cfg).expect("write");

    assert_eq!(
        scalar(&frontmatter, "modified").as_deref(),
        Some(stale_modified)
    );
}

#[test]
fn append_h2_section_existing_file_updates_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/append-updates-modified.md");
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String("2020-01-01T00:00:00.0000000+00:00".to_string()),
    );
    frontmatter.insert(
        "modified".to_string(),
        YamlValue::String("2020-01-01T00:00:00.0000000+00:00".to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Test\n\nContent", &env.cfg).expect("write");
    let (before_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read before");
    let before_modified = assert_valid_rfc3339(
        &before_frontmatter.expect("frontmatter"),
        "modified",
    );

    // T-12.7.1: RTM NFR-1 — Sleep ≥ 50ms to avoid timestamp collision on low-resolution clocks.
    thread::sleep(Duration::from_millis(50));
    append_h2_section(&uri, "New Section", "New content", &env.cfg).expect("append section");

    let (after_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read after");
    let after_frontmatter = after_frontmatter.expect("frontmatter");
    let after_modified = assert_valid_rfc3339(&after_frontmatter, "modified");
    // Lexicographic comparison is valid for RFC 3339 — asserts the timestamp advanced.
    assert!(
        after_modified > before_modified,
        "modified after append ({after_modified}) must be > modified before ({before_modified})"
    );
}

#[test]
fn append_h2_section_existing_file_preserves_all_frontmatter_fields() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/append-preserves-frontmatter.md");
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("My Session".to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Session\n\nContent", &env.cfg).expect("write");
    append_h2_section(&uri, "Thought 1", "First thought content", &env.cfg).expect("append");

    let (read_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read");
    let read_frontmatter = read_frontmatter.expect("frontmatter");
    assert_eq!(
        scalar(&read_frontmatter, "type").as_deref(),
        Some("sequential")
    );
    assert_eq!(
        scalar(&read_frontmatter, "status").as_deref(),
        Some("active")
    );
    assert_eq!(
        scalar(&read_frontmatter, "title").as_deref(),
        Some("My Session")
    );
}

#[test]
fn append_h2_section_existing_file_preserves_existing_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/append-preserves-content.md");
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Session\n\nExisting content here.",
        &env.cfg,
    )
    .expect("write");
    append_h2_section(&uri, "Appended Section", "New content", &env.cfg).expect("append");

    let (_, content, _) = read_markdown(&uri, &env.cfg).expect("read");
    assert!(content.contains("Existing content here."));
    assert!(content.contains("## Appended Section"));
    assert!(content.contains("New content"));
}

#[test]
fn append_h2_section_non_existent_file_creates_file_with_envelope() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/new-append-file.md");

    append_h2_section(&uri, "First Section", "First content", &env.cfg).expect("append");

    let (frontmatter, content, _) = read_markdown(&uri, &env.cfg).expect("read");
    let frontmatter = frontmatter.expect("frontmatter");
    // T-12.7.1: RTM NFR-1 — Assert timestamps are valid RFC 3339, not just present.
    let created = assert_valid_rfc3339(&frontmatter, "created");
    let modified = assert_valid_rfc3339(&frontmatter, "modified");
    assert!(created <= modified, "created ({created}) must be <= modified ({modified})");
    // Assert section heading and content appear in the body.
    assert!(content.contains("## First Section"), "body must contain section heading");
    assert!(content.contains("First content"), "body must contain section content");
}
