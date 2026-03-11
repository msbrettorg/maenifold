use super::support::{
    load_help_doc, normalize_relative_path, resolve_packaged_assets_path, sha256_digest,
    target_metadata,
};
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

pub fn update_assets(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "UpdateAssets")
            .unwrap_or_else(|| "ERROR: Help file not found for UpdateAssets".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let dry_run = payload
        .get("dryRun")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let source_path = resolve_packaged_assets_path()?;
    if !source_path.is_dir() {
        return Err(MaenifoldError::Message(
            "Source assets directory not found".to_string(),
        ));
    }

    let target_path = context.config.assets_path.clone();
    if !target_path.is_dir() {
        return Err(MaenifoldError::Message(
            "Target assets directory does not exist. Run initialization first.".to_string(),
        ));
    }

    // T-1.3: RTM FR-3, RTM-014, RTM-015, RTM-017, RTM-019
    let mut comparison = compare_asset_directories(&source_path, &target_path)?;
    if !dry_run {
        // T-1.3: RTM FR-3, RTM-016
        apply_asset_updates(&source_path, &target_path, &mut comparison)?;
    }

    let summary = render_update_summary(&comparison, dry_run);
    Ok(match context.output_format {
        OutputFormat::Json => success(json!({
            "dryRun": dry_run,
            "status": comparison.status(),
            "hasErrors": comparison.has_errors(),
            "addedFiles": comparison.added_files,
            "updatedFiles": comparison.updated_files,
            "skippedSymlinks": comparison.skipped_symlinks,
            "errors": comparison.errors,
            "summary": summary,
        })),
        OutputFormat::Markdown => summary,
    })
}

#[derive(Debug, Default)]
struct AssetComparison {
    added_files: Vec<String>,
    updated_files: Vec<String>,
    skipped_symlinks: Vec<String>,
    errors: Vec<String>,
}

impl AssetComparison {
    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn status(&self) -> &'static str {
        if self.has_errors() {
            "partial_failure"
        } else {
            "success"
        }
    }
}

fn compare_asset_directories(source_path: &Path, target_path: &Path) -> Result<AssetComparison> {
    let mut comparison = AssetComparison::default();

    for entry in walkdir::WalkDir::new(source_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let source_file = entry.path();
        let relative_path = source_file
            .strip_prefix(source_path)
            .expect("walkdir entry must be under source root");
        let target_file = target_path.join(relative_path);
        let relative_display = normalize_relative_path(relative_path);

        // T-1.5: RTM FR-3 - inspect target paths without following symlinks.
        let Some(target_metadata) = target_metadata(&target_file)? else {
            comparison.added_files.push(relative_display);
            continue;
        };

        if target_metadata.file_type().is_symlink() {
            record_skipped_symlink(&mut comparison, &relative_display);
            continue;
        }

        let source_metadata = fs::metadata(source_file)?;
        if source_metadata.len() != target_metadata.len()
            || sha256_digest(source_file)? != sha256_digest(&target_file)?
        {
            comparison.updated_files.push(relative_display);
        }
    }

    comparison.added_files.sort();
    comparison.updated_files.sort();
    comparison.skipped_symlinks.sort();
    comparison.errors.sort();
    Ok(comparison)
}

fn apply_asset_updates(
    source_path: &Path,
    target_path: &Path,
    comparison: &mut AssetComparison,
) -> Result<()> {
    let pending_files = comparison
        .added_files
        .iter()
        .chain(comparison.updated_files.iter())
        .cloned()
        .collect::<Vec<_>>();

    for relative_path in pending_files {
        let source_file = source_path.join(&relative_path);
        let target_file = target_path.join(&relative_path);

        // T-1.5: RTM FR-3 - re-check the destination before copy to block symlink swaps.
        match target_metadata(&target_file)? {
            Some(metadata) if metadata.file_type().is_symlink() => {
                record_skipped_symlink(comparison, &relative_path);
                continue;
            }
            _ => {}
        }

        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Err(error) = fs::copy(&source_file, &target_file) {
            comparison
                .errors
                .push(format!("{relative_path}: failed to copy file: {error}"));
        }
    }

    comparison.skipped_symlinks.sort();
    comparison.errors.sort();
    Ok(())
}

fn render_update_summary(comparison: &AssetComparison, dry_run: bool) -> String {
    if comparison.added_files.is_empty()
        && comparison.updated_files.is_empty()
        && comparison.skipped_symlinks.is_empty()
        && comparison.errors.is_empty()
    {
        return "# Asset Update Summary\n\nNo changes detected. All assets are up to date."
            .to_string();
    }

    let mut output = String::from("# Asset Update Summary\n\n## Results\n");
    let _ = writeln!(
        output,
        "- Status: {}",
        if comparison.has_errors() {
            "PARTIAL FAILURE"
        } else if dry_run {
            "DRY RUN"
        } else {
            "SUCCESS"
        }
    );
    let _ = writeln!(output, "- Added files: {}", comparison.added_files.len());
    let _ = writeln!(
        output,
        "- Updated files: {}",
        comparison.updated_files.len()
    );
    let _ = writeln!(
        output,
        "- Skipped symlinks: {}",
        comparison.skipped_symlinks.len()
    );
    let _ = writeln!(output, "- Errors: {}", comparison.errors.len());

    append_section(&mut output, "Added Files", &comparison.added_files);
    append_section(&mut output, "Updated Files", &comparison.updated_files);
    append_section(
        &mut output,
        "Skipped Symlink Targets",
        &comparison.skipped_symlinks,
    );
    append_section(&mut output, "Errors", &comparison.errors);

    if dry_run {
        output.push_str("\nThis was a DRY RUN. To apply changes, run with dryRun=false");
    }

    output
}

fn append_section(output: &mut String, heading: &str, items: &[String]) {
    let _ = writeln!(output, "\n### {heading}");
    if items.is_empty() {
        output.push_str("- None\n");
        return;
    }
    for item in items {
        let _ = writeln!(output, "- {item}");
    }
}

fn record_skipped_symlink(comparison: &mut AssetComparison, relative_path: &str) {
    let skipped_entry = relative_path.to_string();
    if !comparison.skipped_symlinks.contains(&skipped_entry) {
        comparison.skipped_symlinks.push(skipped_entry);
    }

    let error_entry = format!("{relative_path}: skipped symlink target");
    if !comparison.errors.contains(&error_entry) {
        comparison.errors.push(error_entry);
    }
}
