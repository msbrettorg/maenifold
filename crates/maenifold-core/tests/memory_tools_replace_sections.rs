use maenifold_core::memory;
use serde_json::json;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, edit_memory_output, isolated_env, read_memory_output, write_memory_file,
};

// T-12.6: RTM NFR-1
#[test]
fn replace_section_when_section_exists_replaces_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Section Test",
        "# Introduction\n\nThis is the introduction section with [[documentation-structure]].\n\n## Methodology\n\nThis is the OLD methodology content that will be replaced.\n\n## Results\n\nThese are the results section with [[data]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "## Methodology\n\nThis is the NEW methodology content with [[improved-approach]] and [[better-techniques]].",
            "sectionName": "Methodology"
        }),
    );

    assert!(
        edit_result.contains(&uri),
        "expected URI in result: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("NEW methodology content"));
    assert!(read_result.contains("[[improved-approach]]"));
    assert!(read_result.contains("[[better-techniques]]"));
    assert!(!read_result.contains("OLD methodology content"));
    assert!(read_result.contains("introduction section with [[documentation-structure]]"));
    assert!(read_result.contains("results section with [[data]]"));
}

// T-12.6: RTM NFR-1
#[test]
fn replace_section_when_section_does_not_exist_throws_invalid_operation() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Missing Section Test",
        "# Introduction\n\nThis is the introduction with [[section-replacement]].\n\n## Results\n\nThese are the results with [[data]].",
        Some("edit-memory-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "## Methodology\n\nNew content with [[test]].",
            "sectionName": "Methodology"
        }),
    )
    .expect_err("missing section should error");

    assert!(err.to_string().contains("Section not found"));
    assert!(err.to_string().contains("Methodology"));
}

// T-12.6: RTM NFR-1
#[test]
fn replace_section_with_h1_heading_replaces_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "H1 Replace Test",
        "# First Section\n\nContent of first section with [[alpha]].\n\n# Second Section\n\nContent of second section with [[beta]].\n\n# Third Section\n\nContent of third section with [[gamma]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "# Second Section\n\nREPLACED content with [[delta]] and [[epsilon]].",
            "sectionName": "Second Section"
        }),
    );

    assert!(
        edit_result.contains(&uri),
        "expected URI in result: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("REPLACED content"));
    assert!(read_result.contains("[[delta]]"));
    assert!(!read_result.contains("[[beta]]"));
    assert!(read_result.contains("[[alpha]]"));
    assert!(read_result.contains("[[gamma]]"));
}

// T-12.6: RTM NFR-1
#[test]
fn replace_section_with_h3_heading_replaces_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "H3 Replace Test",
        "## Main Section\n\nMain content with [[primary]].\n\n### Subsection A\n\nOld subsection A content with [[old-concept]].\n\n### Subsection B\n\nSubsection B content with [[secondary]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "### Subsection A\n\nNew subsection A content with [[new-concept]] and [[innovative-idea]].",
            "sectionName": "Subsection A"
        }),
    );

    assert!(
        edit_result.contains(&uri),
        "expected URI in result: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("New subsection A content"));
    assert!(read_result.contains("[[new-concept]]"));
    assert!(!read_result.contains("[[old-concept]]"));
    assert!(read_result.contains("[[primary]]"));
    assert!(read_result.contains("[[secondary]]"));
}

// T-12.6: RTM NFR-1
#[test]
fn replace_section_old_content_completely_removed_not_appended() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Complete Removal Test",
        "# Test\n\nInitial content with [[initial-setup]].\n\n## Target Section\n\nOld paragraph one with [[deprecated-approach]].\nOld paragraph two with [[legacy-implementation]].\nOld paragraph three with [[outdated-pattern]].\n\n## Next Section\n\nNext section content with [[future-work]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "## Target Section\n\nBrand new single paragraph with [[brand-new]].",
            "sectionName": "Target Section"
        }),
    );

    assert!(
        edit_result.contains(&uri),
        "expected URI in result: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("Brand new single paragraph"));
    assert!(read_result.contains("[[brand-new]]"));
    assert!(!read_result.contains("Old paragraph one"));
    assert!(!read_result.contains("Old paragraph two"));
    assert!(!read_result.contains("Old paragraph three"));
    assert!(!read_result.contains("[[deprecated-approach]]"));
    assert!(!read_result.contains("[[legacy-implementation]]"));
    assert!(!read_result.contains("[[outdated-pattern]]"));
    assert!(read_result.contains("[[initial-setup]]"));
    assert!(read_result.contains("[[future-work]]"));
}

// T-12.6: RTM NFR-1 — migrated from memory_tools_edit_sections.rs
// Load-bearing: only integration test covering the EOF terminus boundary
// where replace_section targets the last section in a multi-section document.
#[test]
fn replace_section_replaces_last_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Last Section Test",
        "Intro with [[boundary-conditions]].\n\n## First Section\n\nFirst content.\n\n## Last Section\n\nThis is the last section content.",
        Some("edit-memory-tests"),
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
        edit_result.contains(&uri),
        "expected URI in result: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("## Last Section"));
    assert!(read_result.contains("New last section content with [[document-end]]"));
    assert!(!read_result.contains("This is the last section content."));
    assert!(read_result.contains("## First Section"));
    assert!(read_result.contains("First content."));
}
