#[path = "markdown_boundary/support.rs"]
mod support;

use maenifold_core::ToolRegistry;
use maenifold_core::json::OutputFormat;
use serde_json::json;
use support::*;

// T-12.7: RTM NFR-1 - split markdown adversarial boundary coverage by theme.

// T-W1-2: RTM-183 — RepairConcepts error output must not leak the absolute
// memory path. The implementation deliberately formats the error with
// `relative_folder`, not `folder_path`; this test guards against regressions.
#[test]
fn rtm183_red1_concept_repair_tool_error_does_not_leak_filesystem_path() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // T-W1-2: RTM-183 — call try_invoke directly so we can inspect both Ok and
    // Err outcomes. The tool returns Err(Message(..)) for a nonexistent folder.
    let result = ToolRegistry::global().try_invoke(
        "RepairConcepts",
        &json!({
            "conceptsToReplace": "[\"test-concept\"]",
            "canonicalConcept": "test",
            "folder": "invalid-nonexistent-folder-12345"
        }),
        OutputFormat::Markdown,
    );

    // Extract user-visible text regardless of Ok/Err variant.
    let output = match result {
        Ok(text) => text,
        Err(err) => err.to_string(),
    };

    let memory_path = env.cfg.memory_path.to_string_lossy();
    assert!(
        !output.contains(memory_path.as_ref()),
        "error output must not leak filesystem memory path ({memory_path}): {output}"
    );
    assert!(
        output.contains("Directory not found"),
        "expected 'Directory not found' in error output: {output}"
    );
}

// T-W1-2: RTM-183 — validate_uri must reject any URI containing literal ".."
// path segments. This is the sole direct unit test of the traversal predicate;
// indirect coverage exists in markdown_boundary_validation.rs via higher-level
// APIs. Parameterized over several traversal shapes to guard the `.any()`
// short-circuit and leading/deep positions.
#[test]
fn rtm183_red2_validate_uri_rejects_dotdot_traversal_segments() {
    let cases: &[(&str, &str)] = &[
        ("memory://foo/../bar", "single mid-path traversal"),
        ("memory://foo/../../bar", "double mid-path traversal"),
        ("memory://../escape", "leading traversal"),
        (
            "memory://deep/path/../../../escape",
            "deep multi-hop traversal",
        ),
    ];
    for &(uri, label) in cases {
        let err = markdown::validate_uri(uri, "uri")
            .expect_err(&format!("expected traversal rejection for {label}: {uri}"));
        assert_invalid_argument(err, "path traversal");
    }
}

// T-W1-2: RTM-183 — validate_uri operates on the raw URI string and does NOT
// decode percent-encoding before checking for ".." segments. This test documents
// that assumption: %2e%2e passes validation because it is not literal "..".
//
// If an upstream caller (e.g. HTTP transport per FR-23 / T-17.2) decodes URIs
// before passing them to validate_uri, percent-encoded traversal would bypass
// the check. That transport must either reject encoded traversal at its layer
// or decode before calling validate_uri so this check still fires.
#[test]
fn rtm183_red3_validate_uri_does_not_decode_percent_encoded_traversal() {
    let encoded_traversals: &[&str] = &[
        "memory://%2e%2e/escape",  // fully encoded ..
        "memory://%2e./escape",    // mixed: %2e + literal .
        "memory://foo/%2e%2e/bar", // mid-path encoded
    ];
    for uri in encoded_traversals {
        assert!(
            markdown::validate_uri(uri, "uri").is_ok(),
            "validate_uri must accept percent-encoded input (no decode step): {uri}"
        );
    }
}
