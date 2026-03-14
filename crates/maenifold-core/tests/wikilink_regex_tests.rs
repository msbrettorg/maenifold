use maenifold_core::markdown::extract_wikilinks;

// T-W1-3: RTM NFR-1 / NFR-2 — Ported WikiLinkRegexTests.cs coverage from the C# oracle.
// Ghost infrastructure (count_concept_occurrences, count_regex_wikilinks) removed per eval:
// test-local reimplementations of production logic provided no real coverage.

#[test]
fn extract_wikilinks_rejects_triple_bracket_pattern() {
    // T-W1-3: RTM NFR-1 / NFR-2 — Triple-bracket patterns must not produce extracted concepts.
    let content = "This has a malformed [[[triple-attack]]] pattern.";

    let concepts = extract_wikilinks(content);

    assert!(
        concepts.is_empty(),
        "Triple bracket pattern should not extract any concepts"
    );
}

#[test]
fn extract_wikilinks_rejects_quadruple_bracket_pattern() {
    // T-W1-3: RTM NFR-1 / NFR-2 — Quadruple-bracket patterns must produce zero extracted concepts.
    // Contract: [[[[x]]]] is malformed and must be fully rejected (not just bracket-sanitized).
    let content = "This has [[[[quadruple]]]] brackets.";

    let concepts = extract_wikilinks(content);

    assert!(
        concepts.is_empty(),
        "Quadruple bracket pattern should not extract any concepts"
    );
}

#[test]
fn extract_wikilinks_extracts_only_valid_concepts() {
    // T-W1-3: RTM NFR-1 / NFR-2 — Mixed valid and malformed patterns should preserve only valid WikiLinks.
    // Merged: also covers nested/prefixed/suffixed malformed brackets (formerly test 4).
    let content = r#"
Valid: [[neural-network]] and [[machine-learning]]
Malformed triple: [[[invalid]]] and [[[[also-invalid]]]]
More valid: [[data-science]]
Nested triple: [[[nested]]]
Prefix triple: text [[[before]]] after
Suffix quadruple: before [[[[after]]]] text
Isolated valid: [[algorithm]]
"#;

    let concepts = extract_wikilinks(content);

    // Positive: all valid [[concept]] patterns extracted
    assert!(
        concepts.contains(&"neural-network".to_string()),
        "Valid concept neural-network must be extracted"
    );
    assert!(
        concepts.contains(&"machine-learning".to_string()),
        "Valid concept machine-learning must be extracted"
    );
    assert!(
        concepts.contains(&"data-science".to_string()),
        "Valid concept data-science must be extracted"
    );
    assert!(
        concepts.contains(&"algorithm".to_string()),
        "Valid concept algorithm must be extracted"
    );

    // Negative: malformed patterns must NOT be extracted
    assert!(
        !concepts.contains(&"invalid".to_string()),
        "Triple-bracket content must not be extracted"
    );
    assert!(
        !concepts.contains(&"also-invalid".to_string()),
        "Quadruple-bracket content must not be extracted"
    );
    assert!(
        !concepts.contains(&"nested".to_string()),
        "Triple-bracket nested content must not be extracted"
    );
    assert!(
        !concepts.contains(&"before".to_string()),
        "Triple-bracket prefixed content must not be extracted"
    );
    assert!(
        !concepts.contains(&"after".to_string()),
        "Quadruple-bracket suffixed content must not be extracted"
    );

    // Exact count guard: regression detector for unexpected extractions
    assert_eq!(
        concepts.len(),
        4,
        "Exactly 4 valid concepts expected; got {:?}",
        concepts
    );

    // No bracket leakage in any extracted concept
    assert!(concepts.iter().all(|c| !c.contains('[')));
    assert!(concepts.iter().all(|c| !c.contains(']')));
}

#[test]
fn extract_wikilinks_counts_only_valid_occurrences_ignoring_malformed() {
    // T-W1-3: RTM NFR-1 / NFR-2 — Replacement for removed count_concept_occurrences test.
    // Tests production extract_wikilinks directly: deduplication must hold, malformed rejected.
    let content = r#"
Valid: [[regex-pattern]]
Invalid: [[[regex-pattern]]]
Valid again: [[regex-pattern]]
"#;

    let concepts = extract_wikilinks(content);

    assert!(
        concepts.contains(&"regex-pattern".to_string()),
        "Valid wikilink must be extracted"
    );
    // extract_wikilinks deduplicates, so repeated valid references yield one entry.
    assert_eq!(
        concepts.len(),
        1,
        "Deduplication must hold; got {:?}",
        concepts
    );
}

#[test]
fn extract_wikilinks_allows_hyphens_underscores_in_concepts() {
    // T-W1-3: RTM NFR-1 / NFR-2 — Valid identifier characters should normalize the same way as the C# oracle.
    let content = r#"
Hyphenated: [[machine-learning]]
Underscored: [[neural_network]]
Mixed: [[deep-learning_model]]
Numbers: [[gpt-4]]
"#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"machine-learning".to_string()));
    assert!(concepts.contains(&"neural-network".to_string()));
    assert!(concepts.contains(&"deep-learning-model".to_string()));
    assert!(concepts.contains(&"gpt-4".to_string()));
}

// --- P4 Coverage Gap Tests ---

#[test]
fn extract_wikilinks_ignores_empty_brackets() {
    // T-W1-3: RTM NFR-2 — Empty [[]] must not extract a concept.
    let content = "Nothing here [[]] empty.";

    let concepts = extract_wikilinks(content);

    assert!(
        concepts.is_empty(),
        "Empty brackets should not extract any concept; got {:?}",
        concepts
    );
}

#[test]
fn extract_wikilinks_trims_whitespace_in_concept() {
    // T-W1-3: RTM NFR-2 — Leading/trailing whitespace inside brackets must be trimmed.
    let content = "Spaced [[ concept ]] here.";

    let concepts = extract_wikilinks(content);

    assert!(
        concepts.contains(&"concept".to_string()),
        "Whitespace-padded concept must be trimmed and extracted; got {:?}",
        concepts
    );
    assert_eq!(concepts.len(), 1);
}

#[test]
fn extract_wikilinks_deduplicates_repeated_concept() {
    // T-W1-3: RTM NFR-2 — Same concept appearing multiple times must yield one entry.
    let content = "First [[concept]] second [[concept]] done.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"concept".to_string()));
    assert_eq!(
        concepts.len(),
        1,
        "Duplicate concepts must be deduplicated; got {:?}",
        concepts
    );
}

#[test]
fn extract_wikilinks_ignores_wikilink_in_inline_code() {
    // T-W1-3: RTM NFR-2 — WikiLinks inside inline code spans must not be extracted.
    let content = "Inline `[[code-concept]]` not extracted.";

    let concepts = extract_wikilinks(content);

    assert!(
        concepts.is_empty(),
        "WikiLink inside inline code must not be extracted; got {:?}",
        concepts
    );
}

#[test]
fn extract_wikilinks_ignores_wikilink_in_fenced_code() {
    // T-W1-3: RTM NFR-2 — WikiLinks inside fenced code blocks must not be extracted.
    let content = "```\n[[fenced-concept]]\n```";

    let concepts = extract_wikilinks(content);

    assert!(
        concepts.is_empty(),
        "WikiLink inside fenced code block must not be extracted; got {:?}",
        concepts
    );
}

#[test]
fn extract_wikilinks_handles_pipe_alias_syntax() {
    // T-W1-3: RTM NFR-2 — [[concept|alias]] pipe syntax extracts both concept and display text.
    let content = "Alias [[concept|display]] here.";

    let concepts = extract_wikilinks(content);

    assert!(
        concepts.contains(&"concept".to_string()),
        "Pipe-syntax concept (dest) must be extracted; got {:?}",
        concepts
    );
    assert!(
        concepts.contains(&"display".to_string()),
        "Pipe-syntax display text must be extracted; got {:?}",
        concepts
    );
}
