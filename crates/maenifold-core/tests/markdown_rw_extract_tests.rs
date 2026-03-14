use maenifold_core::markdown::extract_wikilinks;

// T-12.7.1: RTM NFR-1 — Split markdown extraction regressions by reader behavior.

#[test]
fn extract_wikilinks_skips_code_blocks() {
    let content = r#"Real [[authentication]] here.

```bash
# This [[fake-credential]] and $variable should NOT be extracted
echo "test"
```

Another [[database-connection]] outside."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"authentication".to_string()));
    assert!(concepts.contains(&"database-connection".to_string()));
    assert!(!concepts.contains(&"fake-credential".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn extract_wikilinks_skips_inline_code() {
    let content = "This is a [[machine-learning]] and `[[inline-code]]` in inline code.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"machine-learning".to_string()));
    assert!(!concepts.contains(&"inline-code".to_string()));
    assert_eq!(concepts.len(), 1);
}

// T-12.7.1: RTM NFR-1 — MERGE of extracts_from_headings + extracts_from_lists.
// Covers headings, list items, and block quotes in one fixture.
#[test]
fn extract_wikilinks_extracts_from_all_structural_contexts() {
    let content = r#"# Heading with [[neural-network]]

Body with [[deep-learning]].

- Item with [[api-endpoint]]
- Another item with [[database-schema]]

> Blockquote with [[cache-strategy]]"#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"neural-network".to_string()));
    assert!(concepts.contains(&"deep-learning".to_string()));
    assert!(concepts.contains(&"api-endpoint".to_string()));
    assert!(concepts.contains(&"database-schema".to_string()));
    assert!(concepts.contains(&"cache-strategy".to_string()));
    assert_eq!(concepts.len(), 5);
}

#[test]
fn extract_wikilinks_normalizes_concepts() {
    let content = "[[Machine Learning]] and [[machine-learning]] are the same.";

    let concepts = extract_wikilinks(content);

    assert_eq!(concepts.len(), 1);
    assert!(concepts.contains(&"machine-learning".to_string()));
}

// T-12.7.1: RTM NFR-1 — KEEP, renamed per eval: language-tagged fenced blocks may parse
// via a different token path in pulldown_cmark than bare fenced blocks.
#[test]
fn extract_wikilinks_skips_language_tagged_fenced_code_block() {
    let content = r#"Valid [[dependency-injection]] here.

```csharp
var x = "[[code-snippet]]";
```

Another [[unit-testing]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"dependency-injection".to_string()));
    assert!(concepts.contains(&"unit-testing".to_string()));
    assert!(!concepts.contains(&"code-snippet".to_string()));
    assert_eq!(concepts.len(), 2);
}

// T-12.7.1: RTM NFR-2 — REWRITE: 4-space indented code blocks are context-sensitive in
// CommonMark (require preceding blank line). Strengthened with control assertion.
#[test]
fn extract_wikilinks_skips_four_space_indented_code_block() {
    // Primary: blank line before indented block → treated as code → wikilink excluded.
    let content = "Normal [[refactoring]].\n\n    This is indented code block\n    [[indented-code]]\n\nBack to normal [[code-review]].";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"refactoring".to_string()));
    assert!(concepts.contains(&"code-review".to_string()));
    assert!(
        !concepts.contains(&"indented-code".to_string()),
        "indented wikilink inside code block must be excluded"
    );
    assert_eq!(concepts.len(), 2);

    // Control: without a preceding blank line, 4-space indent is a lazy paragraph
    // continuation — NOT a code block — so the wikilink must be extracted.
    let control = "Paragraph text\n    [[not-actually-code]]";
    let control_concepts = extract_wikilinks(control);
    assert!(
        control_concepts.contains(&"not-actually-code".to_string()),
        "four-space indent without preceding blank line is not a code block"
    );
}

// REMOVED: count_concept_occurrences_skips_code_blocks — guarded test helper, not production API.
// REMOVED: count_concept_occurrences_skips_inline_code — same; implicit coverage accepted.
// REMOVED: count_concept_occurrences_handles_normalization — same; per eval recommendation #2 option (b).
