#[path = "frontmatter/support.rs"]
mod support;

use maenifold_core::markdown::{append_h2_section, read_markdown};
use maenifold_core::memory;
use serde_json::json;
use std::thread;
use std::time::Duration as StdDuration;
use support::{
    TEST_MUTEX, extract_memory_uris, extract_uri_from_json, isolated_env, json_context,
    markdown_context, yaml_string,
};

// T-12.6: RTM NFR-1 — Split append/tool frontmatter regressions into a focused integration test file.
// T-META-001.6: RTM NFR-1 — Ported frontmatter metadata regression coverage from the C# oracle.

// T-META-001.6: RTM NFR-1 — AppendH2Section does not alter the `created` timestamp.
// Field-preservation assertions (title, type, status) are covered by
// markdown_rw_write_tests::append_h2_section_existing_file_preserves_all_frontmatter_fields.
#[test]
fn append_h2_section_preserves_created_timestamp() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-append-preserve-created";

    maenifold_core::markdown::write_markdown(file_uri, None, "# Session\n\n[[concept]]", &env.cfg)
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
        yaml_string(&after_frontmatter, "created"),
        yaml_string(&before_frontmatter, "created"),
        "append_h2_section must not alter the created timestamp"
    );
}

// T-META-001.6: RTM NFR-1 — AppendH2Section positions new content after existing content.
// Content-presence assertions (old content, new heading, new body) are covered by
// markdown_rw_write_tests::append_h2_section_existing_file_preserves_existing_content.
#[test]
fn append_h2_section_appends_after_existing_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-append-ordering";

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
    assert!(
        content
            .find("## Appended Section")
            .expect("new section present")
            > content
                .find("Original [[content]] here.")
                .expect("original content present"),
        "appended section must appear after the original content"
    );
}

// T-META-001.6: RTM NFR-1 — WriteMemory tool produces non-empty created and modified timestamps.
#[test]
fn write_memory_produces_created_and_modified_timestamps() {
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
    assert!(
        yaml_string(&frontmatter, "created").is_some_and(|v| !v.is_empty()),
        "created must be present and non-empty"
    );
    assert!(
        yaml_string(&frontmatter, "modified").is_some_and(|v| !v.is_empty()),
        "modified must be present and non-empty"
    );
}

// T-META-001.6: RTM NFR-1 — EditMemory updates modified via MarkdownWriter.
#[test]
fn edit_memory_advances_modified_timestamp() {
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

// T-META-001.6: RTM NFR-1 — MoveMemory produces a non-empty modified timestamp on the destination.
#[test]
fn move_memory_destination_has_nonempty_modified() {
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
    assert!(
        yaml_string(&frontmatter, "modified").is_some_and(|v| !v.is_empty()),
        "destination modified must be present and non-empty"
    );
}
