use maenifold_core::markdown::{extract_wikilinks, normalize_concept};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

// T-W1-3: RTM NFR-1 / NFR-2 — Ported WikiLinkRegexTests.cs coverage from the C# oracle.
fn count_concept_occurrences(content: &str, concept: &str) -> usize {
    let target = normalize_concept(concept);
    let mut count = 0usize;
    let mut visible = String::with_capacity(content.len());
    let mut code_block_depth = 0usize;
    let bytes = content.as_bytes();

    for (event, range) in Parser::new_ext(content, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_block_depth = code_block_depth.saturating_sub(1),
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            }) => {
                let has_clean_open = range.start == 0 || bytes[range.start - 1] != b'[';
                let has_clean_close = range.end < bytes.len() && bytes[range.end] == b']';
                let has_extra_close = range.end + 1 < bytes.len() && bytes[range.end + 1] == b']';

                if code_block_depth == 0
                    && has_clean_open
                    && has_clean_close
                    && !has_extra_close
                    && normalize_concept(dest_url.as_ref()) == target
                {
                    count += 1;
                }
            }
            Event::Code(_) => {}
            Event::Text(text) | Event::InlineMath(text) | Event::DisplayMath(text) => {
                if code_block_depth == 0 {
                    visible.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if code_block_depth == 0 {
                    visible.push('\n');
                }
            }
            _ => {}
        }
    }

    count + count_regex_wikilinks(&visible, &target)
}

// T-W1-3: RTM NFR-1 / NFR-2 — Mirrors the regex fallback so malformed bracket patterns stay excluded.
fn count_regex_wikilinks(content: &str, target: &str) -> usize {
    let bytes = content.as_bytes();
    let mut count = 0usize;
    let mut index = 0usize;

    while index + 3 < bytes.len() {
        if bytes[index] != b'[' || bytes[index + 1] != b'[' {
            index += 1;
            continue;
        }

        if index > 0 && bytes[index - 1] == b'[' {
            index += 1;
            continue;
        }

        let mut cursor = index + 2;
        let mut closing = None;
        let mut invalid = false;

        while cursor + 1 < bytes.len() {
            match (bytes[cursor], bytes[cursor + 1]) {
                (b']', b']') => {
                    closing = Some(cursor);
                    break;
                }
                (b'[', _) | (b']', _) => {
                    invalid = true;
                    break;
                }
                _ => cursor += 1,
            }
        }

        let Some(end) = closing else {
            index += 2;
            continue;
        };

        if invalid || (end + 2 < bytes.len() && bytes[end + 2] == b']') {
            index = end + 2;
            continue;
        }

        if normalize_concept(&content[index + 2..end]) == target {
            count += 1;
        }
        index = end + 2;
    }

    count
}

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
    // T-W1-3: RTM NFR-1 / NFR-2 — Quadruple-bracket patterns must not leak bracket characters.
    let content = "This has [[[[quadruple]]]] brackets.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.iter().all(|concept| !concept.contains('[')));
    assert!(concepts.iter().all(|concept| !concept.contains(']')));
}

#[test]
fn extract_wikilinks_extracts_only_valid_concepts() {
    // T-W1-3: RTM NFR-1 / NFR-2 — Mixed valid and malformed patterns should preserve only valid WikiLinks.
    let content = r#"
Valid: [[neural-network]] and [[machine-learning]]
Malformed: [[[invalid]]] and [[[[also-invalid]]]]
More valid: [[data-science]]
"#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"neural-network".to_string()));
    assert!(concepts.contains(&"machine-learning".to_string()));
    assert!(concepts.contains(&"data-science".to_string()));
    assert!(concepts.iter().all(|concept| !concept.contains('[')));
    assert!(concepts.iter().all(|concept| !concept.contains(']')));
}

#[test]
fn extract_wikilinks_handles_nested_brackets_correctly() {
    // T-W1-3: RTM NFR-1 / NFR-2 — Nested malformed brackets must not interfere with nearby valid links.
    let content = r#"
Normal: [[algorithm]]
Triple: [[[nested]]]
Prefix: text [[[before]]] after
Suffix: before [[[[after]]]] text
"#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"algorithm".to_string()));
    assert!(concepts.iter().all(|concept| !concept.contains('[')));
    assert!(concepts.iter().all(|concept| !concept.contains(']')));
}

#[test]
fn count_concept_occurrences_ignores_malformed_brackets() {
    // T-W1-3: RTM NFR-1 / NFR-2 — Occurrence counting should align with extraction and ignore malformed links.
    let content = r#"
Valid: [[regex-pattern]]
Invalid: [[[regex-pattern]]]
Valid again: [[regex-pattern]]
"#;

    let count = count_concept_occurrences(content, "regex-pattern");

    assert_eq!(count, 2, "Should count only valid [[WikiLink]] patterns");
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
