use maenifold_core::markdown::{extract_wikilinks, normalize_concept};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

// T-2.3: RTM FR-4 / T-W1-3: RTM NFR-2 — Split adversarial MarkdownReader code-path coverage.
fn count_concept_occurrences(content: &str, concept: &str) -> usize {
    let target = normalize_concept(concept);
    let mut count = 0usize;
    let mut visible = String::with_capacity(content.len());
    let mut code_block_depth = 0usize;

    for event in Parser::new_ext(content, Options::all()) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_block_depth = code_block_depth.saturating_sub(1),
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            }) => {
                if code_block_depth == 0 && normalize_concept(dest_url.as_ref()) == target {
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

// T-2.3: RTM FR-4 — Mirrors markdown regex fallback behavior used by the production reader.
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

// T-2.3: RTM FR-4 — Adversarial code fence extraction coverage.
#[test]
fn extract_wikilinks_nested_code_blocks_outer_only() {
    let content = r#"
Valid [[markdown-parsing]].

```
Outer block with [[should-skip-outer]]
```
Inner attempt with [[should-skip-inner]]
```
```

Valid [[code-fence]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"markdown-parsing".to_string()));
    assert!(concepts.contains(&"code-fence".to_string()));
    assert!(!concepts.contains(&"should-skip-outer".to_string()));
    assert!(concepts.contains(&"should-skip-inner".to_string()));
    assert_eq!(concepts.len(), 3);
}

#[test]
fn extract_wikilinks_triple_backtick_inside_triple_backtick() {
    let content = r#"
Valid [[security-testing]].

```
Code block
```bash
[[nested-attack]]
```
More code
```

Another [[valid-concept]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"security-testing".to_string()));
    assert!(!concepts.contains(&"valid-concept".to_string()));
    assert!(!concepts.contains(&"nested-attack".to_string()));
    assert_eq!(concepts.len(), 1);
}

#[test]
fn extract_wikilinks_escaped_backticks() {
    let content = r"Normal [[regex-pattern]] and \`[[escaped-attack]]\` text.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"regex-pattern".to_string()));
    assert!(concepts.contains(&"escaped-attack".to_string()));
}

#[test]
fn extract_wikilinks_backslash_before_backtick() {
    let content = r"Normal [[string-escaping]] and \`code\` with [[potential-escape]].";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"string-escaping".to_string()));
    assert!(concepts.contains(&"potential-escape".to_string()));
}

#[test]
fn extract_wikilinks_inline_code_followed_by_code_block() {
    let content = r#"
Text with `[[inline-skip]]` and [[real-concept]].

```
[[block-skip]]
```

Final [[another-real]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"real-concept".to_string()));
    assert!(concepts.contains(&"another-real".to_string()));
    assert!(!concepts.contains(&"inline-skip".to_string()));
    assert!(!concepts.contains(&"block-skip".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn extract_wikilinks_multiple_inline_codes_in_same_paragraph() {
    let content = r"Text `[[skip1]]` more `[[skip2]]` and `[[skip3]]` but [[real]] here.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"real".to_string()));
    assert!(!concepts.contains(&"skip1".to_string()));
    assert!(!concepts.contains(&"skip2".to_string()));
    assert!(!concepts.contains(&"skip3".to_string()));
    assert_eq!(concepts.len(), 1);
}

#[test]
fn extract_wikilinks_html_code_elements() {
    let content = r"Normal [[html-parsing]] and <code>[[html-attack]]</code> text.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"html-parsing".to_string()));
}

#[test]
fn extract_wikilinks_html_pre_elements() {
    let content = r#"
Normal [[preformatted-text]].

<pre>
[[pre-attack]]
</pre>

Another [[valid]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"preformatted-text".to_string()));
    assert!(concepts.contains(&"valid".to_string()));
}

#[test]
fn extract_wikilinks_double_backtick_inline_code() {
    let content = r"Normal [[inline-code-handling]] and ``[[double-backtick]]`` text.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"inline-code-handling".to_string()));
    assert!(!concepts.contains(&"double-backtick".to_string()));
}

#[test]
fn extract_wikilinks_tilde_code_block() {
    let content = r#"
Valid [[fence-syntax]].

~~~
[[tilde-attack]]
~~~

Another [[valid]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"fence-syntax".to_string()));
    assert!(concepts.contains(&"valid".to_string()));
}

#[test]
fn extract_wikilinks_unclosed_code_block() {
    let content = r#"
Valid [[unclosed-fence]].

```
[[unclosed-attack]]

Another [[potentially-valid]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"unclosed-fence".to_string()));
}

#[test]
fn extract_wikilinks_code_block_with_extra_backticks() {
    let content = r#"
Valid [[fence-delimiters]].

`````
[[extra-backtick-attack]]
`````

Another [[valid]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"fence-delimiters".to_string()));
    assert!(concepts.contains(&"valid".to_string()));
    assert!(!concepts.contains(&"extra-backtick-attack".to_string()));
}

#[test]
fn count_concept_occurrences_null_byte_injection() {
    let content = "[[null-byte-handling]]\0[[attack]]";

    let count = count_concept_occurrences(content, "null-byte-handling");
    assert!(count >= 1);
}
