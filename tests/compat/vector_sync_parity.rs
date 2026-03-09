#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;

#[test]
fn find_similar_concepts_json_matches_csharp_oracle_after_sync() {
    let sandbox = IsolatedRoot::new("find-similar-parity").expect("sandbox");

    for payload in [
        r#"{"title":"Similarity Seed A","content":"Links [[authentication]] and [[oauth2]]."}"#,
        r#"{"title":"Similarity Seed B","content":"Links [[authentication]] and [[jwt]]."}"#,
    ] {
        let write_args = ["--tool", "WriteMemory", "--payload", payload, "--json"];
        let csharp_write = run_csharp_cli(&write_args, &sandbox).expect("csharp write");
        assert!(csharp_write.success(), "C# write failed: {}", csharp_write.stderr);
    }

    let sync_args = ["--tool", "Sync", "--payload", "{}", "--json"];
    let csharp_sync = run_csharp_cli(&sync_args, &sandbox).expect("csharp sync");
    assert!(csharp_sync.success(), "C# sync failed: {}", csharp_sync.stderr);

    let args = [
        "--tool",
        "FindSimilarConcepts",
        "--payload",
        r#"{"conceptName":"authentication","maxResults":5}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_eq!(csharp.stdout, rust.stdout, "FindSimilarConcepts output drifted from C# oracle");
}

#[test]
fn watcher_lifecycle_text_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("watcher-parity").expect("sandbox");

    let start_args = ["--tool", "StartWatcher", "--payload", r#"{"debounceMs":250}"#];
    let csharp_start = run_csharp_cli(&start_args, &sandbox).expect("csharp start");
    assert!(csharp_start.success(), "C# start failed: {}", csharp_start.stderr);

    let rust_start = run_rust_cli(&start_args, &sandbox).expect("rust start");
    assert!(rust_start.success(), "Rust start failed: {}", rust_start.stderr);
    assert_eq!(csharp_start.stdout, rust_start.stdout, "StartWatcher output drifted from C# oracle");

    let stop_args = ["--tool", "StopWatcher", "--payload", "{}"];
    let csharp_stop = run_csharp_cli(&stop_args, &sandbox).expect("csharp stop");
    assert!(csharp_stop.success(), "C# stop failed: {}", csharp_stop.stderr);

    let rust_stop = run_rust_cli(&stop_args, &sandbox).expect("rust stop");
    assert!(rust_stop.success(), "Rust stop failed: {}", rust_stop.stderr);
    assert_eq!(csharp_stop.stdout, rust_stop.stdout, "StopWatcher output drifted from C# oracle");
}

#[test]
fn sync_json_matches_csharp_oracle_for_empty_root_text_contract() {
    let sandbox = IsolatedRoot::new("leader-sync-parity").expect("sandbox");
    let args = ["--tool", "Sync", "--payload", "{}", "--json"];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_eq!(csharp.stdout, rust.stdout, "Sync output drifted from C# oracle");
}

#[test]
fn extract_concepts_after_sync_matches_csharp_oracle_for_code_block_exclusion() {
    let sandbox = IsolatedRoot::new("markdown-reader-sync-parity").expect("sandbox");
    let write_args = [
        "--tool",
        "WriteMemory",
        "--payload",
        r#"{"title":"Markdown Reader Parity Seed","content":"Visible [[authentication]].\n\n```rust\n[[ignored-in-code]]\n```\n\nInline `[[ignored-inline]]` should be ignored."}"#,
        "--json",
    ];
    let csharp_write = run_csharp_cli(&write_args, &sandbox).expect("csharp write");
    assert!(csharp_write.success(), "C# write failed: {}", csharp_write.stderr);

    let sync_args = ["--tool", "Sync", "--payload", "{}", "--json"];
    let csharp_sync = run_csharp_cli(&sync_args, &sandbox).expect("csharp sync");
    assert!(csharp_sync.success(), "C# sync failed: {}", csharp_sync.stderr);

    let args = [
        "--tool",
        "ExtractConceptsFromFile",
        "--payload",
        r#"{"identifier":"markdown-reader-parity-seed"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp extract");
    assert!(csharp.success(), "C# extract failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust extract");
    assert!(rust.success(), "Rust extract failed: {}", rust.stderr);

    assert_eq!(csharp.stdout, rust.stdout, "ExtractConceptsFromFile output drifted from C# oracle");
}
