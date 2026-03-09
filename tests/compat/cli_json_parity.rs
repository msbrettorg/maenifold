#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;
use json_semantic::{assert_semantically_equal, assert_semantically_equal_ignoring_fields};

#[test]
fn isolated_root_sets_expected_locations() {
    let sandbox = IsolatedRoot::new("compat-root").expect("sandbox");

    assert!(sandbox.root_dir().exists());
    assert!(sandbox.root_dir().join("memory").exists());
    assert!(sandbox.root_dir().join("assets").exists());
    assert_eq!(
        sandbox.database_path(),
        sandbox.root_dir().join("memory.db")
    );
}

#[test]
fn semantic_json_compare_ignores_object_key_order() {
    assert_semantically_equal(
        r#"{"success":true,"data":{"b":2,"a":1}}"#,
        r#"{"data":{"a":1,"b":2},"success":true}"#,
    );
}

#[test]
fn list_assets_json_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("list-assets").expect("sandbox");
    let args = ["--tool", "ListAssets", "--payload", "{}", "--json"];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

#[test]
fn read_catalog_json_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("read-catalog").expect("sandbox");
    let args = [
        "--tool",
        "ReadMcpResource",
        "--payload",
        r#"{"uri":"asset://catalog"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}


#[test]
fn write_memory_json_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("write-memory").expect("sandbox");
    let payload = r#"{"title":"Compat Test","content":"Testing [[compatibility]] from Rust."}"#;
    let args = ["--tool", "WriteMemory", "--payload", payload, "--json"];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_semantically_equal_ignoring_fields(
        &csharp.stdout,
        &rust.stdout,
        &["checksum"],
    );
}

#[test]
fn write_memory_validation_error_matches_csharp_oracle_without_wikilinks() {
    let sandbox = IsolatedRoot::new("write-memory-validation-error").expect("sandbox");
    let payload = r#"{"title":"No Link","content":"plain text only"}"#;
    let args = ["--tool", "WriteMemory", "--payload", payload, "--json"];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

#[test]
fn read_memory_json_matches_csharp_oracle_after_csharp_write() {
    let sandbox = IsolatedRoot::new("read-memory").expect("sandbox");
    let write_payload =
        r#"{"title":"Read Compat","content":"Reading [[compatibility]] through shared storage."}"#;
    let write_args = [
        "--tool",
        "WriteMemory",
        "--payload",
        write_payload,
        "--json",
    ];

    let csharp_write = run_csharp_cli(&write_args, &sandbox).expect("csharp write");
    assert!(
        csharp_write.success(),
        "C# write failed: {}",
        csharp_write.stderr
    );

    let read_args = [
        "--tool",
        "ReadMemory",
        "--payload",
        r#"{"identifier":"memory://read-compat"}"#,
        "--json",
    ];

    let csharp_read = run_csharp_cli(&read_args, &sandbox).expect("csharp read");
    assert!(
        csharp_read.success(),
        "C# read failed: {}",
        csharp_read.stderr
    );

    let rust_read = run_rust_cli(&read_args, &sandbox).expect("rust read");
    assert!(
        rust_read.success(),
        "Rust read failed: {}",
        rust_read.stderr
    );

    assert_semantically_equal_ignoring_fields(
        &csharp_read.stdout,
        &rust_read.stdout,
        &["created", "modified"],
    );
}

#[test]
fn read_memory_json_omits_checksum_when_requested() {
    let sandbox = IsolatedRoot::new("read-memory-no-checksum").expect("sandbox");
    let write_args = [
        "--tool",
        "WriteMemory",
        "--payload",
        r#"{"title":"Checksumless Read","content":"Testing [[checksumless]]."}"#,
        "--json",
    ];

    let csharp_write = run_csharp_cli(&write_args, &sandbox).expect("csharp write");
    assert!(
        csharp_write.success(),
        "C# write failed: {}",
        csharp_write.stderr
    );

    let read_args = [
        "--tool",
        "ReadMemory",
        "--payload",
        r#"{"identifier":"memory://checksumless-read","includeChecksum":false}"#,
        "--json",
    ];

    let csharp_read = run_csharp_cli(&read_args, &sandbox).expect("csharp read");
    assert!(
        csharp_read.success(),
        "C# read failed: {}",
        csharp_read.stderr
    );

    let rust_read = run_rust_cli(&read_args, &sandbox).expect("rust read");
    assert!(
        rust_read.success(),
        "Rust read failed: {}",
        rust_read.stderr
    );

    assert_semantically_equal(&csharp_read.stdout, &rust_read.stdout);
}

#[test]
fn read_memory_not_found_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("read-memory-not-found").expect("sandbox");
    let args = [
        "--tool",
        "ReadMemory",
        "--payload",
        r#"{"identifier":"memory://does-not-exist"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

#[test]
fn list_memories_markdown_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("list-memories").expect("sandbox");
    let write_args = [
        "--tool",
        "WriteMemory",
        "--payload",
        r#"{"title":"List Seed","content":"Testing [[list-memories]]."}"#,
    ];

    let csharp_write = run_csharp_cli(&write_args, &sandbox).expect("csharp write");
    assert!(csharp_write.success(), "C# write failed: {}", csharp_write.stderr);

    let args = ["--tool", "ListMemories", "--payload", "{}"];
    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_eq!(csharp.stdout, rust.stdout, "ListMemories output drifted from C# oracle");
}

#[test]
fn get_help_valid_tool_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("get-help-valid").expect("sandbox");
    let args = [
        "--tool",
        "GetHelp",
        "--payload",
        r#"{"toolName":"writememory"}"#,
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_eq!(csharp.stdout, rust.stdout, "GetHelp(valid) output drifted from C# oracle");
}

#[test]
fn get_help_invalid_tool_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("get-help-invalid").expect("sandbox");
    let args = [
        "--tool",
        "GetHelp",
        "--payload",
        r#"{"toolName":"non-existent-tool-xyz"}"#,
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI failed: {}", rust.stderr);

    assert_eq!(csharp.stdout, rust.stdout, "GetHelp(invalid) output drifted from C# oracle");
}

#[test]
fn write_memory_path_security_error_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("write-memory-path-security").expect("sandbox");
    let payload = r#"{"title":"Blocked","content":"Has [[concept]]","folder":"../escape"}"#;
    let args = ["--tool", "WriteMemory", "--payload", payload, "--json"];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# CLI process failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust CLI process failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}
