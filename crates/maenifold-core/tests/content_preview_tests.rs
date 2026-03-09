use maenifold_core::search::create_smart_preview;

// T-1.6: RTM NFR-1 - ported from ContentPreviewTests.cs sentence-aware preview coverage.

#[test]
fn create_smart_preview_short_content_returns_full_content() {
    let content = "Short content.";

    let result = create_smart_preview(content, 200, 50);

    assert_eq!(result, content);
}

#[test]
fn create_smart_preview_with_complete_sentence_truncates_at_sentence() {
    let content = "First sentence. Second sentence. Third sentence that goes on and on.";

    let result = create_smart_preview(content, 30, 20);

    assert_eq!(result, "First sentence. Second sentence.");
}

#[test]
fn create_smart_preview_no_sentence_boundary_uses_paragraph() {
    let content = "First paragraph without ending\n\nSecond paragraph here";

    let result = create_smart_preview(content, 20, 15);

    assert!(result.ends_with("..."), "unexpected preview: {result}");
    assert!(
        result.contains("First paragraph"),
        "unexpected preview: {result}"
    );
}

#[test]
fn create_smart_preview_no_paragraph_boundary_uses_word_boundary() {
    let content = "ThisIsAVeryLongWordWithNoSpaces FollowedByMore Words";

    let result = create_smart_preview(content, 35, 10);

    assert!(result.ends_with("..."), "unexpected preview: {result}");
    assert!(
        !result.contains("FollowedByMore"),
        "word boundary fallback should exclude later words: {result}"
    );
}

#[test]
fn create_smart_preview_no_boundaries_hard_truncates() {
    let content = "NoSpacesOrPunctuationHereJustOneVeryLongString";

    let result = create_smart_preview(content, 20, 5);

    assert!(result.ends_with("..."), "unexpected preview: {result}");
    assert!(
        result.chars().count() <= 23,
        "hard truncate should keep preview within target + ellipsis: {result}"
    );
}

#[test]
fn create_smart_preview_with_wikilinks_preserves_links() {
    let content = "The [[authentication]] system uses [[JWT]] tokens. More content here.";

    let result = create_smart_preview(content, 60, 20);

    assert!(
        result.contains("[[authentication]]"),
        "expected wikilink preservation: {result}"
    );
    assert!(
        result.contains("[[JWT]]"),
        "expected wikilink preservation: {result}"
    );
}

#[test]
fn create_smart_preview_multiple_short_sentences_truncates_at_natural_boundary() {
    let content = "First. Second. Third. Fourth. Fifth. Sixth. Seventh.";

    let result = create_smart_preview(content, 25, 10);

    assert!(result.contains("First."), "unexpected preview: {result}");
    assert!(result.contains("Second."), "unexpected preview: {result}");
}

#[test]
fn create_smart_preview_sentence_ending_at_limit_returns_full_sentence() {
    let content = "This is exactly fifty characters for testing now. And more after.";

    let result = create_smart_preview(content, 50, 10);

    assert!(
        result.contains("This is exactly fifty characters for testing now."),
        "unexpected preview: {result}"
    );
    assert!(
        !result.contains("And more"),
        "preview should truncate before trailing sentence: {result}"
    );
}
