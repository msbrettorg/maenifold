#[path = "recent_activity_pipeline/support.rs"]
mod recent_activity_pipeline_support;

use recent_activity_pipeline_support::*;

// T-2.3: RTM FR-4 / RTM-027
#[test]
fn format_activity_report_empty_results_returns_no_activity_message() {
    let output = format_recent_activity_report(Vec::new(), false);
    assert_eq!(output, "No recent activity found");
}

// T-2.3: RTM FR-4 / RTM-025
// Rec #1: Strengthened count assertion ("Thoughts: 2" not just "Thoughts:")
// Rec #4: Added timestamp formatting assertion
#[test]
fn format_activity_report_sequential_entry_includes_expected_fields() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/sequential/2026/02/session-fmt-001.md",
            "Test Sequential Session",
            "2026-02-18 10:00:00",
            "# Test Sequential Session\n\n## Thought 1/2\nFirst thought content here.\n\n## Thought 2/2\nSecond thought content here.",
            "active",
        )],
        false,
    );

    assert!(output.contains("# Recent Activity"));
    assert!(output.contains("session-fmt-001"));
    assert!(output.contains("(sequential)"));
    assert!(
        output.contains("Thoughts: 2"),
        "expected 'Thoughts: 2', got:\n{output}"
    );
    assert!(output.contains("Status: active"));
    assert!(
        output.contains("Modified: 2026-02-18 10:00"),
        "expected timestamp 'Modified: 2026-02-18 10:00', got:\n{output}"
    );
}

// T-2.3: RTM FR-4 / RTM-025
// Rec #2: Merged include_content true/false into parameterized test
// Rec #3: Assert content values, not just labels
#[test]
fn format_activity_report_sequential_entry_include_content_toggle() {
    let content = "# Content Session\n\n## Thought 1/3\nFirst prose.\n\n## Thought 2/3\nSecond prose.\n\n## Thought 3/3\nThird prose.";

    // (include_content, expected_present, expected_absent)
    let cases: &[(bool, &[&str], &[&str])] = &[
        (
            true,
            &[
                "First H2: Thought 1/3",
                "First H2 Content: First prose.",
                "Last H2: Thought 3/3",
                "Last H2 Content: Third prose.",
                "Thoughts: 3",
            ],
            &[
                "First: \"", // snippet form should not appear
            ],
        ),
        (
            false,
            &[
                "First: \"", // snippet form should appear
                "Thoughts: 3",
            ],
            &[
                "First H2:", // content detail form should not appear
                "First H2 Content:",
            ],
        ),
    ];

    for (include_content, expected_present, expected_absent) in cases {
        let output = format_recent_activity_report(
            vec![sample_record(
                "memory://thinking/sequential/2026/02/session-fmt-content.md",
                "Content Session",
                "2026-02-18 11:00:00",
                content,
                "active",
            )],
            *include_content,
        );

        for expected in *expected_present {
            assert!(
                output.contains(expected),
                "include_content={include_content}: expected '{expected}' in output:\n{output}"
            );
        }
        for unexpected in *expected_absent {
            assert!(
                !output.contains(unexpected),
                "include_content={include_content}: unexpected '{unexpected}' in output:\n{output}"
            );
        }
    }
}

// T-2.3: RTM FR-4 / RTM-025
// Rec #1: Strengthened count assertion ("Steps: 2" not just "Steps:")
// Rec #4: Added timestamp formatting assertion
#[test]
fn format_activity_report_workflow_entry_includes_steps_and_status() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/workflow/2026/02/wf-fmt-001.md",
            "Workflow Fmt Test",
            "2026-02-18 13:00:00",
            "# Workflow Fmt Test\n\n## Step 1: Init\nInitialization response.\n\n## Response\nFinal result.",
            "active",
        )],
        false,
    );

    assert!(output.contains("wf-fmt-001"));
    assert!(output.contains("(workflow)"));
    assert!(
        output.contains("Steps: 2"),
        "expected 'Steps: 2', got:\n{output}"
    );
    assert!(output.contains("Status: active"));
    assert!(
        output.contains("Modified: 2026-02-18 13:00"),
        "expected timestamp, got:\n{output}"
    );
}

// T-2.3: RTM FR-4 / RTM-025
// Rec #2: Merged memory include_content true/false into parameterized test
// Rec #1: Strengthened section count assertion ("Sections: 2")
// Rec #3: Assert content values, not just labels
#[test]
fn format_activity_report_memory_entry_include_content_toggle() {
    let content = "# Memory Format Test\n\n## Overview\nOverview content.\n\n## Details\nDetailed content here.";

    // (include_content, expected_present, expected_absent)
    let cases: &[(bool, &[&str], &[&str])] = &[
        (
            false,
            &[
                "memory-fmt-test",
                "(memory)",
                "Title: Memory Format Test",
                "Sections: 2",
                "First: \"", // snippet form
            ],
            &["First H2:", "First H2 Content:"],
        ),
        (
            true,
            &[
                "memory-fmt-test",
                "(memory)",
                "Title: Memory Format Test",
                "Sections: 2",
                "First H2: Overview",
                "First H2 Content: Overview content.",
                "Last H2: Details",
                "Last H2 Content: Detailed content here.",
            ],
            &[
                "First: \"", // snippet form should not appear
            ],
        ),
    ];

    for (include_content, expected_present, expected_absent) in cases {
        let output = format_recent_activity_report(
            vec![sample_record(
                "memory://docs/memory-fmt-test.md",
                "Memory Format Test",
                "2026-02-18 14:00:00",
                content,
                "active",
            )],
            *include_content,
        );

        for expected in *expected_present {
            assert!(
                output.contains(expected),
                "include_content={include_content}: expected '{expected}' in output:\n{output}"
            );
        }
        for unexpected in *expected_absent {
            assert!(
                !output.contains(unexpected),
                "include_content={include_content}: unexpected '{unexpected}' in output:\n{output}"
            );
        }
    }
}

// T-2.3: RTM FR-4 / RTM-025
// Rec #5: Added ordering assertion (session-multi-a before note-multi-b)
#[test]
fn format_activity_report_multiple_entries_all_entities_present() {
    let output = format_recent_activity_report(
        vec![
            sample_record(
                "memory://thinking/sequential/2026/02/session-multi-a.md",
                "Session A",
                "2026-02-18 16:00:00",
                "# Session A\n\n## Thought 1/1\nA thought.",
                "active",
            ),
            sample_record(
                "memory://docs/note-multi-b.md",
                "Note B",
                "2026-02-18 15:00:00",
                "# Note B\n\n## Section B\nSome note content.",
                "active",
            ),
        ],
        false,
    );

    assert!(output.contains("session-multi-a"));
    assert!(output.contains("note-multi-b"));
    assert!(output.contains("(sequential)"));
    assert!(output.contains("(memory)"));

    // Rec #5: Verify ordering — session-multi-a (later timestamp) appears before note-multi-b
    let pos_a = output
        .find("session-multi-a")
        .expect("session-multi-a not found");
    let pos_b = output.find("note-multi-b").expect("note-multi-b not found");
    assert!(
        pos_a < pos_b,
        "expected session-multi-a before note-multi-b, got positions {pos_a} vs {pos_b} in:\n{output}"
    );
}

// T-2.3: RTM FR-4 / RTM-025
// Rec #6: Thinking type path — memory://thinking/ prefix without /sequential/ or /workflow/
#[test]
fn format_activity_report_thinking_entry_classified_as_thinking() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/2026/02/thought-note-001.md",
            "Standalone Thinking Note",
            "2026-02-18 17:00:00",
            "# Standalone Thinking Note\n\n## Observation\nSome observation.\n\n## Conclusion\nSome conclusion.",
            "active",
        )],
        false,
    );

    // Thinking type uses format_memory_entry: emits (thinking) label, Title, Sections
    assert!(output.contains("thought-note-001"));
    assert!(
        output.contains("(thinking)"),
        "expected '(thinking)' type label, got:\n{output}"
    );
    assert!(output.contains("Title: Standalone Thinking Note"));
    assert!(
        output.contains("Sections: 2"),
        "expected 'Sections: 2', got:\n{output}"
    );
    assert!(
        output.contains("Modified: 2026-02-18 17:00"),
        "expected timestamp, got:\n{output}"
    );
}

#[test]
fn format_activity_report_prefers_frontmatter_summaries_for_first_and_last() {
    let output = format_recent_activity_report(
        vec![sample_record_with_summaries(
            "memory://thinking/sequential/2026/02/session-frontmatter-001.md",
            "Frontmatter Sequential Session",
            "2026-02-18 18:00:00",
            "# Frontmatter Sequential Session\n\n## Thought 1/2\nBody first content.\n\n## Thought 2/2\nBody last content.",
            "active",
            Some("Thought 1/2: Frontmatter first summary"),
            Some("Thought 2/2: Frontmatter last summary"),
        )],
        true,
    );

    assert!(
        output.contains("First H2: Thought 1/2"),
        "expected heading derived from first summary, got:\n{output}"
    );
    assert!(
        output.contains("First H2 Content: Thought 1/2: Frontmatter first summary"),
        "expected first frontmatter summary content, got:\n{output}"
    );
    assert!(
        output.contains("Last H2: Thought 2/2"),
        "expected heading derived from last summary, got:\n{output}"
    );
    assert!(
        output.contains("Last H2 Content: Thought 2/2: Frontmatter last summary"),
        "expected last frontmatter summary content, got:\n{output}"
    );
    assert!(
        !output.contains("First H2 Content: Body first content."),
        "formatter should prefer frontmatter summary over parsed H2 body:\n{output}"
    );
    assert!(
        !output.contains("Last H2 Content: Body last content."),
        "formatter should prefer frontmatter summary over parsed H2 body:\n{output}"
    );
}
