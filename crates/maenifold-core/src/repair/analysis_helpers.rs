use crate::markdown::normalize_concept;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

// T-3.1: RTM FR-6
pub(super) fn parse_repair_concept_variants(input: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut variants = Vec::new();

    for variant in input
        .split([',', ';', '|'])
        .map(normalize_concept)
        .filter(|variant| !variant.is_empty())
    {
        if seen.insert(variant.clone()) {
            variants.push(variant);
        }
    }

    variants
}

// T-1.4: RTM FR-2
pub(super) fn classify_corruption_variant(concept: &str) -> &'static str {
    let concept_lower = concept.to_ascii_lowercase();

    if is_file_path_variant(concept) {
        "File Paths"
    } else if is_plural_variant(concept) {
        "Singular/Plural"
    } else if concept.contains('-') {
        "Compound (with -)"
    } else if ["system", "framework", "architecture", "tool"]
        .iter()
        .any(|suffix| concept_lower.contains(suffix))
    {
        "With Suffix"
    } else {
        "Other"
    }
}

// T-1.4: RTM FR-2
pub(super) fn is_file_path_variant(concept: &str) -> bool {
    concept.starts_with('/') || concept.contains('.')
}

// T-1.4: RTM FR-2
pub(super) fn is_plural_variant(concept: &str) -> bool {
    let concept_lower = concept.to_ascii_lowercase();
    concept_lower.ends_with('s') && !concept_lower.ends_with("ss")
}

// T-3.1: RTM FR-6
pub(super) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}
