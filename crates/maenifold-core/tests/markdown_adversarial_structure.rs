use maenifold_core::markdown::extract_wikilinks;

// T-2.3: RTM FR-4 / T-W1-3: RTM NFR-2 — Adversarial structure and normalization coverage.

// MERGE(1,2,3): Parameterized — malformed bracket inputs are rejected.
#[test]
fn extract_wikilinks_rejects_malformed_brackets() {
    // T-2.3: RTM FR-4 — malformed bracket constructs must not produce concepts.
    let cases: Vec<(&str, Option<&str>, &str, usize)> = vec![
        // (input, rejected_concept_or_none, kept_concept, expected_len)
        (
            r"Normal [[bracket-validation]] and [[[triple-attack]]] text.",
            Some("triple-attack"),
            "bracket-validation",
            1,
        ),
        (
            r"Normal [[malformed-input]] and [[missing-bracket text continues.",
            Some("missing-bracket text continues"),
            "malformed-input",
            1,
        ),
        (
            r"Normal [[edge-case-handling]] and wrong]] text.",
            None,
            "edge-case-handling",
            1,
        ),
    ];

    for (i, (input, rejected, kept, expected_len)) in cases.iter().enumerate() {
        let concepts = extract_wikilinks(input);

        assert!(
            concepts.contains(&kept.to_string()),
            "case {i}: expected kept concept {kept:?} in {concepts:?}"
        );
        if let Some(bad) = rejected {
            assert!(
                !concepts.contains(&bad.to_string()),
                "case {i}: rejected concept {bad:?} should not appear in {concepts:?}"
            );
        }
        assert_eq!(
            concepts.len(),
            *expected_len,
            "case {i}: expected len {expected_len}, got {} — {concepts:?}",
            concepts.len()
        );
    }
}

// REWRITE(4): P0 — removed client-side filter that masked regression.
#[test]
fn extract_wikilinks_empty_brackets() {
    // T-2.3: RTM FR-4 — [[]] must not produce an empty-string concept.
    let content = r"Normal [[whitespace-validation]] and [[]] and [[another]].";

    let concepts = extract_wikilinks(content); // no filter — tests raw output

    assert!(concepts.contains(&"whitespace-validation".to_string()));
    assert!(concepts.contains(&"another".to_string()));
    assert_eq!(
        concepts.len(),
        2,
        "empty brackets [[]] must not produce a concept — got {concepts:?}"
    );
}

// REWRITE(5): P0 — removed client-side filter that masked regression.
#[test]
fn extract_wikilinks_whitespace_only_brackets() {
    // T-2.3: RTM FR-4 — [[   ]] must not produce an empty/whitespace concept.
    let content = r"Normal [[input-sanitization]] and [[   ]] and [[another]].";

    let concepts = extract_wikilinks(content); // no filter — tests raw output

    assert!(concepts.contains(&"input-sanitization".to_string()));
    assert!(concepts.contains(&"another".to_string()));
    assert_eq!(
        concepts.len(),
        2,
        "whitespace-only brackets [[   ]] must not produce a concept — got {concepts:?}"
    );
}

// KEEP(6): Load-bearing — tests deduplication + normalization interaction.
#[test]
fn extract_wikilinks_normalization_collision() {
    let content = r"[[Test Concept]] and [[test___concept]] and [[TEST//CONCEPT]].";

    let concepts = extract_wikilinks(content);

    assert_eq!(concepts.len(), 1);
    assert!(concepts.contains(&"test-concept".to_string()));
}

// MERGE(7,8): Parameterized — normalization collapses variants to canonical form.
#[test]
fn extract_wikilinks_normalizes_to_canonical_form() {
    // T-W1-3: RTM NFR-2 — normalization must collapse variant spellings.
    let cases: Vec<(&str, &str)> = vec![
        // (input_with_variant_and_canonical, expected_canonical)
        (r"[[--data-model--]] and [[data-model]].", "data-model"),
        (
            r"[[test-----concept]] and [[test-concept]].",
            "test-concept",
        ),
    ];

    for (i, (input, expected)) in cases.iter().enumerate() {
        let concepts = extract_wikilinks(input);

        assert_eq!(
            concepts.len(),
            1,
            "case {i}: expected exactly 1 concept, got {concepts:?}"
        );
        assert!(
            concepts.contains(&expected.to_string()),
            "case {i}: expected canonical {expected:?} in {concepts:?}"
        );
    }
}

// KEEP(9): Load-bearing — guards against fenced code-block concepts being indexed.
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

// P2 recommendation: inline code exclusion.
#[test]
fn extract_wikilinks_inline_code_excluded() {
    // T-2.3: RTM FR-4 — inline code `[[concept]]` must not be extracted.
    let content = r"Real [[valid-concept]] and `[[inline-code]]` text.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"valid-concept".to_string()));
    assert!(
        !concepts.contains(&"inline-code".to_string()),
        "inline code concept should not be extracted — got {concepts:?}"
    );
}

// P2 recommendation: nested brackets.
#[test]
fn extract_wikilinks_nested_brackets_rejected() {
    // T-2.3: RTM FR-4 — nested brackets must not produce phantom concepts.
    let content = r"Text [[outer [[inner]] end]] more text.";

    let concepts = extract_wikilinks(content);

    assert!(
        !concepts.contains(&"outer".to_string()),
        "nested bracket outer should not be extracted — got {concepts:?}"
    );
    // "inner" may or may not be extracted depending on parser behavior,
    // but "outer" and "outer [[inner]] end" must not appear as concepts.
    assert!(
        !concepts.contains(&"outer-inner-end".to_string()),
        "nested bracket composite should not be extracted — got {concepts:?}"
    );
}
