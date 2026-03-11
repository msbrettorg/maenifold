#[path = "frontmatter/support.rs"]
mod support;

use maenifold_core::markdown::{append_h2_section, read_markdown};
use maenifold_core::memory;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::thread;
use std::time::Duration as StdDuration;
use support::{
    TEST_MUTEX, extract_memory_uris, extract_uri_from_json, isolated_env, json_context,
    markdown_context, yaml_string,
};

// T-12.6: RTM NFR-1 — Split append/tool frontmatter regressions into a focused integration test file.
// T-META-001.6: RTM FR-24.x — Ported frontmatter metadata regression coverage from the C# oracle.

// T-META-001.6: RTM FR-24.2 — AppendH2Section routes through WriteMarkdown.
#[test]
fn fr24_2_ac1_append_h2_section_updates_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-append-modified";

    maenifold_core::markdown::write_markdown(file_uri, None, "# Session\n\n[[concept]]", &env.cfg)
        .expect("initial write");
    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read initial");
    let first_modified =
        yaml_string(&frontmatter.expect("frontmatter"), "modified").expect("modified");

    thread::sleep(StdDuration::from_millis(20));

    append_h2_section(file_uri, "Thought 1", "Some [[reasoning]] here.", &env.cfg)
        .expect("append section");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read appended");
    let frontmatter = frontmatter.expect("frontmatter");
    let second_modified = yaml_string(&frontmatter, "modified").expect("modified");
    assert!(!second_modified.is_empty());
    assert_ne!(second_modified, first_modified);
}

// T-META-001.6: RTM FR-24.2 — AppendH2Section preserves frontmatter.
#[test]
fn fr24_2_ac2_append_h2_section_preserves_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-append-preserve-fm";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("My Session".to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    maenifold_core::markdown::write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Session\n\n[[concept]]",
        &env.cfg,
    )
    .expect("write markdown");
    let (before_frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read initial");
    let before_frontmatter = before_frontmatter.expect("frontmatter");

    append_h2_section(
        file_uri,
        "New Section",
        "Content with [[wikilink]].",
        &env.cfg,
    )
    .expect("append section");

    let (after_frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read appended");
    let after_frontmatter = after_frontmatter.expect("frontmatter");
    assert_eq!(
        yaml_string(&after_frontmatter, "title").as_deref(),
        Some("My Session")
    );
    assert_eq!(
        yaml_string(&after_frontmatter, "type").as_deref(),
        Some("sequential")
    );
    assert_eq!(
        yaml_string(&after_frontmatter, "status").as_deref(),
        Some("active")
    );
    assert_eq!(
        yaml_string(&after_frontmatter, "created"),
        yaml_string(&before_frontmatter, "created")
    );
}

// T-META-001.6: RTM FR-24.2 — AppendH2Section preserves prior content and appends new content.
#[test]
fn fr24_2_ac3_append_h2_section_preserves_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-append-content";

    maenifold_core::markdown::write_markdown(
        file_uri,
        None,
        "# Session\n\nOriginal [[content]] here.",
        &env.cfg,
    )
    .expect("write markdown");

    append_h2_section(
        file_uri,
        "Appended Section",
        "New [[section]] text.",
        &env.cfg,
    )
    .expect("append section");

    let (_, content, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    assert!(content.contains("Original [[content]] here."));
    assert!(content.contains("## Appended Section"));
    assert!(content.contains("New [[section]] text."));
    assert!(
        content.find("## Appended Section").expect("new section")
            > content
                .find("Original [[content]] here.")
                .expect("original content")
    );
}

// T-META-001.6: RTM FR-24.3 — Tools delegate timestamps to MarkdownWriter.
#[test]
fn fr24_3_ac1_write_memory_timestamps_via_writer() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let output = memory::write_memory(
        &json_context(&env.cfg),
        &json!({
            "title": "Timestamp Via Writer Test",
            "content": "Testing that [[timestamp-management]] is delegated to MarkdownWriter.",
            "folder": "fr24-tests"
        }),
    )
    .expect("write memory");

    let uri = extract_uri_from_json(&output);
    let (frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(frontmatter.contains_key("modified"));
}

// T-META-001.6: RTM FR-24.3 — EditMemory updates modified via MarkdownWriter.
#[test]
fn fr24_3_ac2_edit_memory_modified_via_writer() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let write_output = memory::write_memory(
        &json_context(&env.cfg),
        &json!({
            "title": "Edit Modified Via Writer Test",
            "content": "Initial [[content]] for edit test.",
            "folder": "fr24-tests"
        }),
    )
    .expect("write memory");
    let uri = extract_uri_from_json(&write_output);

    let (frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read before edit");
    let modified_before =
        yaml_string(&frontmatter.expect("frontmatter"), "modified").expect("modified");

    thread::sleep(StdDuration::from_millis(20));

    let edit_output = memory::edit_memory(
        &markdown_context(&env.cfg),
        &json!({
            "identifier": uri,
            "operation": "append",
            "content": "\n\nAppended [[knowledge]] via EditMemory."
        }),
    )
    .expect("edit memory");
    assert!(
        edit_output.starts_with("Updated memory FILE:"),
        "unexpected output: {edit_output}"
    );

    let (frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read after edit");
    let frontmatter = frontmatter.expect("frontmatter");
    let modified_after = yaml_string(&frontmatter, "modified").expect("modified");
    assert!(!modified_after.is_empty());
    assert_ne!(modified_after, modified_before);
}

// T-META-001.6: RTM FR-24.3 — MoveMemory preserves a modified timestamp on the destination file.
#[test]
fn fr24_3_ac3_move_memory_modified_via_writer() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let write_output = memory::write_memory(
        &json_context(&env.cfg),
        &json!({
            "title": "Move Modified Via Writer Source",
            "content": "Content with [[concepts]] for move test.",
            "folder": "fr24-tests"
        }),
    )
    .expect("write memory");
    let source_uri = extract_uri_from_json(&write_output);

    let move_output = memory::move_memory(
        &markdown_context(&env.cfg),
        &json!({
            "source": source_uri,
            "destination": "fr24-tests/Move Modified Via Writer Dest"
        }),
    )
    .expect("move memory");
    assert!(
        move_output.starts_with("Moved memory FILE:"),
        "unexpected output: {move_output}"
    );

    let uris = extract_memory_uris(&move_output);
    let dest_uri = uris.last().expect("destination uri");
    let (frontmatter, _, _) = read_markdown(dest_uri, &env.cfg).expect("read moved file");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("modified"));
}
