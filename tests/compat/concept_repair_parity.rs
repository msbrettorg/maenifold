#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;
use json_semantic::assert_semantically_equal;
use serde_json::Value;

const DEFAULT_CSHARP_PROJECT: &str =
    "/Users/brett/src/ma-collective/maenifold/src/Maenifold.csproj";

fn write_markdown_file(sandbox: &IsolatedRoot, relative_path: &str, content: &str) {
    let path = sandbox.root_dir().join("memory").join(relative_path);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create memory directories");
    fs::write(path, content).expect("write markdown file");
}

fn require_csharp_oracle(test_name: &str) -> bool {
    match csharp_oracle_unavailable_reason() {
        Some(reason) => {
            eprintln!("skipping {test_name}: {reason}");
            false
        }
        None => true,
    }
}

fn csharp_oracle_unavailable_reason() -> Option<String> {
    let project = env::var_os("MAENIFOLD_CSHARP_PROJECT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CSHARP_PROJECT));
    if !project.is_file() {
        return Some(format!("C# oracle project not found at {}", project.display()));
    }

    match Command::new("dotnet").arg("--version").output() {
        Ok(output) if output.status.success() => None,
        Ok(output) => Some(format!(
            "dotnet --version failed with status {}",
            output.status
        )),
        Err(error) => Some(format!("dotnet is unavailable: {error}")),
    }
}

fn sync_with_csharp(sandbox: &IsolatedRoot) {
    let sync_args = ["--tool", "Sync", "--payload", "{}", "--json"];
    let sync = run_csharp_cli(&sync_args, sandbox).expect("csharp sync");
    assert!(sync.success(), "C# sync failed: {}", sync.stderr);
}

fn json_data(output: &str) -> String {
    serde_json::from_str::<Value>(output)
        .expect("json output")
        .get("data")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| output.to_string())
}

// T-1.6: RTM FR-2, NFR-1; RTM-075 — [[AnalyzeConceptCorruption]] parity test skips cleanly without the C# oracle.
#[test]
fn analyze_concept_corruption_json_matches_csharp_oracle_after_sync() {
    if !require_csharp_oracle("analyze_concept_corruption_json_matches_csharp_oracle_after_sync") {
        return;
    }

    let sandbox = IsolatedRoot::new("analyze-concept-corruption").expect("sandbox");
    write_markdown_file(
        &sandbox,
        "concepts/agents.md",
        r#"---
title: Agents
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Agents
Using [[agent]], [[agents]], [[agent-system]], and [[agent services]].
"#,
    );
    write_markdown_file(
        &sandbox,
        "concepts/agent-orchestrator.md",
        r#"---
title: Agent Orchestrator
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Agent Orchestrator
This note references [[agent]], [[agents]], and [[agent-system]] again.
"#,
    );
    write_markdown_file(
        &sandbox,
        "concepts/agent-filepath.md",
        r#"---
title: Agent File Path
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Agent File Path
The loader also sees [[AgentRunner.cs]] when discussing the [[agent]] family.
"#,
    );

    sync_with_csharp(&sandbox);

    let args = [
        "--tool",
        "AnalyzeConceptCorruption",
        "--payload",
        r#"{"conceptFamily":"agent"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp analyze");
    assert!(csharp.success(), "C# analyze failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust analyze");
    assert!(rust.success(), "Rust analyze failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);

    let rendered = json_data(&rust.stdout);
    assert!(
        rendered.contains("Singular/Plural:") && rendered.contains("agents"),
        "expected plural classification in output: {rendered}"
    );
    assert!(
        rendered.contains("Compound (with -):") && rendered.contains("agent-system"),
        "expected compound classification in output: {rendered}"
    );
    assert!(
        rendered.contains("File Paths:") && rendered.contains("AgentRunner.cs"),
        "expected file-path classification in output: {rendered}"
    );
}

// T-1.6: RTM NFR-1; RTM-079 — related parity test also skips cleanly without the C# oracle.
#[test]
fn repair_concepts_dry_run_json_matches_csharp_oracle_without_modifying_files() {
    if !require_csharp_oracle("repair_concepts_dry_run_json_matches_csharp_oracle_without_modifying_files")
    {
        return;
    }

    let sandbox = IsolatedRoot::new("repair-concepts-dry-run").expect("sandbox");
    let relative_path = "repair/agents.md";
    let original = r#"---
title: Repair Agents
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Repair Agents
Using [[agents]] for a dry-run parity check.
"#;
    write_markdown_file(&sandbox, relative_path, original);

    sync_with_csharp(&sandbox);

    let args = [
        "--tool",
        "RepairConcepts",
        "--payload",
        r#"{"conceptsToReplace":"agents","canonicalConcept":"agent","dryRun":true}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp repair");
    assert!(csharp.success(), "C# repair failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust repair");
    assert!(rust.success(), "Rust repair failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);

    let rendered = json_data(&rust.stdout);
    assert!(
        rendered.contains("Would modify: repair/agents"),
        "expected dry-run modification summary: {rendered}"
    );
    assert!(
        rendered.contains("This was a DRY RUN"),
        "expected dry-run marker: {rendered}"
    );

    let after = fs::read_to_string(sandbox.root_dir().join("memory").join(relative_path))
        .expect("read repaired file");
    assert_eq!(after, original, "dry-run must not modify the source file");
}
