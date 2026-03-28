use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::time::{Instant, sleep};

pub(super) fn install_acpx_stub(root: &Path) -> PathGuard {
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    let stub_path = bin_dir.join("acpx");
    fs::write(&stub_path, acpx_stub_script()).expect("write acpx stub");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&stub_path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&stub_path, permissions).expect("chmod");
    }

    prepend_path(&bin_dir)
}

fn acpx_stub_script() -> &'static str {
    r#"#!/usr/bin/env python3
import fcntl
import json
import os
import sys
import time

ROOT = os.getcwd()
CONFIG_PATH = os.path.join(ROOT, ".executor-stub-config.json")
STATE_PATH = os.path.join(ROOT, ".executor-stub-state.json")
LOCK_PATH = os.path.join(ROOT, ".executor-stub.lock")

def read_json(path, default):
    try:
        with open(path, "r", encoding="utf-8") as handle:
            return json.load(handle)
    except Exception:
        return default

def write_state(state):
    with open(STATE_PATH, "w", encoding="utf-8") as handle:
        json.dump(state, handle)

def mutate_state(mutator):
    with open(LOCK_PATH, "a+", encoding="utf-8") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state = read_json(STATE_PATH, {"current": 0, "max": 0, "events": []})
        mutator(state)
        write_state(state)
        fcntl.flock(lock, fcntl.LOCK_UN)

args = sys.argv[1:]
prompt = args[-1] if args else ""
session = ""
agent = ""
if "-s" in args:
    index = args.index("-s")
    if index + 1 < len(args):
        session = args[index + 1]
    if index > 0:
        agent = args[index - 1]

config = read_json(CONFIG_PATH, {})
behavior = config.get(session) or config.get(agent) or config.get("*") or {}

mutate_state(lambda state: (
    state.__setitem__("current", state.get("current", 0) + 1),
    state.__setitem__("max", max(state.get("max", 0), state.get("current", 0))),
    state.setdefault("events", []).append({"event": "start", "session": session, "agent": agent, "prompt": prompt})
))

try:
    delay = float(behavior.get("delay", 0))
    if delay > 0:
        time.sleep(delay)

    if "stderr" in behavior:
        print(behavior["stderr"], file=sys.stderr)

    if "error" in behavior:
        print(json.dumps({"type": "error", "message": behavior["error"]}))
    else:
        print(json.dumps({"type": "text", "text": behavior.get("text", "")}))

    sys.exit(int(behavior.get("exitCode", 0)))
finally:
    mutate_state(lambda state: (
        state.__setitem__("current", max(0, state.get("current", 0) - 1)),
        state.setdefault("events", []).append({"event": "end", "session": session})
    ))
"#
}

pub(super) fn write_stub_config(root: &Path, config: Value) {
    fs::write(
        root.join(".executor-stub-config.json"),
        serde_json::to_vec_pretty(&config).expect("serialize stub config"),
    )
    .expect("write stub config");
}

pub(super) async fn wait_for_stub_max(root: &Path, expected: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if read_stub_max(root) >= expected {
            return;
        }
        sleep(Duration::from_millis(25)).await;
    }
    panic!("stub max concurrency never reached {expected}");
}

pub(super) fn read_stub_max(root: &Path) -> u64 {
    read_stub_state(root)
        .get("max")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

pub(super) fn read_stub_event_order(root: &Path) -> Vec<String> {
    read_stub_state(root)
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|event| event.get("event").and_then(Value::as_str) == Some("start"))
        .filter_map(|event| {
            event
                .get("agent")
                .and_then(Value::as_str)
                .filter(|agent| !agent.is_empty())
                .map(str::to_string)
        })
        .collect()
}

pub(super) fn read_stub_prompts(root: &Path, agent: &str) -> Vec<String> {
    read_stub_state(root)
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|event| event.get("event").and_then(Value::as_str) == Some("start"))
        .filter(|event| event.get("agent").and_then(Value::as_str) == Some(agent))
        .filter_map(|event| {
            event
                .get("prompt")
                .and_then(Value::as_str)
                .filter(|prompt| !prompt.is_empty())
                .map(str::to_string)
        })
        .collect()
}

fn read_stub_state(root: &Path) -> Value {
    let state_path = root.join(".executor-stub-state.json");
    if !state_path.exists() {
        return json!({"current": 0, "max": 0, "events": []});
    }
    serde_json::from_slice(&fs::read(&state_path).expect("read stub state"))
        .expect("parse stub state")
}

fn prepend_path(path: &Path) -> PathGuard {
    let original = std::env::var_os("PATH");
    let updated = match &original {
        Some(existing) => {
            let mut parts = vec![path.to_path_buf()];
            parts.extend(std::env::split_paths(existing));
            std::env::join_paths(parts).expect("join PATH")
        }
        None => std::env::join_paths([path]).expect("join PATH"),
    };

    unsafe {
        std::env::set_var("PATH", &updated);
    }

    PathGuard { original }
}

pub(super) struct PathGuard {
    original: Option<OsString>,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(path) => unsafe {
                std::env::set_var("PATH", path);
            },
            None => unsafe {
                std::env::remove_var("PATH");
            },
        }
    }
}
