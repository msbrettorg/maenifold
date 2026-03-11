#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;
use serde_json::Value;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_CSHARP_PROJECT: &str =
    "/Users/brett/src/ma-collective/maenifold/src/Maenifold.csproj";

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

fn write_file(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn parse_json_envelope(stdout: &str) -> Value {
    serde_json::from_str(stdout).expect("valid json envelope")
}

fn data_string<'a>(json: &'a Value, label: &str) -> &'a str {
    json.get("data")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{label} JSON envelope missing string data: {json}"))
}

fn extract_json_configuration(output: &str) -> Value {
    let marker = "JSON Configuration:";
    let marker_index = output.find(marker).expect("JSON Configuration marker");
    let suffix = output[marker_index + marker.len()..].trim();
    let json_start = suffix.find('{').expect("json object start");
    let candidate = &suffix[json_start..];

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in candidate.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return serde_json::from_str(&candidate[..=idx]).expect("valid embedded json");
                }
            }
            _ => {}
        }
    }

    panic!("unterminated embedded json: {candidate}");
}

#[derive(Debug, Default)]
struct UpdateAssetsSummary {
    added_files: Vec<String>,
    updated_files: Vec<String>,
    errors: Vec<String>,
}

fn parse_update_assets_summary(report: &str) -> UpdateAssetsSummary {
    let mut summary = UpdateAssetsSummary::default();
    let mut section = "";

    for raw_line in report.lines() {
        let line = raw_line.trim();
        if let Some(next_section) = line.strip_prefix("### ") {
            section = next_section;
            continue;
        }

        let Some(item) = line.strip_prefix("- ") else {
            continue;
        };
        if item == "None" {
            continue;
        }

        match section {
            "Added Files" => summary.added_files.push(item.to_string()),
            "Updated Files" => summary.updated_files.push(item.to_string()),
            "Errors" => summary.errors.push(item.to_string()),
            _ => {}
        }
    }

    summary
}

fn assert_success_envelope(json: &Value, label: &str) {
    assert_eq!(json["success"], true, "{label} expected success envelope: {json}");
    assert!(
        json.get("error").is_some(),
        "{label} envelope missing error field: {json}"
    );
}

// T-10.5: RTM FR-13 / RTM-074 — full oracle parity for [[Adopt]] JSON envelope + instruction text.
// T-10.5: RTM FR-13 — keep C#-oracle-dependent parity tests honestly reported as ignored.
#[ignore = "requires C# oracle"]
#[test]
fn adopt_json_matches_csharp_oracle() {

    let sandbox = IsolatedRoot::new("adopt-parity").expect("sandbox");
    write_file(
        &sandbox.root_dir().join("assets/roles/test-role.json"),
        r#"{
  "id": "test-role",
  "name": "Test Role",
  "personality": "analytical",
  "principles": ["accuracy", "traceability"]
}"#,
    )
    .expect("write adopt fixture");

    let args = [
        "--tool",
        "Adopt",
        "--payload",
        r#"{"type":"role","identifier":"test-role"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp adopt");
    assert!(csharp.success(), "C# Adopt failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust adopt");
    assert!(rust.success(), "Rust Adopt failed: {}", rust.stderr);

    let csharp_json = parse_json_envelope(&csharp.stdout);
    let rust_json = parse_json_envelope(&rust.stdout);
    assert_success_envelope(&csharp_json, "C# Adopt");
    assert_success_envelope(&rust_json, "Rust Adopt");

    let csharp_instruction = data_string(&csharp_json, "C# Adopt").trim_end();
    // T-15.15: RTM FR-1 — Rust now returns a plain string in data, matching C#.
    let rust_instruction = data_string(&rust_json, "Rust Adopt").trim_end();
    assert!(
        rust_instruction.contains("You are now adopting the role: test-role"),
        "expected adoption instruction in output: {rust_instruction}"
    );
    assert_eq!(
        csharp_instruction,
        rust_instruction,
        "Adopt instruction drifted from C# oracle"
    );
    assert_eq!(
        extract_json_configuration(csharp_instruction),
        extract_json_configuration(rust_instruction),
        "Adopt embedded JSON drifted from C# oracle"
    );
}

// T-10.5: RTM FR-13 / RTM-076 — full oracle parity for [[UpdateAssets]] dry-run JSON summary.
// T-10.5: RTM FR-13 — keep C#-oracle-dependent parity tests honestly reported as ignored.
#[ignore = "requires C# oracle"]
#[test]
fn update_assets_dry_run_json_matches_csharp_oracle() {

    let csharp_sandbox = IsolatedRoot::new("update-assets-parity-csharp").expect("csharp sandbox");
    let rust_sandbox = IsolatedRoot::new("update-assets-parity-rust").expect("rust sandbox");
    let warm_args = ["--tool", "UpdateAssets", "--payload", r#"{"dryRun":true}"#, "--json"];

    let csharp_warm = run_csharp_cli(&warm_args, &csharp_sandbox).expect("warm csharp update-assets");
    assert!(
        csharp_warm.success(),
        "C# warm UpdateAssets failed: {}",
        csharp_warm.stderr
    );
    let rust_warm = run_rust_cli(&warm_args, &rust_sandbox).expect("warm rust update-assets");
    assert!(
        rust_warm.success(),
        "Rust warm UpdateAssets failed: {}",
        rust_warm.stderr
    );

    // T-10.5: RTM FR-13 / RTM-076 — mutate the initialized runtime copy so both runtimes
    // must detect the same packaged-vs-runtime drift for [[UpdateAssets]].
    let relative = "colors/blue.json";
    let mutated = r#"{"id":"blue","name":"Blue","mutated":true}"#;
    write_file(&csharp_sandbox.root_dir().join("assets").join(relative), mutated)
        .expect("mutate csharp runtime asset");
    write_file(&rust_sandbox.root_dir().join("assets").join(relative), mutated)
        .expect("mutate rust runtime asset");

    let csharp = run_csharp_cli(&warm_args, &csharp_sandbox).expect("csharp update-assets");
    assert!(csharp.success(), "C# UpdateAssets failed: {}", csharp.stderr);

    let rust = run_rust_cli(&warm_args, &rust_sandbox).expect("rust update-assets");
    assert!(rust.success(), "Rust UpdateAssets failed: {}", rust.stderr);

    let csharp_json = parse_json_envelope(&csharp.stdout);
    let rust_json = parse_json_envelope(&rust.stdout);
    assert_success_envelope(&csharp_json, "C# UpdateAssets");
    assert_success_envelope(&rust_json, "Rust UpdateAssets");
    let csharp_summary = data_string(&csharp_json, "C# UpdateAssets");
    let csharp_sections = parse_update_assets_summary(csharp_summary);
    let rust_added_files = rust_json["data"]["addedFiles"]
        .as_array()
        .expect("rust addedFiles array")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let rust_updated_files = rust_json["data"]["updatedFiles"]
        .as_array()
        .expect("rust updatedFiles array")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let rust_errors = rust_json["data"]["errors"]
        .as_array()
        .expect("rust errors array")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(rust_json["data"]["dryRun"], true, "unexpected output: {rust_json}");
    let summary = rust_json["data"]["summary"]
        .as_str()
        .expect("dry-run summary");
    // PARITY-U01: "This was a DRY RUN" message removed for C# parity.
    assert!(
        !summary.contains("This was a DRY RUN"),
        "DRY RUN trailing message should not appear (C# parity): {summary}"
    );
    assert!(
        summary.contains(relative),
        "expected mutated runtime asset in Rust summary: {summary}"
    );
    assert!(
        csharp_summary.contains("# Asset Update Summary"),
        "expected C# update summary heading: {csharp_summary}"
    );
    assert_eq!(
        csharp_sections.added_files,
        rust_added_files,
        "UpdateAssets added file set drifted from C# oracle"
    );
    assert_eq!(
        csharp_sections.updated_files,
        rust_updated_files,
        "UpdateAssets updated file set drifted from C# oracle"
    );
    assert_eq!(
        csharp_sections.errors,
        rust_errors,
        "UpdateAssets error set drifted from C# oracle"
    );
}
