#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;
use json_semantic::assert_semantically_equal_ignoring_fields;

#[test]
fn search_graph_parity_harness_smoke() {
    let sandbox = IsolatedRoot::new("search-graph-smoke").expect("sandbox");
    assert!(sandbox.root_dir().join("memory").exists());
    assert!(sandbox.root_dir().join("assets").exists());
}

#[test]
fn search_memories_json_matches_csharp_oracle_after_csharp_seed() {
    let sandbox = IsolatedRoot::new("search-parity").expect("sandbox");

    let write_args = [
        "--tool",
        "WriteMemory",
        "--payload",
        r#"{"title":"Search Parity Seed","content":"Shared [[search-index]] testing for [[machine-learning]]."}"#,
        "--json",
    ];
    let csharp_write = run_csharp_cli(&write_args, &sandbox).expect("csharp write");
    assert!(
        csharp_write.success(),
        "C# write failed: {}",
        csharp_write.stderr
    );

    let sync_args = ["--tool", "Sync", "--payload", "{}", "--json"];
    let csharp_sync = run_csharp_cli(&sync_args, &sandbox).expect("csharp sync");
    assert!(
        csharp_sync.success(),
        "C# sync failed: {}",
        csharp_sync.stderr
    );

    let search_args = [
        "--tool",
        "SearchMemories",
        "--payload",
        r#"{"query":"machine learning","mode":"Hybrid","pageSize":5,"page":1}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&search_args, &sandbox).expect("csharp search");
    assert!(csharp.success(), "C# search failed: {}", csharp.stderr);

    let rust = run_rust_cli(&search_args, &sandbox).expect("rust search");
    assert!(rust.success(), "Rust search failed: {}", rust.stderr);

    assert_semantically_equal_ignoring_fields(
        &csharp.stdout,
        &rust.stdout,
        // T-10.19: RTM FR-13 / NFR-1 — search parity ignores volatile scoring/preview
        // fields. T-16.4 removed decayWeight from output to match C#.
        &["created", "modified", "fusedScore", "snippet"],
    );
}

#[test]
fn build_context_missing_concept_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("build-context-missing").expect("sandbox");
    let args = [
        "--tool",
        "BuildContext",
        "--payload",
        r#"{"conceptName":"nonexistentconceptxyz123"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp context");
    assert!(csharp.success(), "C# context failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust context");
    assert!(rust.success(), "Rust context failed: {}", rust.stderr);

    assert_eq!(csharp.stdout, rust.stdout, "BuildContext missing-concept output drifted from C# oracle");
}
