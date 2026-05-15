use super::{build_stdio_adapter_with_root, clear_adapter_cache};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().expect("lock environment mutex")
}

fn write_config(root: &tempfile::TempDir, config: serde_json::Value) {
    std::fs::write(root.path().join("maenifold.json"), config.to_string())
        .expect("write maenifold.json");
}

fn resolve_env(root: &tempfile::TempDir) -> HashMap<String, String> {
    clear_adapter_cache();
    build_stdio_adapter_with_root("codex", &root.path().to_path_buf())
        .expect("resolve adapter")
        .expect("adapter exists")
        .env
        .into_iter()
        .collect()
}

#[test]
fn env_from_file_resolves_value() {
    let _guard = lock_env();
    let root = tempfile::tempdir().expect("create config root");
    let home = tempfile::tempdir().expect("create home root");
    let key_path = home.path().join(".tmp-t-324-2-test-key");
    std::fs::write(&key_path, "file-key\r\n").expect("write env file");
    let old_home = std::env::var("HOME").ok();
    unsafe {
        std::env::set_var("HOME", home.path());
    }
    write_config(
        &root,
        json!({
            "agents": {
                "codex": {
                    "command": "/bin/echo adapter",
                    "envFromFile": {
                        "AZURE_OPENAI_API_KEY": "~/.tmp-t-324-2-test-key"
                    }
                }
            }
        }),
    );

    let env = resolve_env(&root);

    assert_eq!(
        env.get("AZURE_OPENAI_API_KEY"),
        Some(&"file-key".to_string())
    );
    unsafe {
        match old_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[test]
fn env_from_file_missing_skips_with_warning() {
    let _guard = lock_env();
    let root = tempfile::tempdir().expect("create config root");
    write_config(
        &root,
        json!({
            "agents": {
                "codex": {
                    "command": "/bin/echo adapter",
                    "envFromFile": {
                        "AZURE_OPENAI_API_KEY": root.path().join("missing-key").to_string_lossy()
                    },
                    "env": {
                        "OTHER_VAR": "still-resolves"
                    }
                }
            }
        }),
    );

    let env = resolve_env(&root);

    assert_eq!(env.get("AZURE_OPENAI_API_KEY"), None);
    assert_eq!(env.get("OTHER_VAR"), Some(&"still-resolves".to_string()));
}

#[test]
fn env_from_command_resolves_value() {
    let _guard = lock_env();
    let root = tempfile::tempdir().expect("create config root");
    write_config(
        &root,
        json!({
            "agents": {
                "codex": {
                    "command": "/bin/echo adapter",
                    "envFromCommand": {
                        "AZURE_OPENAI_API_KEY": {
                            "command": "/bin/echo",
                            "args": ["command-key"]
                        }
                    }
                }
            }
        }),
    );

    let env = resolve_env(&root);

    assert_eq!(
        env.get("AZURE_OPENAI_API_KEY"),
        Some(&"command-key".to_string())
    );
}

#[test]
fn env_from_command_nonzero_exit_skips() {
    let _guard = lock_env();
    let root = tempfile::tempdir().expect("create config root");
    write_config(
        &root,
        json!({
            "agents": {
                "codex": {
                    "command": "/bin/echo adapter",
                    "envFromCommand": {
                        "AZURE_OPENAI_API_KEY": {
                            "command": "/usr/bin/false",
                            "args": []
                        }
                    },
                    "env": {
                        "OTHER_VAR": "still-resolves"
                    }
                }
            }
        }),
    );

    let env = resolve_env(&root);

    assert_eq!(env.get("AZURE_OPENAI_API_KEY"), None);
    assert_eq!(env.get("OTHER_VAR"), Some(&"still-resolves".to_string()));
}

#[test]
fn precedence_literal_overrides_command_overrides_file_overrides_passthrough() {
    let _guard = lock_env();
    let root = tempfile::tempdir().expect("create config root");
    let literal_file = root.path().join("literal-key");
    let command_file = root.path().join("command-key");
    let file_only = root.path().join("file-only-key");
    std::fs::write(&literal_file, "file-ignored\n").expect("write literal env file");
    std::fs::write(&command_file, "file-ignored\n").expect("write command env file");
    std::fs::write(&file_only, "file-value\n").expect("write file env file");
    unsafe {
        std::env::set_var("T324_LITERAL", "passthrough-ignored");
        std::env::set_var("T324_COMMAND", "passthrough-ignored");
        std::env::set_var("T324_FILE", "passthrough-ignored");
        std::env::set_var("T324_PASSTHROUGH", "passthrough-value");
    }
    write_config(
        &root,
        json!({
            "settings": {
                "envPassthrough": [
                    "T324_LITERAL",
                    "T324_COMMAND",
                    "T324_FILE",
                    "T324_PASSTHROUGH"
                ]
            },
            "agents": {
                "codex": {
                    "command": "/bin/echo adapter",
                    "env": {
                        "T324_LITERAL": "literal-value"
                    },
                    "envFromCommand": {
                        "T324_LITERAL": {
                            "command": "/bin/echo",
                            "args": ["command-ignored"]
                        },
                        "T324_COMMAND": {
                            "command": "/bin/echo",
                            "args": ["command-value"]
                        }
                    },
                    "envFromFile": {
                        "T324_LITERAL": literal_file.to_string_lossy(),
                        "T324_COMMAND": command_file.to_string_lossy(),
                        "T324_FILE": file_only.to_string_lossy()
                    }
                }
            }
        }),
    );

    let env = resolve_env(&root);

    assert_eq!(env.get("T324_LITERAL"), Some(&"literal-value".to_string()));
    assert_eq!(env.get("T324_COMMAND"), Some(&"command-value".to_string()));
    assert_eq!(env.get("T324_FILE"), Some(&"file-value".to_string()));
    assert_eq!(
        env.get("T324_PASSTHROUGH"),
        Some(&"passthrough-value".to_string())
    );
    unsafe {
        std::env::remove_var("T324_LITERAL");
        std::env::remove_var("T324_COMMAND");
        std::env::remove_var("T324_FILE");
        std::env::remove_var("T324_PASSTHROUGH");
    }
}
