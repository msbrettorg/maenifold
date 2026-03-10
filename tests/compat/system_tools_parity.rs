#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use cli_runner::{csharp_assets_dir, run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;
use serde_json::Value;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CSHARP_PROJECT: &str =
    "/Users/brett/src/ma-collective/maenifold/src/Maenifold.csproj";

fn write_memory_file(sandbox: &IsolatedRoot, relative: &str, content: &str) {
    let path = sandbox.root_dir().join("memory").join(relative);
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture directories");
    fs::write(path, content).expect("write fixture markdown");
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

fn workspace_assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .canonicalize()
        .expect("workspace assets dir")
}

fn write_file(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{prefix}-{nanos}-{}", std::process::id())
}

fn remove_file_if_present(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

struct MirroredPackagedAssetGuard {
    rust_path: PathBuf,
    csharp_path: PathBuf,
    file_name: String,
}

impl MirroredPackagedAssetGuard {
    fn create(relative: &str, content: &str) -> io::Result<Self> {
        let rust_path = workspace_assets_dir().join(relative);
        let csharp_path = csharp_assets_dir().join(relative);
        write_file(&rust_path, content)?;
        write_file(&csharp_path, content)?;

        let file_name = Path::new(relative)
            .file_name()
            .expect("fixture file name")
            .to_string_lossy()
            .into_owned();

        Ok(Self {
            rust_path,
            csharp_path,
            file_name,
        })
    }

    fn file_name(&self) -> &str {
        &self.file_name
    }
}

impl Drop for MirroredPackagedAssetGuard {
    fn drop(&mut self) {
        remove_file_if_present(&self.rust_path);
        remove_file_if_present(&self.csharp_path);
    }
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
#[test]
fn adopt_json_matches_csharp_oracle() {
    if !require_csharp_oracle("adopt_json_matches_csharp_oracle") {
        return;
    }

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
    let instruction = rust_json["data"]["instruction"]
        .as_str()
        .expect("adopt instruction");
    assert!(
        instruction.contains("You are now adopting the role: test-role"),
        "expected adoption instruction in output: {instruction}"
    );
    assert_eq!(
        csharp_instruction,
        instruction.trim_end(),
        "Adopt instruction drifted from C# oracle"
    );
    assert_eq!(
        extract_json_configuration(csharp_instruction),
        serde_json::from_str::<Value>(
            rust_json["data"]["jsonConfiguration"]
                .as_str()
                .expect("rust jsonConfiguration"),
        )
        .expect("rust embedded adopt json"),
        "Adopt embedded JSON drifted from C# oracle"
    );
}

// T-10.5: RTM FR-13 / RTM-076 — full oracle parity for [[UpdateAssets]] dry-run JSON summary.
#[test]
fn update_assets_dry_run_json_matches_csharp_oracle() {
    if !require_csharp_oracle("update_assets_dry_run_json_matches_csharp_oracle") {
        return;
    }

    let sandbox = IsolatedRoot::new("update-assets-parity").expect("sandbox");
    let relative = format!("roles/{}.json", unique_name("system-tools-parity-update-assets"));
    let _fixture = MirroredPackagedAssetGuard::create(
        &relative,
        r#"{"id":"system-tools-parity-update-assets","name":"Parity Update Asset","version":2}"#,
    )
    .expect("create packaged fixture");

    let args = [
        "--tool",
        "UpdateAssets",
        "--payload",
        r#"{"dryRun":true}"#,
        "--json",
    ];

    let csharp = run_csharp_cli(&args, &sandbox).expect("csharp update-assets");
    assert!(csharp.success(), "C# UpdateAssets failed: {}", csharp.stderr);

    let rust = run_rust_cli(&args, &sandbox).expect("rust update-assets");
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
    assert!(
        summary.contains("This was a DRY RUN"),
        "expected dry-run summary text: {summary}"
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
