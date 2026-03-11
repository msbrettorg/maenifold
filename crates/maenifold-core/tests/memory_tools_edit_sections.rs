use maenifold_core::memory;
use serde_json::json;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, edit_memory_output, isolated_env, read_memory_output, write_memory_file,
};

// T-12.6: RTM NFR-1
#[test]
fn edit_memory_replace_section_replaces_existing_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Section Test",
        "This is the intro with a [[markdown-sections]].\n\n## First Section\n\nOld content in first section.\n\n## Second Section\n\nOld content in second section with [[document-structure]].\n\n## Third Section\n\nOld content in third section.",
        Some("memory-tools-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New content in second section with [[content-editing]].\n\nMultiple lines work too.",
            "sectionName": "Second Section"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("## Second Section"));
    assert!(read_result.contains("New content in second section with [[content-editing]]"));
    assert!(read_result.contains("Multiple lines work too."));
    assert!(!read_result.contains("Old content in second section"));
    assert!(!read_result.contains("[[document-structure]]"));
    assert!(read_result.contains("## First Section"));
    assert!(read_result.contains("Old content in first section."));
    assert!(read_result.contains("## Third Section"));
    assert!(read_result.contains("Old content in third section."));
}

#[test]
fn edit_memory_replace_section_throws_when_section_not_found() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Section Not Found Test",
        "This is the intro with a [[error-handling]].\n\n## Existing Section\n\nSome content here.",
        Some("memory-tools-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New content with [[validation]].",
            "sectionName": "Nonexistent Section"
        }),
    )
    .expect_err("missing section should error");

    assert!(err.to_string().contains("Section not found"));
    assert!(err.to_string().contains("Nonexistent Section"));
}

#[test]
fn edit_memory_replace_section_works_with_different_heading_levels() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Heading Levels Test",
        "Intro with [[markdown-hierarchy]].\n\n# Level 1 Heading\n\nContent under level 1.\n\n## Level 2 Heading\n\nContent under level 2.\n\n### Level 3 Heading\n\nContent under level 3.",
        Some("memory-tools-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New content for level 2 with [[section-replacement]].",
            "sectionName": "Level 2 Heading"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("## Level 2 Heading"));
    assert!(read_result.contains("New content for level 2 with [[section-replacement]]"));
    assert!(!read_result.contains("Content under level 2."));
    assert!(read_result.contains("# Level 1 Heading"));
    assert!(read_result.contains("Content under level 1."));
    assert!(read_result.contains("### Level 3 Heading"));
    assert!(read_result.contains("Content under level 3."));
}

#[test]
fn edit_memory_replace_section_replaces_last_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Last Section Test",
        "Intro with [[boundary-conditions]].\n\n## First Section\n\nFirst content.\n\n## Last Section\n\nThis is the last section content.",
        Some("memory-tools-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New last section content with [[document-end]].",
            "sectionName": "Last Section"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("## Last Section"));
    assert!(read_result.contains("New last section content with [[document-end]]"));
    assert!(!read_result.contains("This is the last section content."));
    assert!(read_result.contains("## First Section"));
    assert!(read_result.contains("First content."));
}
