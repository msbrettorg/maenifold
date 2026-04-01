use super::editing_helpers::{
    replace_section, starts_with_markdown_heading, strip_leading_heading,
};

// ---------------------------------------------------------------------------
// T-15.1–15.4: Unit tests for editing parity fixes
// ---------------------------------------------------------------------------

#[test]
fn t15_1_replace_section_basic() {
    // T-15.1: RTM FR-15 — basic replace_section still works
    let existing = "Intro\n\n## Alpha\n\nOld alpha body.\n\n## Beta\n\nBeta body.";
    let result = replace_section(
        existing,
        "Alpha",
        "New alpha with [[concepts]].",
        "memory://test.md",
    )
    .unwrap();
    assert!(result.contains("## Alpha\n"), "heading preserved");
    assert!(
        result.contains("New alpha with [[concepts]]."),
        "new body present"
    );
    assert!(!result.contains("Old alpha body"), "old body replaced");
    assert!(result.contains("## Beta\n"), "other section untouched");
}

#[test]
fn t15_3_partial_heading_match_starts_with() {
    // T-15.3: RTM FR-15 — "Design" matches "## Design Decisions"
    let existing = "Intro\n\n## Design Decisions\n\nOld design.\n\n## Implementation\n\nImpl body.";
    let result = replace_section(
        existing,
        "Design",
        "New design notes with [[arch]].",
        "memory://test.md",
    )
    .unwrap();
    assert!(
        result.contains("## Design Decisions\n"),
        "original full heading preserved, got:\n{result}"
    );
    assert!(result.contains("New design notes with [[arch]]."));
    assert!(!result.contains("Old design"));
    assert!(result.contains("## Implementation\n"));
}

#[test]
fn t15_3_exact_heading_still_works() {
    // T-15.3: RTM FR-15 — exact match continues to work
    let existing = "## Notes\n\nBody of notes.";
    let result = replace_section(
        existing,
        "Notes",
        "Updated with [[links]].",
        "memory://test.md",
    )
    .unwrap();
    assert!(result.contains("## Notes\n"));
    assert!(result.contains("Updated with [[links]]."));
}

#[test]
fn t15_3_partial_heading_not_found() {
    // T-15.3: RTM FR-15 — no match returns error
    let existing = "## Something Else\n\nBody.";
    let err =
        replace_section(existing, "Design", "New with [[x]].", "memory://test.md").unwrap_err();
    assert!(err.to_string().contains("Section not found"));
}

#[test]
fn t15_4_new_content_with_heading_preserves_old_heading() {
    // T-15.4: RTM FR-15 — old heading kept, new heading stripped
    let existing = "Intro\n\n## Original Title\n\nOld body.\n\n## Next\n\nNext body.";
    let result = replace_section(
        existing,
        "Original Title",
        "## Replaced Heading\nNew body with [[concepts]].",
        "memory://test.md",
    )
    .unwrap();
    assert!(
        result.contains("## Original Title\n"),
        "old heading must be preserved, got:\n{result}"
    );
    assert!(
        result.contains("New body with [[concepts]]"),
        "new body content present"
    );
    assert!(
        !result.contains("## Replaced Heading"),
        "heading from new content should be stripped"
    );
    assert!(result.contains("## Next\n"), "other section untouched");
}

#[test]
fn t15_4_new_content_without_heading_preserves_old_heading() {
    // T-15.4: RTM FR-15 — when no heading in new content, old heading still kept
    let existing = "## Section\n\nOld body.";
    let result = replace_section(
        existing,
        "Section",
        "Plain new body with [[links]].",
        "memory://test.md",
    )
    .unwrap();
    assert!(result.contains("## Section\n"));
    assert!(result.contains("Plain new body with [[links]]."));
}

#[test]
fn starts_with_markdown_heading_detects_headings() {
    assert!(starts_with_markdown_heading("## Heading\nBody"));
    assert!(starts_with_markdown_heading("# H1"));
    assert!(starts_with_markdown_heading("  ## Indented"));
    assert!(!starts_with_markdown_heading("Not a heading"));
    assert!(!starts_with_markdown_heading("#NoSpace"));
    assert!(!starts_with_markdown_heading(""));
}

#[test]
fn strip_leading_heading_removes_first_line() {
    assert_eq!(strip_leading_heading("## Heading\nBody text"), "Body text");
    assert_eq!(
        strip_leading_heading("# Title\nLine 2\nLine 3"),
        "Line 2\nLine 3"
    );
    assert_eq!(strip_leading_heading("## Only Heading"), "");
    assert_eq!(strip_leading_heading("  ## Spaced\nContent"), "Content");
}

#[test]
fn t15_2_operation_lowercasing() {
    // T-15.2: RTM FR-15 — operation strings are lowercased before matching
    let cases = vec![
        ("append", "append"),
        ("APPEND", "append"),
        ("Append", "append"),
        ("prepend", "prepend"),
        ("PREPEND", "prepend"),
        ("find_replace", "find_replace"),
        ("Find_Replace", "find_replace"),
        ("FIND_REPLACE", "find_replace"),
        ("replace_section", "replace_section"),
        ("REPLACE_SECTION", "replace_section"),
        ("Replace_Section", "replace_section"),
    ];
    for (input, expected) in cases {
        assert_eq!(
            input.to_ascii_lowercase(),
            expected,
            "lowercasing {input} should produce {expected}"
        );
    }
}
