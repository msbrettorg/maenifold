// T-1.3: RTM FR-13 — Adopt + Workflow oracle parity tests against C# reference implementation.
// All tests are marked #[ignore] because the C# oracle (dotnet) may not be available in CI.
// Run with: cargo test --test adopt_workflow_parity -- --include-ignored

#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;
use json_semantic::{assert_semantically_equal, assert_semantically_equal_ignoring_fields};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn parse_json_envelope(stdout: &str) -> Value {
    serde_json::from_str(stdout).expect("valid json envelope")
}

fn data_string<'a>(json: &'a Value, label: &str) -> &'a str {
    json.get("data")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{label} JSON envelope missing string data: {json}"))
}

/// Extracts the first well-formed JSON object that follows the `JSON Configuration:` marker
/// embedded in Adopt instruction text.  Used to compare embedded asset configs semantically
/// even when C# (Newtonsoft) and Rust (serde_json) serialize key order differently.
fn extract_json_configuration(output: &str) -> Value {
    let marker = "JSON Configuration:";
    let marker_index = output
        .find(marker)
        .unwrap_or_else(|| panic!("'JSON Configuration:' marker not found in:\n{output}"));
    let suffix = output[marker_index + marker.len()..].trim();
    let json_start = suffix
        .find('{')
        .unwrap_or_else(|| panic!("no JSON object after 'JSON Configuration:' in:\n{output}"));
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
                    return serde_json::from_str(&candidate[..=idx])
                        .expect("valid embedded JSON configuration");
                }
            }
            _ => {}
        }
    }

    panic!("unterminated embedded JSON in output:\n{candidate}");
}

// ---------------------------------------------------------------------------
// Adopt parity tests  (ADAPT-001 … ADAPT-014)
// ---------------------------------------------------------------------------

/// ADAPT-001 — Adopt role `architect` produces matching instruction text + embedded JSON.
// T-1.3: RTM FR-13 / RTM-074
#[ignore = "requires C# oracle"]
#[test]
fn adopt_role_architect_matches_csharp() {
    let sandbox = IsolatedRoot::new("adopt-role-architect").expect("sandbox");
    let args = [
        "--tool",
        "Adopt",
        "--payload",
        r#"{"type":"role","identifier":"architect"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# Adopt(architect) failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust Adopt(architect) failed: {}", rust.stderr);

    // Compare envelope success/error fields; data is a string so we verify it separately.
    assert_semantically_equal_ignoring_fields(&csharp.stdout, &rust.stdout, &["data"]);

    let csharp_json = parse_json_envelope(&csharp.stdout);
    let rust_json = parse_json_envelope(&rust.stdout);
    let csharp_instruction = data_string(&csharp_json, "C# Adopt(architect)").trim_end();
    let rust_instruction = data_string(&rust_json, "Rust Adopt(architect)").trim_end();

    assert!(
        rust_instruction.contains("You are now adopting the role: architect"),
        "expected adoption header in Rust output: {rust_instruction}"
    );
    assert_eq!(
        csharp_instruction, rust_instruction,
        "Adopt(architect) instruction text drifted from C# oracle"
    );
    assert_eq!(
        extract_json_configuration(csharp_instruction),
        extract_json_configuration(rust_instruction),
        "Adopt(architect) embedded JSON configuration drifted from C# oracle"
    );
}

/// ADAPT-007 — Adopt color `blue` produces matching instruction text + embedded JSON.
// T-1.3: RTM FR-13 / RTM-074
#[ignore = "requires C# oracle"]
#[test]
fn adopt_color_blue_matches_csharp() {
    let sandbox = IsolatedRoot::new("adopt-color-blue").expect("sandbox");
    let args = [
        "--tool",
        "Adopt",
        "--payload",
        r#"{"type":"color","identifier":"blue"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# Adopt(blue) failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust Adopt(blue) failed: {}", rust.stderr);

    assert_semantically_equal_ignoring_fields(&csharp.stdout, &rust.stdout, &["data"]);

    let csharp_json = parse_json_envelope(&csharp.stdout);
    let rust_json = parse_json_envelope(&rust.stdout);
    let csharp_instruction = data_string(&csharp_json, "C# Adopt(blue)").trim_end();
    let rust_instruction = data_string(&rust_json, "Rust Adopt(blue)").trim_end();

    assert!(
        rust_instruction.contains("You are now adopting the color: blue"),
        "expected adoption header in Rust output: {rust_instruction}"
    );
    assert_eq!(
        csharp_instruction, rust_instruction,
        "Adopt(blue) instruction text drifted from C# oracle"
    );
    assert_eq!(
        extract_json_configuration(csharp_instruction),
        extract_json_configuration(rust_instruction),
        "Adopt(blue) embedded JSON configuration drifted from C# oracle"
    );
}

/// ADAPT-012 — `ListAssets` for perspective type returns identical asset catalog.
// T-1.3: RTM FR-13 / RTM-074
#[ignore = "requires C# oracle"]
#[test]
fn adopt_perspective_matches_csharp() {
    let sandbox = IsolatedRoot::new("adopt-perspective").expect("sandbox");
    let args = [
        "--tool",
        "ListAssets",
        "--payload",
        r#"{"type":"perspective"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# ListAssets(perspective) failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust ListAssets(perspective) failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

/// ADAPT-013 — Adopt with an invalid `type` value — both CLIs should return an error.
// T-1.3: RTM FR-13 / RTM-074
#[ignore = "requires C# oracle"]
#[test]
fn adopt_invalid_type_matches_csharp() {
    let sandbox = IsolatedRoot::new("adopt-invalid-type").expect("sandbox");
    let args = [
        "--tool",
        "Adopt",
        "--payload",
        r#"{"type":"invalid","identifier":"test"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");

    // Both runtimes must reject an unknown asset type.
    assert_eq!(csharp.exit_code, rust.exit_code, "exit codes differ for invalid Adopt type");
    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

/// ADAPT-014 — Adopt a role identifier that does not exist — both CLIs should error.
// T-1.3: RTM FR-13 / RTM-074
#[ignore = "requires C# oracle"]
#[test]
fn adopt_nonexistent_identifier_matches_csharp() {
    let sandbox = IsolatedRoot::new("adopt-nonexistent-id").expect("sandbox");
    let args = [
        "--tool",
        "Adopt",
        "--payload",
        r#"{"type":"role","identifier":"nonexistent-role-xyz"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");

    // Both runtimes must fail when the requested identifier is absent.
    assert_eq!(
        csharp.exit_code, rust.exit_code,
        "exit codes differ for nonexistent Adopt identifier"
    );
    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

/// ADAPT-009 — `ListAssets` for role type returns matching role catalog.
// T-1.3: RTM FR-13 / RTM-074
#[ignore = "requires C# oracle"]
#[test]
fn list_assets_roles_matches_csharp() {
    let sandbox = IsolatedRoot::new("list-assets-roles").expect("sandbox");
    let args = [
        "--tool",
        "ListAssets",
        "--payload",
        r#"{"type":"role"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# ListAssets(role) failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust ListAssets(role) failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

/// ADAPT-010 — `ListAssets` for color type returns matching color catalog.
// T-1.3: RTM FR-13 / RTM-074
#[ignore = "requires C# oracle"]
#[test]
fn list_assets_colors_matches_csharp() {
    let sandbox = IsolatedRoot::new("list-assets-colors").expect("sandbox");
    let args = [
        "--tool",
        "ListAssets",
        "--payload",
        r#"{"type":"color"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# ListAssets(color) failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust ListAssets(color) failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

/// ADAPT-011 — `ListAssets` for workflow type returns matching workflow catalog.
// T-1.3: RTM FR-13 / RTM-074
#[ignore = "requires C# oracle"]
#[test]
fn list_assets_workflows_matches_csharp() {
    let sandbox = IsolatedRoot::new("list-assets-workflows").expect("sandbox");
    let args = [
        "--tool",
        "ListAssets",
        "--payload",
        r#"{"type":"workflow"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(csharp.success(), "C# ListAssets(workflow) failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(rust.success(), "Rust ListAssets(workflow) failed: {}", rust.stderr);

    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

// ---------------------------------------------------------------------------
// Workflow parity tests  (WORK-001 … WORK-009)
// ---------------------------------------------------------------------------

/// WORK-001 — Starting workflow `agentic-research` produces a matching session envelope.
/// Session IDs and timestamps are inherently dynamic and must be ignored during comparison.
// T-1.3: RTM FR-13 / RTM-067
#[ignore = "requires C# oracle"]
#[test]
fn workflow_start_agentic_research_matches_csharp() {
    let sandbox = IsolatedRoot::new("workflow-agentic-research").expect("sandbox");
    let args = [
        "--tool",
        "Workflow",
        "--payload",
        r#"{"workflowId":"agentic-research"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    assert!(
        csharp.success(),
        "C# Workflow(agentic-research) failed: {}",
        csharp.stderr
    );

    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");
    assert!(
        rust.success(),
        "Rust Workflow(agentic-research) failed: {}",
        rust.stderr
    );

    // Ignore all runtime-generated identifiers and wall-clock fields.
    assert_semantically_equal_ignoring_fields(
        &csharp.stdout,
        &rust.stdout,
        &["sessionId", "startedAt", "updatedAt", "timestamp", "created", "updated"],
    );
}

/// WORK-009 — A workflow ID that does not exist — both CLIs should return an error.
// T-1.3: RTM FR-13 / RTM-067
#[ignore = "requires C# oracle"]
#[test]
fn workflow_invalid_id_matches_csharp() {
    let sandbox = IsolatedRoot::new("workflow-invalid-id").expect("sandbox");
    let args = [
        "--tool",
        "Workflow",
        "--payload",
        r#"{"workflowId":"nonexistent-workflow-id"}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");

    // Both runtimes must reject an unknown workflow ID.
    assert_eq!(
        csharp.exit_code, rust.exit_code,
        "exit codes differ for nonexistent workflow ID"
    );
    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}

/// T-15.14 — `Workflow` called with no identifying parameters — both CLIs should error.
// T-15.14: RTM FR-13
#[ignore = "requires C# oracle"]
#[test]
fn workflow_no_params_matches_csharp() {
    let sandbox = IsolatedRoot::new("workflow-no-params").expect("sandbox");
    let args = ["--tool", "Workflow", "--payload", "{}", "--json"];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp cli");
    let rust = run_rust_cli(&args, &sandbox).expect("rust cli");

    // Both runtimes must reject a Workflow invocation with no workflowId or sessionId.
    assert_eq!(
        csharp.exit_code, rust.exit_code,
        "exit codes differ for Workflow with no params"
    );
    assert_semantically_equal(&csharp.stdout, &rust.stdout);
}
