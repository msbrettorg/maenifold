#[path = "markdown_boundary/support.rs"]
mod support;

use serde_json::json;
use support::*;

// T-12.7: RTM NFR-1 - split markdown adversarial boundary coverage by theme.

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_red1_concept_repair_tool_error_does_not_leak_filesystem_path() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let output = invoke_repair_concepts(json!({
        "conceptsToReplace": "[\"test-concept\"]",
        "canonicalConcept": "test",
        "folder": "invalid-nonexistent-folder-12345"
    }));

    assert!(!output.contains(env.cfg.memory_path.to_string_lossy().as_ref()));
    assert!(
        output.contains("ERROR")
            || output.contains("Scanning 0 markdown files")
            || output.contains("Files scanned: 0"),
        "unexpected output: {output}"
    );
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_red5_validate_uri_traversal_single_dotdot_segment_throws() {
    let err = markdown::validate_uri("memory://foo/../bar", "uri")
        .expect_err("single traversal segment should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_red5_validate_uri_traversal_double_dotdot_segment_throws() {
    let err = markdown::validate_uri("memory://foo/../../bar", "uri")
        .expect_err("double traversal segment should fail");

    assert_invalid_argument(err, "path traversal");
}
