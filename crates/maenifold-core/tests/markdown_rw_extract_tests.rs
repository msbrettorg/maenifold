#[path = "markdown_rw_support/mod.rs"]
mod markdown_rw_support;

use maenifold_core::markdown::extract_wikilinks;
use markdown_rw_support::count_concept_occurrences;

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

#[test]
fn extract_wikilinks_extracts_from_headings() {
    let content = "# Heading with [[neural-network]]\n\nBody with [[deep-learning]].";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"neural-network".to_string()));
    assert!(concepts.contains(&"deep-learning".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn extract_wikilinks_extracts_from_lists() {
    let content = "- Item with [[api-endpoint]]\n- Another item with [[database-schema]]\n\nText with [[cache-strategy]].";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"api-endpoint".to_string()));
    assert!(concepts.contains(&"database-schema".to_string()));
    assert!(concepts.contains(&"cache-strategy".to_string()));
    assert_eq!(concepts.len(), 3);
}

#[test]
fn extract_wikilinks_normalizes_concepts() {
    let content = "[[Machine Learning]] and [[machine-learning]] are the same.";

    let concepts = extract_wikilinks(content);

    assert_eq!(concepts.len(), 1);
    assert!(concepts.contains(&"machine-learning".to_string()));
}

#[test]
fn extract_wikilinks_skips_fenced_code_block_with_language() {
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

#[test]
fn extract_wikilinks_skips_indented_code_blocks() {
    let content = "Normal [[refactoring]].\n\n    This is indented code block\n    [[indented-code]]\n\nBack to normal [[code-review]].";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"refactoring".to_string()));
    assert!(concepts.contains(&"code-review".to_string()));
    assert!(!concepts.contains(&"indented-code".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn count_concept_occurrences_skips_code_blocks() {
    let content = "First [[logging]].\n\n```bash\n[[logging]] in code\n```\n\nSecond [[logging]].";

    let count = count_concept_occurrences(content, "logging");

    assert_eq!(count, 2);
}

#[test]
fn count_concept_occurrences_skips_inline_code() {
    let content =
        "First [[microservices]] and `[[microservices]]` in code and third [[microservices]].";

    let count = count_concept_occurrences(content, "microservices");

    assert_eq!(count, 2);
}

#[test]
fn count_concept_occurrences_handles_normalization() {
    let content = "[[API Design]] and [[api-design]] are the same.";

    let count = count_concept_occurrences(content, "api-design");

    assert_eq!(count, 2);
}
