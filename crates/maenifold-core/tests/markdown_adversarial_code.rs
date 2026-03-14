use maenifold_core::markdown::extract_wikilinks;

// T-2.3: RTM FR-4 / NFR-2 — Adversarial code fence / inline code extraction coverage.
// Every test follows the pattern: (a) positive assertion for extracted wikilinks,
// (b) negative assertion for suppressed wikilinks, (c) count to catch phantom extractions.

// T-2.3: RTM FR-4 — Wikilink appearing between two adjacent code fences (outside any block)
// is correctly extracted. The first fence opens and closes a block containing
// [[should-skip-outer]]. The text between the two fences is normal markdown.
#[test]
fn extract_wikilinks_between_adjacent_code_fences_is_extracted() {
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
    // should-skip-inner appears between two fences (outside any code block), so it IS extracted.
    assert!(concepts.contains(&"should-skip-inner".to_string()));
    assert_eq!(concepts.len(), 3);
}

// T-2.3: RTM FR-4 — A ```bash info-string line inside an open code block does NOT close
// the fence early. The outer ``` opens a block; the ```bash line is just content.
// The block extends until the next standalone ```.
#[test]
fn extract_wikilinks_code_block_with_info_string_suppresses_wikilinks() {
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

// T-2.3: RTM FR-4 — A backslash before a backtick (\`) does not create a code span.
// Wikilinks both inside and adjacent to \`...\` regions are extracted normally.
#[test]
fn extract_wikilinks_backslash_before_backtick_does_not_suppress() {
    // Case 1: wikilink inside \`...\` span — still extracted
    let content_inside = r"Normal [[regex-pattern]] and \`[[escaped-attack]]\` text.";
    let concepts_inside = extract_wikilinks(content_inside);

    assert!(concepts_inside.contains(&"regex-pattern".to_string()));
    assert!(concepts_inside.contains(&"escaped-attack".to_string()));
    assert_eq!(concepts_inside.len(), 2);

    // Case 2: wikilink after \`...\` span — still extracted
    let content_after = r"Normal [[string-escaping]] and \`code\` with [[potential-escape]].";
    let concepts_after = extract_wikilinks(content_after);

    assert!(concepts_after.contains(&"string-escaping".to_string()));
    assert!(concepts_after.contains(&"potential-escape".to_string()));
    assert_eq!(concepts_after.len(), 2);
}

// T-2.3: RTM FR-4 — KEEP: inline code AND fenced code block suppression combined.
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

// T-2.3: RTM FR-4 — KEEP: multiple inline code spans don't leak state.
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

// T-2.3: RTM FR-4 — HTML <code> elements. pulldown-cmark treats inline <code> tags as raw
// HTML, not as code spans, so wikilinks inside them are NOT suppressed by the parser.
// This test documents the actual extraction behavior: both wikilinks are extracted.
#[test]
fn extract_wikilinks_html_code_elements() {
    let content = r"Normal [[html-parsing]] and <code>[[html-attack]]</code> text.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"html-parsing".to_string()));
    // NOTE: <code> HTML tags do not suppress wikilink extraction — pulldown-cmark
    // does not treat raw HTML <code> as a code span. Both wikilinks are extracted.
    assert!(concepts.contains(&"html-attack".to_string()));
    assert_eq!(concepts.len(), 2);
}

// T-2.3: RTM FR-4 — HTML <pre> elements suppress wikilink extraction. pulldown-cmark
// treats <pre> blocks as HTML blocks, suppressing wikilink parsing inside them.
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
    assert!(!concepts.contains(&"pre-attack".to_string()));
    assert_eq!(concepts.len(), 2);
}

// T-2.3: RTM FR-4 — KEEP: double-backtick code spans.
#[test]
fn extract_wikilinks_double_backtick_inline_code() {
    let content = r"Normal [[inline-code-handling]] and ``[[double-backtick]]`` text.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"inline-code-handling".to_string()));
    assert!(!concepts.contains(&"double-backtick".to_string()));
}

// T-2.3: RTM FR-4 — Tilde (~~~) fenced code blocks suppress wikilinks.
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
    assert!(!concepts.contains(&"tilde-attack".to_string()));
    assert_eq!(concepts.len(), 2);
}

// T-2.3: RTM FR-4 — Unclosed code block: everything after the opening fence is suppressed.
// The wikilink before the fence is extracted; wikilinks inside and after the unclosed
// fence are treated as code block content and suppressed.
#[test]
fn extract_wikilinks_unclosed_code_block() {
    let content = r#"
Valid [[unclosed-fence]].

```
[[unclosed-attack]]

Another [[potentially-valid]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"unclosed-fence".to_string()));
    assert!(!concepts.contains(&"unclosed-attack".to_string()));
    assert!(!concepts.contains(&"potentially-valid".to_string()));
    assert_eq!(concepts.len(), 1);
}

// T-2.3: RTM FR-4 — KEEP: 5-backtick fence correctly suppresses content.
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
    assert_eq!(concepts.len(), 2);
}

// T-2.3: RTM FR-4 — Null byte in content does not corrupt wikilink extraction.
// Tests extract_wikilinks directly (not a test helper) to verify production behavior.
#[test]
fn extract_wikilinks_null_byte_in_content() {
    let content = "[[null-byte-handling]]\0[[attack]]";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"null-byte-handling".to_string()));
    // Null byte does not suppress subsequent wikilinks — both are extracted.
    assert!(concepts.contains(&"attack".to_string()));
    assert_eq!(concepts.len(), 2);
}
