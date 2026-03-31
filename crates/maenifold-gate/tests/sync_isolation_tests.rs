use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use maenifold_gate::{
    TaskLocation,
    pipeline::{PipelineDef, load_pipeline_from_str},
    sync::{extract_requirement, extract_title, resolve_task_state},
};
use tempfile::{TempDir, tempdir};

fn pipeline() -> PipelineDef {
    load_pipeline_from_str(
        r#"{
          "name":"test",
          "stages":[{"name":"red","verdicts":["pass","reject"],"transitions":{"pass":"done","reject":"pending"}}]
        }"#,
    )
    .unwrap()
}

fn setup_root() -> TempDir {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("todo")).unwrap();
    fs::create_dir_all(dir.path().join("done")).unwrap();
    dir
}

fn write_task(root: &Path, location: TaskLocation, name: &str, content: &str) -> PathBuf {
    let path = root.join(location.as_str()).join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn extract_requirement_finds_fr_requirement() {
    assert_eq!(
        extract_requirement("**Requirement**: FR-123\n"),
        Some("FR-123".into())
    );
}

#[test]
fn extract_requirement_finds_nfr_requirement() {
    assert_eq!(
        extract_requirement("**Requirement**: NFR-7 extra text\n"),
        Some("NFR-7".into())
    );
}

#[test]
fn extract_requirement_returns_none_when_missing() {
    assert_eq!(extract_requirement("# T-1 — Title\n"), None);
}

#[test]
fn extract_requirement_uses_first_valid_match() {
    let content = "**Requirement**: FR-3\n**Requirement**: NFR-9\n";
    assert_eq!(extract_requirement(content), Some("FR-3".into()));
}

#[test]
fn extract_requirement_rejects_malformed_values() {
    assert_eq!(extract_requirement("**Requirement**: FR-abc\n"), None);
}

#[test]
fn extract_title_parses_standard_heading() {
    assert_eq!(extract_title("# T-7 — Ship it\n"), Some("Ship it".into()));
}

#[test]
fn extract_title_keeps_heading_without_separator() {
    assert_eq!(
        extract_title("# Just a title\n"),
        Some("Just a title".into())
    );
}

#[test]
fn extract_title_does_not_split_on_hyphen_minus() {
    assert_eq!(
        extract_title("# T-7 - Ship it\n"),
        Some("T-7 - Ship it".into())
    );
}

#[test]
fn extract_title_parses_en_dash_separator() {
    assert_eq!(extract_title("# T-7 – Ship it\n"), Some("Ship it".into()));
}

#[test]
fn extract_title_preserves_unicode_content() {
    assert_eq!(
        extract_title("# T-7 — Résumé 🚀\n"),
        Some("Résumé 🚀".into())
    );
}

#[test]
fn extract_title_returns_none_for_empty_input() {
    assert_eq!(extract_title(""), None);
}

#[test]
fn extract_title_returns_none_without_heading() {
    assert_eq!(extract_title("Body only\n## Nested\n"), None);
}

#[test]
fn resolve_task_state_prefers_frontmatter_status_and_attempts() -> Result<()> {
    let root = setup_root();
    let path = write_task(
        root.path(),
        TaskLocation::Done,
        "T-1.md",
        "---\nstatus: red\nattempts: 2\n---\n\n# T-1 — Title\n\n**Requirement**: FR-1\n",
    );

    let state = resolve_task_state(&path, &fs::read_to_string(&path)?, &pipeline())?;
    assert_eq!(state.location, TaskLocation::Done);
    assert_eq!(state.status, "red");
    assert_eq!(state.attempts, 2);
    Ok(())
}

#[test]
fn resolve_task_state_without_frontmatter_uses_legacy_defaults() -> Result<()> {
    let root = setup_root();
    let path = write_task(
        root.path(),
        TaskLocation::Done,
        "T-2.md",
        "# T-2 — Legacy Task\n\n**Requirement**: FR-2\n",
    );

    let state = resolve_task_state(&path, &fs::read_to_string(&path)?, &pipeline())?;
    assert_eq!(state.location, TaskLocation::Done);
    assert_eq!(state.status, "pending");
    assert_eq!(state.attempts, 0);
    Ok(())
}
