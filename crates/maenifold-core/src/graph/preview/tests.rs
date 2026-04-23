// T-01.16: behavioural tests for the preview module. Imports through
// `super` so reorganisation of internal submodules stays transparent.

use super::extract_section_with_concept;

#[test]
fn centers_preview_on_mid_section_concept_mention() {
    let content = format!(
        "# Title\n\n## Notes\n\n{}\nThe focus is [[buildcontext]] because it keeps the graph traversal grounded.\n{}",
        "Intro filler. ".repeat(80),
        "Closing filler. ".repeat(80)
    );

    let preview = extract_section_with_concept(&content, "buildcontext", 160);

    assert!(preview.contains("[[buildcontext]]"));
    assert!(!preview.starts_with("## Notes"));
}

#[test]
fn prefers_body_mention_over_heading_only_match() {
    let content = format!(
        "# Title\n\n## BuildContext\n\n{}\nA later [[buildcontext]] mention should drive the preview output.\n{}",
        "Heading filler. ".repeat(40),
        "Trailing filler. ".repeat(40)
    );

    let preview = extract_section_with_concept(&content, "buildcontext", 140);

    assert!(preview.contains("later [[buildcontext]] mention"));
}

#[test]
fn fallback_body_preview_still_contains_plain_text_concept() {
    let content = format!(
        "{} Build context is the relevant phrase in the middle of the file. {}",
        "Lead text ".repeat(70),
        "Tail text ".repeat(70)
    );

    let preview = extract_section_with_concept(&content, "build-context", 120);

    assert!(preview.to_ascii_lowercase().contains("build context"));
}
