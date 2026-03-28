use crate::config::Config;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub(crate) struct ConfigResetGuard;

impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn think_context() -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure dirs");
    (
        tmp,
        ToolContext::new(config, OutputFormat::Markdown),
        ConfigResetGuard,
    )
}

pub(crate) fn workflow_context(
    workflow_id: &str,
    step_count: usize,
) -> (TempDir, ToolContext, ConfigResetGuard) {
    let (tmp, context, reset) = think_context();
    fs::create_dir_all(context.config.assets_path.join("workflows")).expect("workflows dir");
    let steps = (1..=step_count)
        .map(|n| {
            json!({
                "id": format!("step-{n}"),
                "name": format!("Step {n}"),
                "description": format!("Handle [[step-{n}]]")
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        context
            .config
            .assets_path
            .join("workflows")
            .join(format!("{workflow_id}.json")),
        serde_json::to_vec_pretty(&json!({ "steps": steps })).expect("workflow json"),
    )
    .expect("write workflow");
    (tmp, context, reset)
}

pub(crate) fn install_acpx_stub(root: &Path) -> PathGuard {
    let bin = root.join("bin");
    fs::create_dir_all(&bin).expect("bin dir");
    let stub = bin.join("acpx");
    fs::write(&stub, acpx_stub()).expect("write stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&stub).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&stub, permissions).expect("chmod");
    }
    prepend_path(&bin)
}

#[allow(clippy::too_many_lines)]
fn acpx_stub() -> &'static str {
    r#"#!/usr/bin/env python3
import json, os, re, sys
root=os.getcwd(); cfg=os.path.join(root,".runner-stub-config.json"); state=os.path.join(root,".runner-stub-state.json")
def read(path, default):
    try:
        with open(path, "r", encoding="utf-8") as h: return json.load(h)
    except Exception: return default
def read_text(path):
    try:
        with open(path, "r", encoding="utf-8") as h: return h.read()
    except Exception: return ""
def find_session_file(session_id):
    target=session_id + ".md"
    for dirpath, _, filenames in os.walk(root):
        if target in filenames: return os.path.join(dirpath, target)
    return None
def observe_prompt(session, agent):
    observation={"session":session,"agent":agent}
    if session.startswith("think-"):
        sequential_session_id=session[len("think-"):]
        observation["activeSubmachineSessionId"]=sequential_session_id
        sequential_path=find_session_file(sequential_session_id)
        if sequential_path:
            sequential_text=read_text(sequential_path)
            match=re.search(r"\[\[workflow/(workflow-[A-Za-z0-9-]+)\]\]", sequential_text)
            if match:
                workflow_session_id=match.group(1)
                observation["parentWorkflowSession"]=workflow_session_id
                workflow_path=find_session_file(workflow_session_id)
                if workflow_path:
                    workflow_text=read_text(workflow_path)
                    observation["registeredBeforePrompt"]=bool(re.search(r"activeSubmachineSessionId:\s*['\"]?" + re.escape(sequential_session_id) + r"['\"]?", workflow_text))
                    observation["workflowWaiting"]=bool(re.search(r"phase:\s*['\"]?waiting['\"]?", workflow_text))
    return observation
def write(path, data):
    with open(path, "w", encoding="utf-8") as h: json.dump(data, h)
args=sys.argv[1:]; s=read(state, {"counts":{}, "sessions":[], "prompts":[], "observations":[]})
if len(args) >= 5 and args[1:4] == ["sessions", "new", "--name"]:
    s["sessions"].append(args[4]); write(state, s); sys.exit(0)
session=args[args.index("-s")+1] if "-s" in args else ""
agent=args[args.index("-s")-1] if "-s" in args and args.index("-s") > 0 else ""
config=read(cfg, [])
if isinstance(config, list):
    key=session
    entry={"responses": config}
else:
    key=session if session in config else agent if agent in config else "*"
    entry=config.get(key, {})
    if isinstance(entry, list):
        entry={"responses": entry}
count=s["counts"].get(key, 0); s["counts"][key]=count+1; write(state, s)
s["prompts"].append({"session":session,"agent":agent}); write(state, s)
s["observations"].append(observe_prompt(session, agent)); write(state, s)
responses=entry.get("responses", [entry]); behavior=responses[count] if count < len(responses) else responses[-1]
if isinstance(behavior, str): behavior={"text": behavior}
if "stderr" in behavior: print(behavior["stderr"], file=sys.stderr)
if "error" in behavior:
    print(json.dumps({"type":"error","message":behavior["error"]}))
elif "text" in behavior:
    print(json.dumps({"type":"text","text":behavior["text"]}))
sys.exit(int(behavior.get("exitCode", 0)))
"#
}

pub(crate) fn write_stub_config(root: &Path, responses: Value) {
    fs::write(
        root.join(".runner-stub-config.json"),
        serde_json::to_vec(&responses).expect("json"),
    )
    .expect("write config");
}

pub(crate) fn read_stub_state(root: &Path) -> Value {
    serde_json::from_slice(&fs::read(root.join(".runner-stub-state.json")).expect("read state"))
        .expect("parse state")
}

fn prepend_path(path: &Path) -> PathGuard {
    let original = std::env::var_os("PATH");
    let updated = match &original {
        Some(existing) => std::env::join_paths(
            std::iter::once(path.to_path_buf()).chain(std::env::split_paths(existing)),
        )
        .expect("join PATH"),
        None => std::env::join_paths([path]).expect("join PATH"),
    };
    unsafe {
        std::env::set_var("PATH", &updated);
    }
    PathGuard { original }
}

pub(crate) struct PathGuard {
    original: Option<OsString>,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        if let Some(path) = self.original.take() {
            unsafe {
                std::env::set_var("PATH", path);
            }
        } else {
            unsafe {
                std::env::remove_var("PATH");
            }
        }
    }
}
