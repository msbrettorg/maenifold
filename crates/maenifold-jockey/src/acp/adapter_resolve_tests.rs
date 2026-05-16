use super::{build_stdio_adapter_with_root, clear_adapter_cache};
use serde_json::json;

fn write_config(root: &tempfile::TempDir) {
    std::fs::write(
        root.path().join("maenifold.json"),
        json!({
            "agents": {
                "Copilot": {
                    "command": "/bin/echo adapter"
                }
            }
        })
        .to_string(),
    )
    .expect("write maenifold.json");
}

#[test]
fn build_stdio_adapter_with_root_resolves_configured_agent_case_insensitively() {
    let root = tempfile::tempdir().expect("create temp config root");
    write_config(&root);

    clear_adapter_cache();
    let adapter = build_stdio_adapter_with_root("copilot", &root.path().to_path_buf())
        .expect("configured agent should resolve")
        .expect("adapter should exist");

    assert_eq!(adapter.runtime_key, "copilot");
    assert_eq!(adapter.binary, "/bin/echo");
    assert_eq!(adapter.args, vec!["adapter"]);
}

#[test]
fn build_stdio_adapter_with_root_errors_for_agent_missing_from_config() {
    let root = tempfile::tempdir().expect("create temp config root");
    write_config(&root);

    clear_adapter_cache();
    let err = match build_stdio_adapter_with_root("nonexistent", &root.path().to_path_buf()) {
        Ok(_) => panic!("missing agent should fail resolution"),
        Err(err) => err,
    };

    assert!(err.contains("not found in maenifold.json"), "{err}");
}
