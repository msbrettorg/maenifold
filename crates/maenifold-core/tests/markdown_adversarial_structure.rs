use maenifold_core::markdown::extract_wikilinks;

// T-2.3: RTM FR-4 / T-W1-3: RTM NFR-2 — Split adversarial structure and normalization coverage.
#[test]
fn extract_wikilinks_triple_brackets() {
    let content = r"Normal [[bracket-validation]] and [[[triple-attack]]] text.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"bracket-validation".to_string()));
    assert!(!concepts.contains(&"triple-attack".to_string()));
}

#[test]
fn extract_wikilinks_missing_closing_bracket() {
    let content = r"Normal [[malformed-input]] and [[missing-bracket text continues.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"malformed-input".to_string()));
    assert!(!concepts.contains(&"missing-bracket text continues".to_string()));
}

#[test]
fn extract_wikilinks_missing_opening_bracket() {
    let content = r"Normal [[edge-case-handling]] and wrong]] text.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"edge-case-handling".to_string()));
    assert_eq!(concepts.len(), 1);
}

#[test]
fn extract_wikilinks_empty_brackets() {
    let content = r"Normal [[whitespace-validation]] and [[]] and [[another]].";

    let concepts = extract_wikilinks(content)
        .into_iter()
        .filter(|concept| !concept.trim().is_empty())
        .collect::<Vec<_>>();

    assert!(concepts.contains(&"whitespace-validation".to_string()));
    assert!(concepts.contains(&"another".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn extract_wikilinks_whitespace_only_brackets() {
    let content = r"Normal [[input-sanitization]] and [[   ]] and [[another]].";

    let concepts = extract_wikilinks(content)
        .into_iter()
        .filter(|concept| !concept.trim().is_empty())
        .collect::<Vec<_>>();

    assert!(concepts.contains(&"input-sanitization".to_string()));
    assert!(concepts.contains(&"another".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn extract_wikilinks_normalization_collision() {
    let content = r"[[Test Concept]] and [[test___concept]] and [[TEST//CONCEPT]].";

    let concepts = extract_wikilinks(content);

    assert_eq!(concepts.len(), 1);
    assert!(concepts.contains(&"test-concept".to_string()));
}

#[test]
fn extract_wikilinks_leading_trailing_hyphens() {
    let content = r"[[--data-model--]] and [[data-model]].";

    let concepts = extract_wikilinks(content);

    assert_eq!(concepts.len(), 1);
    assert!(concepts.contains(&"data-model".to_string()));
}

#[test]
fn extract_wikilinks_multiple_adjacent_hyphens() {
    let content = r"[[test-----concept]] and [[test-concept]].";

    let concepts = extract_wikilinks(content);

    assert_eq!(concepts.len(), 1);
    assert!(concepts.contains(&"test-concept".to_string()));
}

#[test]
fn extract_wikilinks_only_code_blocks() {
    let content = r#"
```
[[attack1]]
```

```
[[attack2]]
```"#;

    let concepts = extract_wikilinks(content);

    assert_eq!(concepts.len(), 0);
}
