use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn repo_root() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate should live under crates/maenifold-cli")
        .to_path_buf()
}

#[test]
fn audit_parity_script_exists_and_is_executable() {
    let script = repo_root().join("scripts/audit-parity.sh");
    let metadata = fs::metadata(&script).expect("scripts/audit-parity.sh should exist");
    assert!(metadata.is_file(), "audit-parity.sh should be a file");

    #[cfg(unix)]
    assert_ne!(
        metadata.permissions().mode() & 0o111,
        0,
        "audit-parity.sh should be executable"
    );
}

#[test]
fn parity_uat_doc_contains_required_sections() {
    let doc_path = repo_root().join("docs/parity-uat.md");
    let doc = fs::read_to_string(&doc_path).expect("docs/parity-uat.md should exist");

    for heading in [
        "# CLI / MCP Parity UAT — Recurring Procedure",
        "## Purpose & cadence",
        "## Prerequisites",
        "## Procedure A: Surface enumeration",
        "## Procedure B: Per-tool round-trip",
        "## Procedure C: Downstream proxy + discovery",
        "## Procedure D: Schema drift",
        "## Procedure E: Skill resync",
        "## When to escalate",
        "## Footer",
    ] {
        assert!(doc.contains(heading), "missing heading: {heading}");
    }
}
