use super::analysis::{load_help_doc, parse_repair_concept_variants};
use super::semantics::{semantic_validation_is_blocked, validate_repair_concepts_semantics};
use super::wikilinks::{create_wikilinks_for_plain_text_variants, replace_wikilink_variants};
use crate::error::{MaenifoldError, Result};
use crate::markdown::{get_memory_files, uri_to_path};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::Value;
use std::fs;

// T-3.1: RTM FR-6
pub fn repair_concepts(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let create_wikilinks = payload
        .get("createWikiLinks")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let variants = parse_repair_concept_variants(
        payload
            .get("conceptsToReplace")
            .and_then(Value::as_str)
            .ok_or(MaenifoldError::MissingProperty("conceptsToReplace"))?,
    );
    if variants.is_empty() {
        return Err(MaenifoldError::Message(
            "No concepts to replace provided".to_string(),
        ));
    }

    let canonical_concept = payload
        .get("canonicalConcept")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("canonicalConcept"))?
        .trim()
        .to_string();
    if learn {
        let help = load_help_doc(&context.config.assets_path, "RepairConcepts")
            .unwrap_or_else(|| "ERROR: Help file not found for RepairConcepts".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }
    if canonical_concept.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "canonicalConcept cannot be empty".to_string(),
        ));
    }
    if canonical_concept
        .chars()
        .any(|character| matches!(character, '[' | ']'))
    {
        return Err(MaenifoldError::InvalidArgument(
            "canonicalConcept cannot contain '[' or ']' characters".to_string(),
        ));
    }

    let folder_relative = payload
        .get("folder")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(|folder| folder.trim_matches('/').to_string())
        .filter(|folder| !folder.is_empty());
    let folder_uri = folder_relative
        .as_deref()
        .map(|folder| format!("memory://{folder}"));
    let dry_run = payload
        .get("dryRun")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let min_semantic_similarity = payload
        .get("minSemanticSimilarity")
        .and_then(Value::as_f64)
        .unwrap_or(0.7);

    if let Some(relative_folder) = folder_relative.as_deref() {
        let folder_path = context.config.memory_path.join(relative_folder);
        if !folder_path.exists() {
            return Err(MaenifoldError::Message(format!("Directory not found: {relative_folder}")));
        }
    }

    let semantic_validation_output = validate_repair_concepts_semantics(
        context,
        &variants,
        &canonical_concept,
        min_semantic_similarity,
    )?;
    // T-14.9: RTM-042 / RTM-043 — match the C# oracle by returning the warning
    // text as a successful response and halting before any file scan/modification.
    if semantic_validation_is_blocked(&semantic_validation_output) {
        return Ok(semantic_validation_output);
    }

    let file_uris = get_memory_files(&context.config, folder_uri.as_deref())?;
    let files_scanned = file_uris.len();
    let mut output = String::new();
    if !semantic_validation_output.is_empty() {
        output.push_str(&semantic_validation_output);
    }
    output.push_str(&format!(
        "Scanning {} markdown files...\nLooking for variants: {}\nWill replace with: [[{}]]\n\n",
        files_scanned,
        variants.join(", "),
        canonical_concept
    ));
    let mut files_modified = 0usize;
    let mut total_replacements = 0usize;

    for uri in file_uris {
        let path = uri_to_path(&uri, &context.config.memory_path)?;
        let original_content = fs::read_to_string(&path)?;
        let (updated_content, matched_variants, replacement_count) = if create_wikilinks {
            create_wikilinks_for_plain_text_variants(
                &original_content,
                &variants,
                &canonical_concept,
            )?
        } else {
            replace_wikilink_variants(&original_content, &variants, &canonical_concept)?
        };

        if replacement_count == 0 {
            continue;
        }

        files_modified += 1;
        total_replacements += replacement_count;

        let display_path = path
            .strip_prefix(&context.config.memory_path)
            .unwrap_or(path.as_path());
        // PARITY-R01-FMT: RTM NFR-1 — Strip .md extension from display paths (C# omits it)
        let display_str = display_path.display().to_string();
        let display_str = display_str.strip_suffix(".md").unwrap_or(&display_str);
        if dry_run {
            output.push_str(&format!("Would modify: {display_str}\n"));
        } else {
            fs::write(&path, updated_content)?;
            output.push_str(&format!("✓ Modified: {display_str}\n"));
        }

        // PARITY-R01-FMT: RTM NFR-1 — Wrap source in [[]] and add (normalized: ...) suffix
        for variant in matched_variants {
            output.push_str(&format!(
                "  [[{}]] → [[{}]] (normalized: {})\n",
                variant, canonical_concept, canonical_concept
            ));
        }
        output.push('\n');
    }

    output.push_str(&format!(
        "=== SUMMARY ===\nFiles scanned: {}\nFiles modified: {}\nTotal replacements: {}",
        files_scanned, files_modified, total_replacements
    ));
    if dry_run {
        output.push_str("\n\nThis was a DRY RUN. To apply changes, run with dryRun=false");
    }
    output
        .push_str("\nAfter applying changes, run 'sync' to rebuild the graph with clean concepts.");

    Ok(output)
}
