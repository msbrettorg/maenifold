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

// T-1.6: RTM NFR-1 — paragraph fallback when no sentence terminator exists.
#[test]
fn create_smart_preview_no_sentence_boundary_uses_paragraph() {
    let content = "First paragraph without ending\n\nSecond paragraph here";

    let result = create_smart_preview(content, 20, 15);

    // max_length=35, search_space contains \n\n at byte 30, paragraph_char_end=30 >= min_acceptable(5)
    assert_eq!(result, "First paragraph without ending...");
}

// T-1.6: RTM NFR-1 — word-boundary fallback when no sentence or paragraph boundary exists.
#[test]
fn create_smart_preview_no_paragraph_boundary_uses_word_boundary() {
    let content = "ThisIsAVeryLongWordWithNoSpaces FollowedByMore Words";

    let result = create_smart_preview(content, 35, 10);

    // max_length=45, no sentence or paragraph boundaries, rfind(' ')=31 → word truncation
    assert_eq!(result, "ThisIsAVeryLongWordWithNoSpaces...");
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

// T-1.6: RTM NFR-1 — wikilinks preserved when truncation fires at a sentence boundary.
#[test]
fn create_smart_preview_with_wikilinks_preserves_links() {
    let content = "The [[authentication]] system uses [[JWT]] tokens. More content here.";

    // target=30, tolerance=20 → max_length=50, min_acceptable=10
    // search_space (50 chars) ends at "tokens." → sentence boundary at index 49
    // 49 >= 10 → sentence truncation includes both wikilinks, excludes trailing sentence
    let result = create_smart_preview(content, 30, 20);

    assert_eq!(result, "The [[authentication]] system uses [[JWT]] tokens.");
    // Verify wikilinks survived truncation (redundant with assert_eq but documents intent)
    assert!(result.contains("[[authentication]]"));
    assert!(result.contains("[[JWT]]"));
    assert!(!result.contains("More content"));
}

// T-1.6: RTM NFR-1 — multiple sentence boundaries, algorithm picks last one in window.
#[test]
fn create_smart_preview_multiple_short_sentences_truncates_at_natural_boundary() {
    let content = "First. Second. Third. Fourth. Fifth. Sixth. Seventh.";

    let result = create_smart_preview(content, 25, 10);

    // max_length=35, sentence boundaries at indices 5,13,20,28; last valid=28 >= min_acceptable(15)
    assert_eq!(result, "First. Second. Third. Fourth.");
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

// T-1.6: RTM NFR-1 — sentence boundary below min_acceptable forces fallthrough to word boundary.
#[test]
fn create_smart_preview_sentence_below_minimum_falls_through_to_word() {
    // "Ok." ends at index 2 — the only sentence boundary in the search window.
    // min_acceptable = 25, so 2 < 25 → sentence fallback skipped, word boundary used.
    let content = "Ok. The quick brown fox jumps over the lazy dog without more periods";

    let result = create_smart_preview(content, 30, 5);

    // max_length=35, search_space has space at index 34 → rfind(' ')=34
    // word truncation: first 34 chars trimmed = "Ok. The quick brown fox jumps over"
    assert_eq!(result, "Ok. The quick brown fox jumps over...");
}

// T-1.6: RTM NFR-1 — '!' recognized as sentence terminator by find_last_sentence_boundary.
#[test]
fn create_smart_preview_exclamation_as_sentence_boundary() {
    let content = "What an amazing day! The rest continues with more text that goes on.";

    let result = create_smart_preview(content, 20, 10);

    // max_length=30, '!' at index 19, next char ' ' → boundary; 19 >= min_acceptable(10)
    assert_eq!(result, "What an amazing day!");
}

// T-1.6: RTM NFR-1 — '?' recognized as sentence terminator by find_last_sentence_boundary.
#[test]
fn create_smart_preview_question_mark_as_sentence_boundary() {
    let content = "Is this working yet? The rest of the content continues on and on.";

    let result = create_smart_preview(content, 20, 10);

    // max_length=30, '?' at index 19, next char ' ' → boundary; 19 >= min_acceptable(10)
    assert_eq!(result, "Is this working yet?");
}

// T-1.6: RTM NFR-1 — tolerance=0 means min_acceptable==target; sentence must be exactly at target.
#[test]
fn create_smart_preview_zero_tolerance_uses_word_boundary() {
    let content = "First sentence. Second sentence. Third sentence.";

    let result = create_smart_preview(content, 20, 0);

    // max_length=20, min_acceptable=20
    // search_space="First sentence. Seco" → sentence boundary '.' at index 14; 14 < 20 → fallthrough
    // no \n\n; rfind(' ')=15 → word boundary → "First sentence."
    assert_eq!(result, "First sentence....");
}

// T-1.6: RTM NFR-1 — target_length=0 produces hard truncation to just the ellipsis.
#[test]
fn create_smart_preview_zero_target_produces_ellipsis_only() {
    let content = "Some content here.";

    let result = create_smart_preview(content, 0, 0);

    // content_len=18 > 0 → enters truncation; max_length=0, all fallbacks fail
    // hard truncation: truncate_chars(content, 0) = "" → "..."
    assert_eq!(result, "...");
}
