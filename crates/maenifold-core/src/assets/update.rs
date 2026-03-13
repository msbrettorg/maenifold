use super::support::{
    load_help_doc, normalize_relative_path, resolve_packaged_assets_root, sha256_digest,
    target_metadata,
};
use super::types::AssetKind;
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
    // T-ASSET-PATH: RTM FR-3 — resolve packaged assets at runtime, not compile-time.
    let source_path = resolve_packaged_assets_root()
        .ok_or_else(|| MaenifoldError::Message("Source assets directory not found".to_string()))?;
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

// PARITY-U01: RTM NFR-1 — Only compare the 4 published asset catalogs
// (workflows/, roles/, colors/, perspectives/), matching C# scope.
// Directories like integrations/, models/, agents/, native/, usage/ are
// NOT updateable assets and must be excluded from the comparison.
fn compare_asset_directories(source_path: &Path, target_path: &Path) -> Result<AssetComparison> {
    let mut comparison = AssetComparison::default();

    // T-1.3: RTM FR-3 — walk only the published catalog directories.
    for kind in AssetKind::all() {
        let source_dir = source_path.join(kind.dir_name());
        if !source_dir.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&source_dir)
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

// T-1.3: RTM FR-3 — render markdown summary matching C# AssetUpdateResult.ToString() format.
// PARITY-U01: No Status/DRY RUN line, no Skipped symlinks count, omit empty sections,
// no trailing DRY RUN message.
fn render_update_summary(comparison: &AssetComparison, _dry_run: bool) -> String {
    if comparison.added_files.is_empty()
        && comparison.updated_files.is_empty()
        && comparison.skipped_symlinks.is_empty()
        && comparison.errors.is_empty()
    {
        return "# Asset Update Summary\n\nNo changes detected. All assets are up to date."
            .to_string();
    }

    let mut output = String::from("# Asset Update Summary\n\n");
    // T-1.3: RTM FR-3 — Results section: counts only, matching C# format.
    let _ = writeln!(output, "## Results");
    let _ = writeln!(output, "- Added files: {}", comparison.added_files.len());
    let _ = writeln!(
        output,
        "- Updated files: {}",
        comparison.updated_files.len()
    );
    let _ = writeln!(output, "- Errors: {}", comparison.errors.len());

    // T-1.3: RTM FR-3 — only emit sections that have content (C# parity).
    append_section_if_nonempty(&mut output, "Added Files", &comparison.added_files);
    append_section_if_nonempty(&mut output, "Updated Files", &comparison.updated_files);
    append_section_if_nonempty(&mut output, "Errors", &comparison.errors);

    output
}

// T-1.3: RTM FR-3 — C# only emits sections with content; empty sections are omitted.
fn append_section_if_nonempty(output: &mut String, heading: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(output, "\n### {heading}");
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
