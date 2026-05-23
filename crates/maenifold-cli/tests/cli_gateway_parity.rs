use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

const GATEWAY_META_TOOLS: &[(&str, &str)] = &[
    ("GatewayIndex", "{}"),
    (
        "GatewayExecute",
        r#"{"domain":"maenifold.system","command":"GetConfig","parameters":{"learn":true}}"#,
    ),
    (
        "GatewayBatch",
        r#"{"operation":"submit","continueOnError":true,"items":[{"domain":"maenifold.system","command":"GetConfig","parameters":{"learn":true}}]}"#,
    ),
];

const LEARN_ONLY_TOOLS: &[&str] = &[
    "GatewayIndex",
    "GatewayExecute",
    "GatewayBatch",
    "GatewayIntent",
    "gate_Status",
    "gate_Sync",
    "gate_Append",
    "gate_Pass",
    "gate_Fail",
    "gate_Reopen",
    "gate_History",
    "gate_TaskBody",
];

#[test]
fn gateway_meta_tools_accept_minimal_valid_payloads() {
    for (tool, payload) in GATEWAY_META_TOOLS {
        let output = run_tool(tool, payload);
        assert_success(tool, payload, &output);
    }
}

#[test]
fn gate_read_tools_accept_minimal_valid_payloads() {
    let root = gate_fixture_root("read-tools");
    let root_json = json_string(&root.display().to_string());
    let task_id = "T-242.16";

    let cases = [
        (
            "gate_Status",
            format!(r#"{{"root":{root_json},"includeDone":true}}"#),
        ),
        (
            "gate_Sync",
            format!(r#"{{"root":{root_json},"rebuild":true}}"#),
        ),
        (
            "gate_History",
            format!(r#"{{"root":{root_json},"taskId":"{task_id}"}}"#),
        ),
        (
            "gate_TaskBody",
            format!(r#"{{"root":{root_json},"taskId":"{task_id}"}}"#),
        ),
    ];

    for (tool, payload) in cases {
        let output = run_tool(tool, &payload);
        assert_success(tool, &payload, &output);
    }
}

#[test]
fn gateway_and_gate_tools_support_learn_short_circuit() {
    for tool in LEARN_ONLY_TOOLS {
        let payload = r#"{"learn":true}"#;
        let output = run_tool(tool, payload);
        assert_success(tool, payload, &output);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.trim().is_empty(),
            "{tool} learn=true returned no help text"
        );
    }
}

fn run_tool(tool: &str, payload: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_maenifold"))
        .args(["tool", tool, payload])
        .output()
        .unwrap_or_else(|err| panic!("failed to run maenifold tool {tool}: {err}"))
}

fn assert_success(tool: &str, payload: &str, output: &Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "{tool} should be reachable from CLI for payload {payload}; status: {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status.code()
    );
    assert!(
        !stdout.contains("Unknown tool:") && !stderr.contains("Unknown tool:"),
        "{tool} hit the CLI Unknown tool path for payload {payload}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

fn gate_fixture_root(name: &str) -> PathBuf {
    let base = std::env::var_os("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .expect("current dir")
                .join("target")
                .join("cli_gateway_parity")
        });
    let root = base.join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("todo")).expect("create fixture todo");
    fs::create_dir_all(root.join("done")).expect("create fixture done");
    fs::write(
        root.join("todo").join("T-242.16.md"),
        "---\n\
         id: T-242.16\n\
         title: CLI gateway parity fixture\n\
         pipeline: sdd\n\
         ---\n\
         # CLI gateway parity fixture\n\n\
         ## Acceptance Criteria\n\n\
         1. CLI gate commands can read this task.\n",
    )
    .expect("write fixture task");
    root
}

fn json_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
