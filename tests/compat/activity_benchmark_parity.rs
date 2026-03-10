#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_CSHARP_PROJECT: &str =
    "/Users/brett/src/ma-collective/maenifold/src/Maenifold.csproj";

const BENCHMARK_SECTION_HEADERS: [&str; 5] = [
    "GRPH-009: CTE vs N+1 Pattern Benchmark",
    "Search Performance Benchmark",
    "Sync Performance Benchmark",
    "Complex Traversal Bottleneck Analysis",
    "System Health Report",
];

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

fn write_memory_file(sandbox: &IsolatedRoot, relative: &str, content: &str) {
    let path = sandbox.root_dir().join("memory").join(relative);
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture directories");
    fs::write(path, content).expect("write fixture markdown");
}

fn recent_activity_fixture(sandbox: &IsolatedRoot) {
    // T-10.6: RTM FR-13 / RTM-077 — seed mixed [[RecentActivity]] fixture files inside [[IsolatedRoot]].
    write_memory_file(
        sandbox,
        "thinking/sequential/2026/03/session-recent.md",
        r#"---
title: Session Recent
created: 2026-03-08T00:00:00Z
modified: 2026-03-08T00:00:00Z
---
# Session Recent

## Thought 1
Working through [[RecentActivity]] parity.

## Thought 2
Comparing [[maenifold-rs]] against [[c#-oracle]].
"#,
    );
    write_memory_file(
        sandbox,
        "thinking/workflow/2026/03/workflow-middle.md",
        r#"---
title: Workflow Middle
created: 2026-03-08T00:00:00Z
modified: 2026-03-08T00:00:00Z
---
# Workflow Middle

## Step 1
Collect evidence for [[RunFullBenchmark]].

## Step 2
Summarize findings for [[RecentActivity]].
"#,
    );
    write_memory_file(
        sandbox,
        "projects/knowledge-note.md",
        r#"---
title: Knowledge Note
created: 2026-03-08T00:00:00Z
modified: 2026-03-08T00:00:00Z
---
# Knowledge Note

## Intro
Stored [[memory]] content for parity coverage.

## Outro
Closing section for [[system-tools]] verification.
"#,
    );
}

fn benchmark_fixture(sandbox: &IsolatedRoot) {
    // T-10.6: RTM FR-13 / RTM-078 — seed enough markdown files for [[RunFullBenchmark]] to render all sections.
    for index in 0..110 {
        write_memory_file(
            sandbox,
            &format!("benchmark/seed-{index:03}.md"),
            &format!(
                r#"---
title: Benchmark Seed {index}
created: 2026-03-08T00:00:00Z
modified: 2026-03-08T00:00:00Z
---
# Benchmark Seed {index}

## Evidence
Testing [[benchmark-hub]], [[search-performance]], [[graph-traversal]], and [[seed-{index}]].

## Summary
Shared corpus for [[RunFullBenchmark]] structure parity.
"#
            ),
        );
    }
}

fn run_sync_with_csharp(sandbox: &IsolatedRoot) {
    // T-10.6: RTM FR-13 — use the [[c#-oracle]] to populate shared SQLite state before parity assertions.
    let sync_args = ["--tool", "Sync", "--payload", "{}", "--json"];
    let csharp_sync = run_csharp_cli(&sync_args, sandbox).expect("csharp sync");
    assert!(
        csharp_sync.success(),
        "C# sync failed: {}",
        csharp_sync.stderr
    );
}

fn parse_json_envelope(stdout: &str) -> Value {
    serde_json::from_str(stdout).expect("valid json envelope")
}

fn data_string<'a>(json: &'a Value, label: &str) -> &'a str {
    json.get("data")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{label} JSON envelope missing string data: {json}"))
}

fn assert_success_envelope(json: &Value, label: &str) {
    assert_eq!(json["success"], true, "{label} expected success envelope: {json}");
    assert!(
        json.get("error").is_some(),
        "{label} envelope missing error field: {json}"
    );
}

fn sandbox_has_indexed_content(sandbox: &IsolatedRoot) {
    assert!(
        Path::new(sandbox.database_path()).exists(),
        "expected sync to create database at {}",
        sandbox.database_path().display()
    );
}

fn recent_activity_items(report: &str) -> Vec<String> {
    let mut items: Vec<_> = report
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let remainder = trimmed.strip_prefix("**")?;
            let (name, tail) = remainder.split_once("**")?;
            Some(format!("{name}{}", tail.trim()))
        })
        .collect();
    items.sort();
    items
}

fn present_benchmark_sections(report: &str) -> Vec<&'static str> {
    BENCHMARK_SECTION_HEADERS
        .into_iter()
        .filter(|heading| report.contains(heading))
        .collect()
}

// T-10.6: RTM FR-13 / RTM-077 — parity harness for [[RecentActivity]] JSON envelopes and item coverage.
#[test]
fn recent_activity_json_has_oracle_aligned_activity_items() {
    if !require_csharp_oracle("recent_activity_json_has_oracle_aligned_activity_items") {
        return;
    }

    let sandbox = IsolatedRoot::new("recent-activity-parity").expect("sandbox");
    recent_activity_fixture(&sandbox);
    run_sync_with_csharp(&sandbox);
    sandbox_has_indexed_content(&sandbox);

    let args = [
        "--tool",
        "RecentActivity",
        "--payload",
        r#"{"limit":5,"filter":"all"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp recent activity");
    assert!(
        csharp.success(),
        "C# RecentActivity failed: {}",
        csharp.stderr
    );
    let rust = run_rust_cli(&args, &sandbox).expect("rust recent activity");
    assert!(rust.success(), "Rust RecentActivity failed: {}", rust.stderr);

    let csharp_json = parse_json_envelope(&csharp.stdout);
    let rust_json = parse_json_envelope(&rust.stdout);
    assert_success_envelope(&csharp_json, "C# RecentActivity");
    assert_success_envelope(&rust_json, "Rust RecentActivity");

    let csharp_report = data_string(&csharp_json, "C# RecentActivity");
    let rust_report = data_string(&rust_json, "Rust RecentActivity");
    assert!(
        csharp_report.starts_with("# Recent Activity"),
        "expected recent activity heading: {csharp_report}"
    );
    assert!(
        rust_report.starts_with("# Recent Activity"),
        "expected recent activity heading: {rust_report}"
    );
    assert_eq!(
        recent_activity_items(csharp_report),
        recent_activity_items(rust_report),
        "RecentActivity item coverage drifted between C# and Rust"
    );
}

// T-10.6: RTM FR-13 / RTM-078 — structure-only parity for [[RunFullBenchmark]] JSON envelope category headings.
#[test]
fn run_full_benchmark_json_has_expected_structure() {
    if !require_csharp_oracle("run_full_benchmark_json_has_expected_structure") {
        return;
    }

    let sandbox = IsolatedRoot::new("run-full-benchmark-parity").expect("sandbox");
    benchmark_fixture(&sandbox);
    run_sync_with_csharp(&sandbox);
    sandbox_has_indexed_content(&sandbox);

    let args = [
        "--tool",
        "RunFullBenchmark",
        "--payload",
        r#"{"iterations":1,"includeDeepTraversal":false}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp benchmark");
    assert!(
        csharp.success(),
        "C# RunFullBenchmark failed: {}",
        csharp.stderr
    );
    let rust = run_rust_cli(&args, &sandbox).expect("rust benchmark");
    assert!(
        rust.success(),
        "Rust RunFullBenchmark failed: {}",
        rust.stderr
    );

    let csharp_json = parse_json_envelope(&csharp.stdout);
    let rust_json = parse_json_envelope(&rust.stdout);
    assert_success_envelope(&csharp_json, "C# RunFullBenchmark");
    assert_success_envelope(&rust_json, "Rust RunFullBenchmark");

    let csharp_report = data_string(&csharp_json, "C# RunFullBenchmark");
    let rust_report = data_string(&rust_json, "Rust RunFullBenchmark");
    assert!(
        csharp_report.contains("MA CORE PERFORMANCE BENCHMARK SUITE"),
        "expected benchmark suite heading in C# output: {csharp_report}"
    );
    assert!(
        rust_report.contains("MA CORE PERFORMANCE BENCHMARK SUITE"),
        "expected benchmark suite heading in Rust output: {rust_report}"
    );

    let rust_sections = present_benchmark_sections(rust_report);
    assert_eq!(
        rust_sections,
        BENCHMARK_SECTION_HEADERS,
        "Rust benchmark structure drifted: {rust_report}"
    );
    assert!(
        rust_report.contains("Skipped: includeDeepTraversal=false"),
        "expected Rust structure-only skip marker: {rust_report}"
    );

    let csharp_failed = csharp_report.contains("BENCHMARK FAILED:");
    if csharp_failed {
        assert!(
            csharp_report.contains("BENCHMARK FAILED:"),
            "expected explicit benchmark failure marker: {csharp_report}"
        );
    } else {
        assert_eq!(
            present_benchmark_sections(csharp_report),
            BENCHMARK_SECTION_HEADERS,
            "C# benchmark structure drifted: {csharp_report}"
        );
        assert!(
            csharp_report.contains("Skipped: includeDeepTraversal=false"),
            "expected C# structure-only skip marker: {csharp_report}"
        );
    }
}
