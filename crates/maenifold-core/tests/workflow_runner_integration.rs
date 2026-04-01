//! Integration tests for [`maenifold_core::workflow::run_workflow`] against a real Codex ACP agent.
//!
//! All tests are `#[ignore]` — run explicitly with:
//!   cargo test -p maenifold-core --test workflow_runner_integration -- --ignored --nocapture
//!
//! Prerequisites:
//!   * `~/maenifold/agents.json` must contain a `"Codex"` entry with a valid ACP command.
//!   * `~/maenifold` must be a valid maenifold root (memory.db, assets/, etc.).

use maenifold_core::{
    AssetManager, Config,
    tools::ToolContext,
    util::json::OutputFormat,
    workflow::{WorkflowResult, WorkflowRunStatus, run_workflow},
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    env, fs,
    path::PathBuf,
};
use tokio::time::{Duration, timeout};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct AgentsFile {
    agents: HashMap<String, AgentConfig>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AgentConfig {
    command: String,
    #[serde(default)]
    env: HashMap<String, String>,
}

fn maenifold_root() -> PathBuf {
    PathBuf::from(env::var_os("HOME").expect("HOME must be set")).join("maenifold")
}

fn load_codex_config() -> AgentConfig {
    let config_path = maenifold_root().join("agents.json");
    let contents = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", config_path.display()));
    let mut file: AgentsFile = serde_json::from_str(&contents)
        .unwrap_or_else(|e| panic!("cannot parse agents.json: {e}"));
    file.agents
        .remove("Codex")
        .expect("Codex agent not found in ~/maenifold/agents.json")
}

/// Build a [`ToolContext`] that points at the real `~/maenifold` root.
fn live_context() -> ToolContext {
    let config = Config::current();
    ToolContext::new(config, OutputFormat::Markdown)
}

/// Write a workflow JSON definition into `~/maenifold/assets/workflows/<id>.json`.
///
/// The definition contains two steps:
/// * `step-1` — plain step, no sequential thinking required.
/// * `step-2` — requires sequential thinking (`requiresSequentialThinking: true`).
fn write_test_workflow(workflow_id: &str) {
    let root = maenifold_root();
    let workflows_dir = root.join("assets").join("workflows");
    fs::create_dir_all(&workflows_dir).expect("create workflows dir");

    let definition = json!({
        "id": workflow_id,
        "name": "Workflow Runner Integration Test",
        "description": "Two-step integration test for [[run-workflow]] runner coverage.",
        "steps": [
            {
                "id": "step-1",
                "name": "Step 1 — Plain Analysis",
                "description": "Perform a brief [[workflow]] analysis. State one key observation about the current task.",
                "requiresSequentialThinking": false,
                "metadata": {
                    "reasoning_effort": "low",
                    "stop_conditions": ["One observation stated"]
                }
            },
            {
                "id": "step-2",
                "name": "Step 2 — Deep Reasoning",
                "description": "Synthesise findings from step 1 using [[sequential-thinking]]. Provide a short conclusion.",
                "requiresSequentialThinking": true,
                "metadata": {
                    "reasoning_effort": "low",
                    "toolHints": {
                        "SequentialThinking": {
                            "estimated_thoughts": 3
                        }
                    },
                    "stop_conditions": ["Conclusion provided"]
                }
            }
        ]
    });

    let dest = workflows_dir.join(format!("{workflow_id}.json"));
    fs::write(
        &dest,
        serde_json::to_string_pretty(&definition).expect("serialize workflow"),
    )
    .unwrap_or_else(|e| panic!("cannot write {}: {e}", dest.display()));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Smoke-test: `run_workflow` drives a 2-step workflow to completion via Codex.
///
/// Verifies:
/// * Both steps are executed (`steps_completed >= 2`).
/// * The workflow terminates with `WorkflowRunStatus::Completed`.
/// * A non-empty `session_id` is returned.
#[tokio::test]
#[ignore = "requires a live Codex ACP agent and ~/maenifold setup"]
async fn workflow_runner_executes_two_step_workflow() {
    let _agent_cfg = load_codex_config(); // confirms agent is configured
    const WORKFLOW_ID: &str = "writ-two-step";

    write_test_workflow(WORKFLOW_ID);

    let ctx = live_context();
    // Ensure the asset layer is initialised (idempotent for an existing root).
    AssetManager::initialize_assets(&ctx.config).expect("asset init");

    let result: WorkflowResult = timeout(
        Duration::from_secs(30),
        run_workflow(
            &ctx,
            WORKFLOW_ID,
            "Codex",
            "Complete each step concisely with [[WikiLinks]] where relevant.",
            None,
        ),
    )
    .await
    .expect("run_workflow timed out after 30 s")
    .expect("run_workflow should succeed");

    assert!(
        !result.session_id.is_empty(),
        "session_id must be non-empty; got: {:?}",
        result.session_id
    );
    assert_eq!(
        result.status,
        WorkflowRunStatus::Completed,
        "workflow must complete successfully; status={:?}",
        result.status
    );
    assert!(
        result.steps_completed >= 2,
        "both steps must execute; steps_completed={}",
        result.steps_completed
    );
    assert!(
        result.conclusion.is_some(),
        "a conclusion must be captured after the final step"
    );
}

/// ST submachine test: step-2 carries `requiresSequentialThinking: true`; the runner
/// must seed a sequential-thinking session and link it as a submachine before the
/// agent responds to that step.
///
/// We verify this indirectly by confirming the workflow still completes (the
/// submachine plumbing is exercised in the path from step-2 through `maybe_run_step_thinking`).
#[tokio::test]
#[ignore = "requires a live Codex ACP agent and ~/maenifold setup"]
async fn workflow_runner_seeds_st_submachine_for_thinking_step() {
    let _agent_cfg = load_codex_config();
    const WORKFLOW_ID: &str = "writ-st-submachine";

    // Re-use the same 2-step definition (step-2 has requiresSequentialThinking: true).
    write_test_workflow(WORKFLOW_ID);

    let ctx = live_context();
    AssetManager::initialize_assets(&ctx.config).expect("asset init");

    let result = timeout(
        Duration::from_secs(30),
        run_workflow(
            &ctx,
            WORKFLOW_ID,
            "Codex",
            "Think carefully. Use [[WikiLinks]] in every response.",
            None,
        ),
    )
    .await
    .expect("run_workflow timed out after 30 s")
    .expect("run_workflow should succeed even when ST submachine is spawned");

    assert_eq!(
        result.status,
        WorkflowRunStatus::Completed,
        "workflow with ST step must complete; status={:?}",
        result.status
    );
    assert!(
        result.steps_completed >= 2,
        "both steps must be driven; steps_completed={}",
        result.steps_completed
    );
}
