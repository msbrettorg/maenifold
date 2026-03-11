#[path = "frontmatter/support.rs"]
mod support;

use maenifold_core::markdown::{read_markdown, write_markdown};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::thread;
use std::time::Duration as StdDuration;
use support::{TEST_MUTEX, isolated_env, read_raw_markdown, yaml_string};

// T-12.6: RTM NFR-1 — Split write/frontmatter envelope regressions into a focused integration test file.
// T-META-001.6: RTM FR-24.x — Ported frontmatter metadata regression coverage from the C# oracle.

// T-META-001.6: RTM FR-24.1 — WriteMarkdown envelope management.
#[test]
fn fr24_1_ac1_write_markdown_new_file_has_created_and_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-new";

    write_markdown(file_uri, None, "# Test\n\n[[concept]]", &env.cfg).expect("write markdown");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(frontmatter.contains_key("modified"));
    assert!(yaml_string(&frontmatter, "created").is_some_and(|value| !value.is_empty()));
    assert!(yaml_string(&frontmatter, "modified").is_some_and(|value| !value.is_empty()));
}

// T-META-001.6: RTM FR-24.1 — Existing created value survives rewrites.
#[test]
fn fr24_1_ac2_write_markdown_existing_created_preserved_on_rewrite() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-preserve-created";

    write_markdown(file_uri, None, "# Test\n\n[[concept]]", &env.cfg).expect("initial write");
    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read first write");
    let original_created =
        yaml_string(&frontmatter.expect("frontmatter"), "created").expect("created");

    thread::sleep(StdDuration::from_millis(10));

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(original_created.clone()),
    );
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Test updated\n\n[[concept]]",
        &env.cfg,
    )
    .expect("rewrite markdown");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read rewrite");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        yaml_string(&frontmatter, "created").as_deref(),
        Some(original_created.as_str())
    );
    assert!(yaml_string(&frontmatter, "modified").is_some_and(|value| !value.is_empty()));
}

// T-META-001.6: RTM FR-24.1 — Missing created is added on write.
#[test]
fn fr24_1_ac3_write_markdown_missing_created_added_on_rewrite() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-add-created";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert("title".to_string(), YamlValue::String("Test".to_string()));
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Test\n\n[[concept]]",
        &env.cfg,
    )
    .expect("write markdown");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(yaml_string(&frontmatter, "created").is_some_and(|value| !value.is_empty()));
}

// T-META-001.6: RTM FR-24.1 — Null frontmatter still yields an envelope.
#[test]
fn fr24_1_ac4_write_markdown_null_frontmatter_dict_created() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-null-fm";

    write_markdown(file_uri, None, "# Null FM\n\n[[wikilink]]", &env.cfg).expect("write markdown");

    let raw_text = read_raw_markdown(&env.cfg, file_uri);
    assert!(
        raw_text.starts_with("---"),
        "raw text should start with YAML frontmatter: {raw_text}"
    );

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(frontmatter.contains_key("modified"));
}

// T-META-001.6: RTM FR-24.1 — Lifecycle timestamps are preserved.
#[test]
fn fr24_1_ac5_write_markdown_lifecycle_timestamps_not_overwritten() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-lifecycle";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    frontmatter.insert(
        "completed".to_string(),
        YamlValue::String("2025-01-01T00:00:00Z".to_string()),
    );
    frontmatter.insert(
        "cancelled".to_string(),
        YamlValue::String("2025-02-01T00:00:00Z".to_string()),
    );

    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Lifecycle\n\n[[workflow]]",
        &env.cfg,
    )
    .expect("write markdown");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        yaml_string(&frontmatter, "completed").as_deref(),
        Some("2025-01-01T00:00:00Z")
    );
    assert_eq!(
        yaml_string(&frontmatter, "cancelled").as_deref(),
        Some("2025-02-01T00:00:00Z")
    );
}
