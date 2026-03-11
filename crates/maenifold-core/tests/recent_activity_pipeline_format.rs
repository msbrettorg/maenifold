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
    assert!(output.contains("Thoughts:"));
    assert!(output.contains("Status: active"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_sequential_entry_include_content_true_shows_h2_content_fields() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/sequential/2026/02/session-fmt-content.md",
            "Content Session",
            "2026-02-18 11:00:00",
            "# Content Session\n\n## Thought 1/2\nFirst prose.\n\n## Thought 2/2\nSecond prose.",
            "active",
        )],
        true,
    );

    assert!(output.contains("First H2:"));
    assert!(output.contains("First H2 Content:"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_sequential_entry_include_content_false_shows_snippet() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://thinking/sequential/2026/02/session-fmt-snippet.md",
            "Snippet Session",
            "2026-02-18 12:00:00",
            "# Snippet Session\n\n## Thought 1/3\nThought content for snippet.\n\n## Thought 2/3\nSecond thought.\n\n## Thought 3/3\nThird thought.",
            "active",
        )],
        false,
    );

    assert!(output.contains("First:"));
    assert!(!output.contains("First H2:"));
}

// T-2.3: RTM FR-4 / RTM-025
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
    assert!(output.contains("Steps:"));
    assert!(output.contains("Status: active"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_memory_entry_includes_title_and_sections() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://docs/memory-fmt-test.md",
            "Memory Format Test",
            "2026-02-18 14:00:00",
            "# Memory Format Test\n\n## Overview\nOverview content.\n\n## Details\nDetailed content here.",
            "active",
        )],
        false,
    );

    assert!(output.contains("memory-fmt-test"));
    assert!(output.contains("(memory)"));
    assert!(output.contains("Title: Memory Format Test"));
    assert!(output.contains("Sections:"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn format_activity_report_memory_entry_include_content_true_shows_h2_fields() {
    let output = format_recent_activity_report(
        vec![sample_record(
            "memory://docs/mem-content-test.md",
            "Memory Content Test",
            "2026-02-18 15:00:00",
            "# Memory Content Test\n\n## First Section\nFirst section content.\n\n## Last Section\nLast section content.",
            "active",
        )],
        true,
    );

    assert!(output.contains("First H2: First Section"));
    assert!(output.contains("First H2 Content:"));
    assert!(output.contains("Last H2: Last Section"));
}

// T-2.3: RTM FR-4 / RTM-025
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
}
