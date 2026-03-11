use crate::error::{MaenifoldError, Result};
use crate::markdown::{
    extract_wikilinks, file_exists, find_file_as_uri, get_memory_files, normalize_concept,
    read_markdown,
};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

// T-3.1: RTM FR-6
pub fn extract_concepts_from_file(context: &ToolContext, payload: &Value) -> Result<String> {
    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    // T-16.3: RTM FR-25.2 — resolve identifier to URI, then check existence.
    // C# returns a successful "ERROR: ..." response for not-found; Rust must match.
    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        match find_file_as_uri(identifier, &context.config) {
            Some(found) => found,
            None => {
                return Ok(format!("ERROR: Memory file not found: {identifier}"));
            }
        }
    };

    if !file_exists(&uri, &context.config)? {
        return Ok(format!("ERROR: Memory file not found: {identifier}"));
    }

    let (_, content, _) = read_markdown(&uri, &context.config)?;
    // PARITY-E01: RTM NFR-1 — preserve document order (C# parity).
    // extract_wikilinks already returns concepts in document order with
    // deduplication on the normalized form.  We only need to deduplicate
    // again after lowercasing, using a HashSet to track seen values while
    // a Vec preserves insertion (document) order.  BTreeSet was previously
    // used here which silently re-sorted alphabetically.
    let mut seen = HashSet::new();
    let concepts: Vec<String> = extract_wikilinks(&content)
        .into_iter()
        .map(|concept| concept.to_ascii_lowercase())
        .filter(|concept| seen.insert(concept.clone()))
        .collect();

    // PARITY-E02-ZERO: RTM NFR-1 — C# outputs different message when no concepts found
    if concepts.is_empty() {
        return Ok("No [[WikiLink]] concepts found in file".to_string());
    }

    let mut output = format!("Found {} CONCEPTS in file:\n", concepts.len());
    for concept in &concepts {
        output.push_str(&format!("  • [[{}]]\n", concept));
    }
    Ok(output.trim_end().to_string())
}

// T-1.2: RTM FR-2
pub fn analyze_concept_corruption(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let concept_family = payload
        .get("conceptFamily")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("conceptFamily"))?
        .trim();
    if learn {
        let help = load_help_doc(&context.config.assets_path, "AnalyzeConceptCorruption")
            .unwrap_or_else(|| {
                "ERROR: Help file not found for AnalyzeConceptCorruption".to_string()
            });
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }
    if concept_family.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "conceptFamily cannot be empty".to_string(),
        ));
    }

    let max_results = match payload.get("maxResults") {
        Some(value) => match value.as_i64() {
            Some(parsed) if parsed < 0 => {
                return Err(MaenifoldError::InvalidArgument(
                    "maxResults must be zero or greater".to_string(),
                ));
            }
            Some(parsed) => parsed as usize,
            None => 50,
        },
        None => 50,
    };
    let concept_family_lower = concept_family.to_ascii_lowercase();

    let wikilink_pattern =
        Regex::new(r"\[\[([^\]]+)\]\]").expect("AnalyzeConceptCorruption wikilink regex");
    let mut counts = HashMap::<String, usize>::new();

    for uri in get_memory_files(&context.config, None)? {
        let (_, content, _) = read_markdown(&uri, &context.config)?;
        for capture in wikilink_pattern.captures_iter(&content) {
            let Some(raw_match) = capture.get(1) else {
                continue;
            };
            let original = raw_match.as_str().trim();
            if original.is_empty() {
                continue;
            }

            let normalized = normalize_concept(original);
            if normalized.contains(&concept_family_lower) {
                *counts.entry(original.to_string()).or_default() += 1;
            }
        }
    }

    let total_unique_variants = counts.len();
    let mut variants = counts.into_iter().collect::<Vec<_>>();
    variants.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| {
                left.0
                    .to_ascii_lowercase()
                    .cmp(&right.0.to_ascii_lowercase())
            })
            .then_with(|| left.0.cmp(&right.0))
    });
    variants.truncate(max_results);

    let mut grouped = BTreeMap::<&'static str, Vec<(String, usize)>>::new();
    for (concept, count) in &variants {
        grouped
            .entry(classify_corruption_variant(concept))
            .or_default()
            .push((concept.clone(), *count));
    }

    let mut output = format!(
        "=== Concept Family Analysis: '{}' ===\nFound {} unique variants",
        concept_family, total_unique_variants
    );

    for category in [
        "Singular/Plural",
        "Compound (with -)",
        "With Suffix",
        "File Paths",
        "Other",
    ] {
        let Some(entries) = grouped.get(category) else {
            continue;
        };
        if entries.is_empty() {
            continue;
        }

        output.push_str(&format!("\n\n{category}:\n"));
        for (concept, count) in entries {
            output.push_str(&format!("  • {concept} ({count}x)\n"));
        }
    }

    let plural_replacements = variants
        .iter()
        .filter_map(|(concept, _)| {
            (!is_file_path_variant(concept) && is_plural_variant(concept))
                .then_some(concept.clone())
        })
        .take(10)
        .collect::<Vec<_>>();
    let compound_replacements = variants
        .iter()
        .filter_map(|(concept, _)| {
            let concept_lower = concept.to_ascii_lowercase();
            (!is_file_path_variant(concept)
                && (concept_lower.contains(&format!("-{concept_family_lower}"))
                    || concept_lower.contains(&format!("{concept_family_lower}-"))))
            .then_some(concept.clone())
        })
        .take(10)
        .collect::<Vec<_>>();

    let plural_canonical = concept_family
        .strip_suffix(['s', 'S'])
        .filter(|value| !value.is_empty())
        .unwrap_or(concept_family);

    // PARITY-REPA02: RTM NFR-1 — Always emit SUGGESTED REPAIRS header (C# parity)
    output.push_str("\n\n=== SUGGESTED REPAIRS ===\n");
    if !plural_replacements.is_empty() || !compound_replacements.is_empty() {
        if !plural_replacements.is_empty() {
            output.push_str("Fix plural forms:\n");
            output.push_str(&format!(
                "  RepairConcepts conceptsToReplace='{}' canonicalConcept='{}'\n",
                plural_replacements.join(","),
                plural_canonical
            ));
        }
        if !compound_replacements.is_empty() {
            output.push_str("Fix compound forms:\n");
            output.push_str(&format!(
                "  RepairConcepts conceptsToReplace='{}' canonicalConcept='{}'\n",
                compound_replacements.join(","),
                concept_family
            ));
        }
    }

    Ok(output)
}

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

// T-1.2: RTM FR-2
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

// T-1.2: RTM FR-2
pub(super) fn is_file_path_variant(concept: &str) -> bool {
    concept.starts_with('/') || concept.contains('.')
}

// T-1.2: RTM FR-2
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
