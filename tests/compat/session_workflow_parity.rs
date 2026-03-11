#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;
use json_semantic::assert_semantically_equal_ignoring_fields;
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn sequential_thinking_json_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("sequential-thinking").expect("sandbox");
    let args = [
        "--tool",
        "SequentialThinking",
        "--payload",
        r#"{"response":"Tracing [[session-paths]] in [[maenifold-rs]].","thoughtNumber":0,"totalThoughts":2,"nextThoughtNeeded":true}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_semantically_equal_ignoring_fields(&csharp.stdout, &rust.stdout, &["sessionId", "message"]);

    let csharp_json: Value = serde_json::from_str(&csharp.stdout).expect("csharp json");
    let rust_json: Value = serde_json::from_str(&rust.stdout).expect("rust json");
    let csharp_message = csharp_json["data"]["message"].as_str().unwrap_or_default();
    let rust_message = rust_json["data"]["message"].as_str().unwrap_or_default();
    for marker in ["Continue with thought 1/2", "CHECK YOUR MEMORY"] {
        assert!(csharp_message.contains(marker), "missing marker in C# output: {marker}");
        assert!(rust_message.contains(marker), "missing marker in Rust output: {marker}");
    }

    let csharp_session = csharp_json["data"]["sessionId"].as_str().expect("csharp sessionId");
    let rust_session = rust_json["data"]["sessionId"].as_str().expect("rust sessionId");
    let csharp_file = read_named_file(sandbox.root_dir(), csharp_session).expect("csharp session file");
    let rust_file = read_named_file(sandbox.root_dir(), rust_session).expect("rust session file");
    assert_eq!(normalize_sequential_markdown(&csharp_file), normalize_sequential_markdown(&rust_file));
}

#[test]
fn sequential_thinking_requires_zero_for_first_thought_like_csharp() {
    let sandbox = IsolatedRoot::new("sequential-thinking-first-thought").expect("sandbox");
    let args = [
        "--tool",
        "SequentialThinking",
        "--payload",
        r#"{"response":"Testing [[parity]]","thoughtNumber":1,"totalThoughts":3,"nextThoughtNeeded":true}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    // T-10.19: RTM FR-13 / NFR-1 — both CLIs reject starting with thoughtNumber=1.
    assert_eq!(csharp.exit_code, rust.exit_code, "exit codes differ");

    assert_semantically_equal_ignoring_fields(&csharp.stdout, &rust.stdout, &["message"]);
}


fn read_named_file(root: &Path, file_stem: &str) -> Option<String> {
    find_named_markdown(root.join("memory"), file_stem).and_then(|path| fs::read_to_string(path).ok())
}

fn find_named_markdown(root: PathBuf, file_stem: &str) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_named_markdown(path, file_stem) {
                return Some(found);
            }
            continue;
        }
        let matches = path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(|value| value == file_stem)
            .unwrap_or(false);
        if matches {
            return Some(path);
        }
    }
    None
}

fn normalize_sequential_markdown(input: &str) -> String {
    let session = Regex::new(r"session-\d+-\d{5}").expect("session regex");
    let iso = Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:\+00:00|Z)").expect("iso regex");
    let human = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC").expect("human regex");
    let output = session.replace_all(input, "session-<id>");
    let output = iso.replace_all(&output, "<iso>");
    let output = human.replace_all(&output, "<utc>");
    output.trim().to_string()
}

