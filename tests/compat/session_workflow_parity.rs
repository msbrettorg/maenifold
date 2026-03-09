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

#[test]
#[ignore = "T-10.19 backlog: Rust Workflow emits plain text under --json while C# returns the JSON envelope"]
fn workflow_json_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("workflow").expect("sandbox");
    let args = [
        "--tool",
        "Workflow",
        "--payload",
        r#"{"workflowId":"deductive-reasoning"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_eq!(normalize_workflow_text(&csharp.stdout), normalize_workflow_text(&rust.stdout));

    let csharp_session = parse_created_session_id(&csharp.stdout).expect("csharp workflow session id");
    let rust_session = parse_created_session_id(&rust.stdout).expect("rust workflow session id");
    let csharp_file = read_named_file(sandbox.root_dir(), &csharp_session).expect("csharp workflow file");
    let rust_file = read_named_file(sandbox.root_dir(), &rust_session).expect("rust workflow file");
    assert_eq!(normalize_workflow_markdown(&csharp_file), normalize_workflow_markdown(&rust_file));
}

#[test]
#[ignore = "T-10.19 backlog: Rust AssumptionLedger emits plain text under --json while C# returns the JSON envelope"]
fn assumption_ledger_append_json_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("assumption-ledger").expect("sandbox");
    let args = [
        "--tool",
        "AssumptionLedger",
        "--payload",
        r#"{"action":"append","assumption":"Check [[workflow-session]] parity","concepts":["workflow-session"],"confidence":"medium"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_eq!(normalize_assumption_text(&csharp.stdout), normalize_assumption_text(&rust.stdout));

    let csharp_uri = parse_memory_uri(&csharp.stdout).expect("csharp assumption uri");
    let rust_uri = parse_memory_uri(&rust.stdout).expect("rust assumption uri");
    let csharp_file = read_memory_uri_file(sandbox.root_dir(), &csharp_uri).expect("csharp assumption file");
    let rust_file = read_memory_uri_file(sandbox.root_dir(), &rust_uri).expect("rust assumption file");
    assert_eq!(normalize_assumption_markdown(&csharp_file), normalize_assumption_markdown(&rust_file));
}

fn parse_created_session_id(text: &str) -> Option<String> {
    text.lines()
        .next()
        .and_then(|line| line.strip_prefix("Created session: "))
        .map(str::trim)
        .map(str::to_string)
}

fn parse_memory_uri(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.split("memory://").nth(1).map(|rest| format!("memory://{}", rest.trim())))
}

fn read_memory_uri_file(root: &Path, uri: &str) -> Option<String> {
    let relative = uri.trim_start_matches("memory://");
    let path = root.join("memory").join(format!("{relative}.md"));
    fs::read_to_string(path).ok()
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

fn normalize_workflow_text(input: &str) -> String {
    let workflow = Regex::new(r"workflow-\d+").expect("workflow regex");
    workflow.replace_all(input, "workflow-<id>").trim().to_string()
}

fn normalize_workflow_markdown(input: &str) -> String {
    let workflow = Regex::new(r"workflow-\d+").expect("workflow regex");
    let iso = Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:\+00:00|Z)").expect("iso regex");
    let human = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC").expect("human regex");
    let output = workflow.replace_all(input, "workflow-<id>");
    let output = iso.replace_all(&output, "<iso>");
    let output = human.replace_all(&output, "<utc>");
    output.trim().to_string()
}

fn normalize_assumption_text(input: &str) -> String {
    let assumption = Regex::new(r"assumption-\d+").expect("assumption regex");
    assumption.replace_all(input, "assumption-<id>").trim().to_string()
}

fn normalize_assumption_markdown(input: &str) -> String {
    let assumption = Regex::new(r"assumption-\d+").expect("assumption regex");
    let iso = Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:\+00:00|Z)").expect("iso regex");
    let output = assumption.replace_all(input, "assumption-<id>");
    let output = iso.replace_all(&output, "<iso>");
    output.trim().to_string()
}
