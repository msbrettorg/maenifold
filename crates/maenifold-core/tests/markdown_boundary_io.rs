#[path = "markdown_boundary/support.rs"]
mod support;

use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use support::*;

// T-12.7: RTM NFR-1 - split markdown I/O boundary coverage by theme.

// T-W1-2: RTM FR-28.1
#[test]
fn fr28_1_ac1_read_markdown_valid_uri_returns_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("Test Note".to_string()),
    );
    seed_file(
        &env.cfg,
        "memory://notes/test-note",
        Some(&frontmatter),
        "# Hello\n\nWorld",
    );

    let (read_fm, read_content, checksum) =
        markdown::read_markdown("memory://notes/test-note", &env.cfg).expect("read markdown");

    let read_fm = read_fm.expect("frontmatter");
    assert_eq!(yaml_string(&read_fm, "title").as_deref(), Some("Test Note"));
    assert!(read_content.contains("World"));
    assert!(!checksum.is_empty());
}

// T-W1-2: RTM FR-28.1
#[test]
fn fr28_1_ac2_read_markdown_nested_uri_returns_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(
        &env.cfg,
        "memory://projects/alpha/docs/readme",
        None,
        "# Nested\n\nDeeply nested content.",
    );

    let (_, content, _) =
        markdown::read_markdown("memory://projects/alpha/docs/readme", &env.cfg).expect("read");

    assert!(content.contains("Deeply nested content."));
}

// T-W1-2: RTM FR-28.1
#[test]
fn fr28_1_ac3_read_markdown_non_existent_uri_returns_not_found_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err =
        markdown::read_markdown("memory://does-not-exist", &env.cfg).expect_err("missing file");

    assert_message_error(err, "Memory file not found");
}

// T-W1-2: RTM FR-28.2
#[test]
fn fr28_2_ac1_write_markdown_valid_uri_creates_file() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    markdown::write_markdown(
        "memory://simple-note",
        None,
        "# Simple\n\nContent",
        &env.cfg,
    )
    .expect("write markdown");

    assert!(env.cfg.memory_path.join("simple-note.md").exists());
}

// T-W1-2: RTM FR-28.2
#[test]
fn fr28_2_ac2_write_markdown_nested_uri_creates_intermediate_dirs() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    markdown::write_markdown(
        "memory://deep/nested/folder/structure/note",
        None,
        "# Deep\n\nContent",
        &env.cfg,
    )
    .expect("write nested markdown");

    assert!(
        env.cfg
            .memory_path
            .join("deep")
            .join("nested")
            .join("folder")
            .join("structure")
            .join("note.md")
            .exists()
    );
}

// T-W1-2: RTM FR-28.2
#[test]
fn fr28_2_ac3_write_markdown_frontmatter_round_trips() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("Round Trip".to_string()),
    );
    frontmatter.insert(
        "tags".to_string(),
        YamlValue::Sequence(vec![
            YamlValue::String("test".to_string()),
            YamlValue::String("boundary".to_string()),
        ]),
    );

    markdown::write_markdown(
        "memory://roundtrip-test",
        Some(&frontmatter),
        "# RT\n\nContent",
        &env.cfg,
    )
    .expect("write roundtrip");

    let (read_fm, read_content, _) =
        markdown::read_markdown("memory://roundtrip-test", &env.cfg).expect("read roundtrip");

    let read_fm = read_fm.expect("frontmatter");
    assert_eq!(
        yaml_string(&read_fm, "title").as_deref(),
        Some("Round Trip")
    );
    assert!(matches!(
        read_fm.get("tags"),
        Some(YamlValue::Sequence(tags))
            if tags
                == &vec![
                    YamlValue::String("test".to_string()),
                    YamlValue::String("boundary".to_string())
                ]
    ));
    assert!(read_content.contains("Content"));
}

// T-W1-2: RTM FR-28.3
#[test]
fn fr28_3_ac1_update_markdown_valid_checksum_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(
        &env.cfg,
        "memory://checksum-test",
        None,
        "# Original\n\nOriginal content",
    );
    let (frontmatter, _, checksum) =
        markdown::read_markdown("memory://checksum-test", &env.cfg).expect("read checksum");

    markdown::update_markdown(
        "memory://checksum-test",
        frontmatter.as_ref(),
        "# Updated\n\nNew content",
        Some(&checksum),
        &env.cfg,
    )
    .expect("update markdown");

    let (_, updated_content, _) =
        markdown::read_markdown("memory://checksum-test", &env.cfg).expect("read updated");
    assert!(updated_content.contains("New content"));
}

// T-W1-2: RTM FR-28.3
#[test]
fn fr28_3_ac2_update_markdown_stale_checksum_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(
        &env.cfg,
        "memory://stale-checksum-test",
        None,
        "# Original\n\nContent",
    );

    let err = markdown::update_markdown(
        "memory://stale-checksum-test",
        None,
        "# Tampered\n\nContent",
        Some("bogus-checksum-that-does-not-match"),
        &env.cfg,
    )
    .expect_err("stale checksum should fail");

    assert_message_error(err, "Checksum mismatch");
}

// T-W1-2: RTM FR-28.4
#[test]
fn fr28_4_ac1_append_h2_section_valid_uri_appends_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(
        &env.cfg,
        "memory://append-test",
        None,
        "# Base\n\nExisting content.",
    );

    markdown::append_h2_section(
        "memory://append-test",
        "New Section",
        "Appended content here.",
        &env.cfg,
    )
    .expect("append section");

    let (_, content, _) =
        markdown::read_markdown("memory://append-test", &env.cfg).expect("read appended file");
    assert!(content.contains("Existing content."));
    assert!(content.contains("## New Section"));
    assert!(content.contains("Appended content here."));
}

// T-W1-2: RTM FR-28.4
#[test]
fn fr28_4_ac2_append_h2_section_non_existent_uri_creates_file() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    markdown::append_h2_section(
        "memory://append-creates-file",
        "First Section",
        "Created from nothing.",
        &env.cfg,
    )
    .expect("append creates file");

    assert!(markdown::file_exists("memory://append-creates-file", &env.cfg).expect("exists"));
    let (_, content, _) =
        markdown::read_markdown("memory://append-creates-file", &env.cfg).expect("read created");
    assert!(content.contains("## First Section"));
    assert!(content.contains("Created from nothing."));
}
