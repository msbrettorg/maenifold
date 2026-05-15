use super::{build_stdio_adapter_with_root, clear_adapter_cache};
use serde_json::json;

fn write_config(root: &tempfile::TempDir, config: serde_json::Value) {
    std::fs::write(root.path().join("maenifold.json"), config.to_string())
        .expect("write maenifold.json");
}

#[test]
fn adapter_t301_31_returns_preflight_failure_before_spawn_when_env_unresolved() {
    let root = tempfile::tempdir().expect("create temp config root");
    let missing = "/nonexistent/t301-31-adapter-token";
    write_config(
        &root,
        json!({
            "agents": {
                "Codex": {
                    "command": "/bin/echo should-not-spawn",
                    "envFromFile": { "TEST_VAR": missing }
                }
            }
        }),
    );

    clear_adapter_cache();
    let err = match build_stdio_adapter_with_root("Codex", &root.path().to_path_buf()) {
        Ok(_) => panic!("unresolved envFromFile should fail preflight before spawn"),
        Err(err) => err,
    };

    assert!(err.contains("preflight"), "{err}");
    assert!(err.contains("Codex") || err.contains("codex"), "{err}");
    assert!(err.contains("TEST_VAR"), "{err}");
    assert!(err.contains(missing), "{err}");
    assert!(
        err.contains("not found") || err.contains("No such file"),
        "{err}"
    );
}

#[test]
fn adapter_t301_31_dispatches_normally_when_env_resolves() {
    let root = tempfile::tempdir().expect("create temp config root");
    let secret_path = root.path().join("adapter-token");
    std::fs::write(&secret_path, "resolved-token\n").expect("write secret file");
    write_config(
        &root,
        json!({
            "agents": {
                "Codex": {
                    "command": "/bin/echo adapter",
                    "envFromFile": { "TEST_VAR": secret_path.to_string_lossy() }
                }
            }
        }),
    );

    clear_adapter_cache();
    let adapter = build_stdio_adapter_with_root("Codex", &root.path().to_path_buf())
        .expect("valid envFromFile should pass preflight")
        .expect("adapter exists");

    assert_eq!(adapter.binary, "/bin/echo");
    assert_eq!(adapter.args, vec!["adapter"]);
    assert!(adapter
        .env
        .iter()
        .any(|(k, v)| k == "TEST_VAR" && v == "resolved-token"));
}
