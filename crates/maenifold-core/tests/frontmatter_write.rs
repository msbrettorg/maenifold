#[path = "frontmatter/support.rs"]
mod support;

use maenifold_core::markdown::{read_markdown, write_markdown};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use support::{TEST_MUTEX, isolated_env, read_raw_markdown, yaml_string};

// T-12.6: RTM NFR-1 — Split write/frontmatter envelope regressions into a focused integration test file.
// Frontmatter metadata regression coverage ported from the C# oracle.

// T-12.6: RTM NFR-1 — Existing created value survives rewrites; modified is advanced.
#[test]
fn write_markdown_existing_created_preserved_on_rewrite() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-preserve-created";
    let original_created = "2020-01-01T00:00:00.0000000+00:00";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(original_created.to_string()),
    );
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Test updated\n\n[[concept]]",
        &env.cfg,
    )
    .expect("write markdown");

    let (read_fm, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let read_fm = read_fm.expect("frontmatter");
    assert_eq!(
        yaml_string(&read_fm, "created").as_deref(),
        Some(original_created),
        "created timestamp must be preserved verbatim"
    );
    assert!(
        read_fm.contains_key("modified"),
        "modified key must be present"
    );
    assert_ne!(
        yaml_string(&read_fm, "modified").as_deref(),
        Some(original_created),
        "modified must differ from original created (timestamp was advanced)"
    );
}

// T-12.6: RTM NFR-1 — Null frontmatter produces a raw YAML envelope on disk.
#[test]
fn write_markdown_null_frontmatter_file_has_yaml_envelope() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-null-fm";

    write_markdown(file_uri, None, "# Null FM\n\n[[wikilink]]", &env.cfg).expect("write markdown");

    let raw_text = read_raw_markdown(&env.cfg, file_uri);
    assert!(
        raw_text.starts_with("---"),
        "raw file must open with YAML frontmatter delimiter: {raw_text}"
    );
}
