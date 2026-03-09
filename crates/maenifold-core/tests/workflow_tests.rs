use chrono::{Datelike, TimeZone, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{Frontmatter, read_markdown};
use maenifold_core::{AssetManager, Config, Result, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-W1-6: RTM FR-17.6 - isolate workflow parity tests under a temp root.
fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    TestEnv { _tmp: tmp, cfg }
}

// T-W1-6: RTM FR-17.6 - seed temporary workflow assets that mirror the C# test fixtures.
fn write_test_workflow(cfg: &Config, name: &str, step_count: usize, description_prefix: &str) {
    let steps = (1..=step_count)
        .map(|index| {
            json!({
                "id": format!("step-{index}"),
                "name": format!("Step {index}"),
                "description": format!("{description_prefix} {index} description for [[workflow]] coverage")
            })
        })
        .collect::<Vec<_>>();

    let workflow = json!({
        "id": name,
        "name": format!("Test Workflow {name}"),
        "description": format!("Test workflow for [[Workflow]] coverage: {name}"),
        "steps": steps
    });

    let path = cfg
        .assets_path
        .join("workflows")
        .join(format!("{name}.json"));
    fs::create_dir_all(path.parent().expect("workflow parent")).expect("workflow dirs");
    fs::write(
        path,
        serde_json::to_string_pretty(&workflow).expect("workflow json"),
    )
    .expect("write workflow");
}

fn workflow_tools_env() -> TestEnv {
    let env = isolated_env();
    write_test_workflow(&env.cfg, "wtt-alpha", 3, "Test step");
    write_test_workflow(&env.cfg, "wtt-beta", 2, "Test step");
    write_test_workflow(&env.cfg, "wtt-gamma", 1, "Test step");
    env
}

fn workflow_operations_env() -> TestEnv {
    let env = isolated_env();
    write_test_workflow(&env.cfg, "test-workflow-1", 3, "Operations step");
    write_test_workflow(&env.cfg, "test-workflow-2", 2, "Operations step");
    write_test_workflow(&env.cfg, "test-workflow-3", 2, "Operations step");
    env
}

fn invoke_workflow(payload: Value) -> Result<String> {
    ToolRegistry::global().try_invoke("Workflow", &payload, OutputFormat::Markdown)
}

fn extract_session_id(output: &str) -> String {
    output
        .lines()
        .find_map(|line| line.strip_prefix("Created session: "))
        .map(str::trim)
        .map(str::to_string)
        .expect("session id in output")
}

fn start_session(workflow_id: Value) -> (String, String) {
    let output = invoke_workflow(json!({ "workflowId": workflow_id })).expect("start workflow");
    let session_id = extract_session_id(&output);
    (output, session_id)
}

fn workflow_session_uri(session_id: &str) -> String {
    let millis = session_id
        .split('-')
        .nth(1)
        .and_then(|segment| segment.parse::<i64>().ok());
    let timestamp = millis
        .and_then(|value| Utc.timestamp_millis_opt(value).single())
        .unwrap_or_else(Utc::now);
    format!(
        "memory://thinking/workflow/{:04}/{:02}/{:02}/{}",
        timestamp.year(),
        timestamp.month(),
        timestamp.day(),
        session_id
    )
}

fn read_workflow_session(session_id: &str, cfg: &Config) -> (Option<Frontmatter>, String) {
    let uri = workflow_session_uri(session_id);
    let (frontmatter, content, _) = read_markdown(&uri, cfg).expect("read workflow session");
    (frontmatter, content)
}

fn queue_from_frontmatter(frontmatter: &Frontmatter) -> Vec<String> {
    match frontmatter.get("queue") {
        Some(YamlValue::Sequence(values)) => values
            .iter()
            .filter_map(YamlValue::as_str)
            .map(str::to_string)
            .collect(),
        Some(YamlValue::String(value)) if value.trim().starts_with('[') => {
            serde_json::from_str(value).expect("queue json")
        }
        Some(YamlValue::String(value)) if !value.trim().is_empty() => vec![value.clone()],
        _ => Vec::new(),
    }
}

fn frontmatter_usize(frontmatter: &Frontmatter, key: &str) -> usize {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            YamlValue::Number(number) => number.as_u64().map(|value| value as usize),
            YamlValue::String(text) => text.parse::<usize>().ok(),
            _ => None,
        })
        .expect("numeric frontmatter field")
}

// T-W1-6: RTM FR-17.6 - WorkflowToolsTests.cs port (22 tests).
#[test]
fn learn_mode_short_circuits_execution_without_creating_session() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let output = invoke_workflow(json!({ "learn": true })).expect("learn mode");

    assert!(!output.is_empty(), "expected learn output");
    assert!(
        !output.contains("Created session:"),
        "learn mode must not create a session: {output}"
    );
    assert!(
        !output.contains("Workflow usage"),
        "learn mode must bypass operation guidance: {output}"
    );
    assert!(
        !output.contains("Cannot provide both"),
        "learn mode must bypass parameter validation: {output}"
    );
}

#[test]
fn workflow_with_workflow_id_creates_new_session() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let output = invoke_workflow(json!({ "workflowId": "wtt-alpha" })).expect("start workflow");

    assert!(
        output.contains("Created session:"),
        "unexpected output: {output}"
    );
    assert!(output.contains("Step 1/3"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");
}

#[test]
fn workflow_created_session_is_readable_via_markdown_io() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-beta"));
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");

    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
    assert_eq!(
        frontmatter.get("type").and_then(YamlValue::as_str),
        Some("workflow")
    );
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 0);
    assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 0);
}

#[test]
fn workflow_with_nonexistent_workflow_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let error = invoke_workflow(json!({ "workflowId": "does-not-exist-xyz" }))
        .expect_err("missing workflow should error")
        .to_string();

    assert!(
        error.contains("does-not-exist-xyz"),
        "unexpected error: {error}"
    );
    assert!(error.contains("not found"), "unexpected error: {error}");
}

#[test]
fn workflow_with_both_workflow_id_and_session_id_throws() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let error = invoke_workflow(json!({
        "workflowId": "wtt-alpha",
        "sessionId": "workflow-123456789"
    }))
    .expect_err("mixed start+continue should error")
    .to_string();

    assert!(
        error.contains("Cannot provide both workflowId and sessionId"),
        "unexpected error: {error}"
    );
    assert!(error.contains("sessionId"), "unexpected error: {error}");
    assert!(error.contains("workflowId"), "unexpected error: {error}");
}

#[test]
fn workflow_advances_from_step1_to_step2() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Completed step 1 with [[workflow]] and [[session-management]]."
    }))
    .expect("continue workflow");

    assert!(output.contains("Step 2/3"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 1);
}

#[test]
fn workflow_response_without_wikilinks_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Completed step 1 without any double-bracket concepts."
    }))
    .expect("workflow validation output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("[[WikiLink]]"),
        "unexpected output: {output}"
    );
}

#[test]
fn status_transition_active_to_completed() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Final step with [[session-management]].",
        "status": "completed",
        "conclusion": "Session concluded with [[workflow]] insights and [[maenifold]] learnings."
    }))
    .expect("complete workflow");

    assert!(output.contains("completed"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");

    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
    assert!(
        frontmatter.get("completed").is_some(),
        "missing completed timestamp"
    );
    assert!(
        content.contains("Conclusion"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[maenifold]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn completion_without_conclusion_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Done with [[workflow]].",
        "status": "completed"
    }))
    .expect("completion validation output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("Conclusion required"),
        "unexpected output: {output}"
    );
}

#[test]
fn completion_with_conclusion_lacking_wikilinks_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Done with [[workflow]].",
        "status": "completed",
        "conclusion": "This is a conclusion without any WikiLinks."
    }))
    .expect("completion validation output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("[[WikiLinks]]"),
        "unexpected output: {output}"
    );
}

#[test]
fn status_transition_active_to_cancelled() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Cancelling this session with [[cancellation-reason]].",
        "status": "cancelled"
    }))
    .expect("cancel workflow");

    assert!(output.contains("cancelled"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("cancelled")
    );
    assert!(
        frontmatter.get("cancelled").is_some(),
        "missing cancelled timestamp"
    );
}

#[test]
fn serial_queue_starts_with_first_workflow_in_array() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let queue_json = serde_json::to_string(&["wtt-alpha", "wtt-beta"]).expect("queue json");
    let (output, session_id) = start_session(json!(queue_json));

    assert!(output.contains("Step 1/3"), "unexpected output: {output}");
    assert!(
        output.contains("Created session:"),
        "unexpected output: {output}"
    );

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    let queue = queue_from_frontmatter(&frontmatter);
    assert_eq!(queue, vec!["wtt-alpha".to_string(), "wtt-beta".to_string()]);
}

#[test]
fn serial_queue_advances_to_next_workflow_after_first_completes() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let queue_json = serde_json::to_string(&["wtt-gamma", "wtt-beta"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step done with [[workflow-transition]] and [[serial-queuing]]."
    }))
    .expect("transition workflow");

    assert!(output.contains("Step 1/2"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 1);
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 0);
}

#[test]
fn serial_queue_completes_when_all_workflows_done_with_conclusion() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let queue_json = serde_json::to_string(&["wtt-gamma", "wtt-gamma"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let response = "Step done with [[workflow-queue]] completion.";
    let conclusion =
        "All workflows completed — [[serial-queuing]] verified with [[workflow]] mechanics.";

    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("complete first workflow");
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "conclusion": conclusion
    }))
    .expect("complete queue");

    assert!(
        output.contains("All workflows"),
        "unexpected output: {output}"
    );
    assert!(!output.contains("ERROR"), "unexpected output: {output}");

    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
    assert!(
        content.contains("[[serial-queuing]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn serial_queue_exhausted_without_conclusion_prompts_for_conclusion() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-gamma"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Done with [[workflow-queue]] exhaustion test."
    }))
    .expect("queue exhaustion output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("All workflows complete"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("conclusion parameter"),
        "unexpected output: {output}"
    );
}

#[test]
fn view_queue_shows_current_position_and_next_step() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let queue_json =
        serde_json::to_string(&["wtt-alpha", "wtt-beta", "wtt-gamma"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let output = invoke_workflow(json!({ "sessionId": session_id, "view": true }))
        .expect("view workflow queue");

    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("Queue:"), "unexpected output: {output}");
    assert!(output.contains("wtt-alpha"), "unexpected output: {output}");
    assert!(output.contains("wtt-beta"), "unexpected output: {output}");
    assert!(output.contains("wtt-gamma"), "unexpected output: {output}");
    assert!(output.contains("Position:"), "unexpected output: {output}");
    assert!(output.contains("step 1/3"), "unexpected output: {output}");
}

#[test]
fn view_reflects_updated_position_after_step_advancement() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step 1 done with [[step-tracking]]."
    }))
    .expect("advance workflow");
    let output = invoke_workflow(json!({ "sessionId": session_id, "view": true }))
        .expect("view advanced workflow");

    assert!(output.contains("step 2/3"), "unexpected output: {output}");
    assert!(output.contains("Next:"), "unexpected output: {output}");
}

#[test]
fn append_adds_workflow_to_queue() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({ "sessionId": session_id, "append": "wtt-beta" }))
        .expect("append workflow");

    assert!(
        output.contains("Added 1 workflow(s) to queue"),
        "unexpected output: {output}"
    );
    assert!(output.contains("wtt-beta"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[1], "wtt-beta");
}

#[test]
fn append_adds_multiple_workflows_as_json_array() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let append_json = serde_json::to_string(&["wtt-beta", "wtt-gamma"]).expect("append json");
    let output = invoke_workflow(json!({ "sessionId": session_id, "append": append_json }))
        .expect("append workflows");

    assert!(
        output.contains("Added 2 workflow(s) to queue"),
        "unexpected output: {output}"
    );
    assert!(output.contains("wtt-beta"), "unexpected output: {output}");
    assert!(output.contains("wtt-gamma"), "unexpected output: {output}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 3);
}

#[test]
fn conclusion_with_wikilinks_is_stored_in_session_content() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let conclusion = concat!(
        "Completed with insights about [[sequential-thinking]] and [[workflow]] mechanics. ",
        "Key findings: [[maenifold]] enables [[session-management]] at scale."
    );

    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step 1 done with [[workflow]].",
        "status": "completed",
        "conclusion": conclusion
    }))
    .expect("complete workflow");

    let (_, content) = read_workflow_session(&session_id, &env.cfg);
    assert!(
        content.contains("Conclusion"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[sequential-thinking]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[workflow]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[maenifold]]"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[session-management]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn workflow_with_no_parameters_throws_invalid_operation_exception() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let output = invoke_workflow(json!({})).expect("usage guidance");

    assert!(
        output.contains("Workflow usage"),
        "unexpected output: {output}"
    );
    assert!(output.contains("workflowId"), "unexpected output: {output}");
    assert!(output.contains("sessionId"), "unexpected output: {output}");
}

#[test]
fn workflow_with_only_session_id_throws_invalid_operation_exception() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_tools_env();

    let (_, session_id) = start_session(json!("wtt-alpha"));
    let output = invoke_workflow(json!({ "sessionId": session_id })).expect("usage guidance");

    assert!(
        output.contains("Workflow usage"),
        "unexpected output: {output}"
    );
    assert!(output.contains("view=true"), "unexpected output: {output}");
}

// T-W1-6: RTM FR-17.6 - WorkflowOperationsTests.cs port (22 tests).
#[test]
fn append_valid_workflow_to_queue() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "append": "test-workflow-2"
    }))
    .expect("append valid workflow");

    assert!(
        output.contains("Added 1 workflow(s) to queue"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("test-workflow-2"),
        "unexpected output: {output}"
    );
    assert!(!output.contains("ERROR"), "unexpected output: {output}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[1], "test-workflow-2");
}

#[test]
fn append_with_invalid_session_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let invalid_session_id = "1000000000000";
    let output = invoke_workflow(json!({
        "sessionId": invalid_session_id,
        "append": "test-workflow-1"
    }))
    .expect("append invalid session output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains(invalid_session_id),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("doesn't exist"),
        "unexpected output: {output}"
    );
}

#[test]
fn append_with_missing_workflow_file_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let missing = "nonexistent-workflow-xyz";
    let error = invoke_workflow(json!({
        "sessionId": session_id,
        "append": missing
    }))
    .expect_err("missing append workflow should error")
    .to_string();

    assert!(error.contains(missing), "unexpected error: {error}");
    assert!(error.contains("not found"), "unexpected error: {error}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 1);
}

#[test]
fn append_multiple_workflows_as_json_array() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let append_json =
        serde_json::to_string(&["test-workflow-2", "test-workflow-3"]).expect("append json");
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "append": append_json
    }))
    .expect("append workflow array");

    assert!(
        output.contains("Added 2 workflow(s) to queue"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("test-workflow-2"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("test-workflow-3"),
        "unexpected output: {output}"
    );
    assert!(output.contains("New queue:"), "unexpected output: {output}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(
        queue,
        vec![
            "test-workflow-1".to_string(),
            "test-workflow-2".to_string(),
            "test-workflow-3".to_string()
        ]
    );
}

#[test]
fn append_validates_all_workflows_before_queue_update() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let mixed_json = serde_json::to_string(&["test-workflow-2", "nonexistent-workflow-xyz"])
        .expect("mixed json");
    let error = invoke_workflow(json!({
        "sessionId": session_id,
        "append": mixed_json
    }))
    .expect_err("mixed append should error")
    .to_string();

    assert!(
        error.contains("nonexistent-workflow-xyz"),
        "unexpected error: {error}"
    );

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let queue = queue_from_frontmatter(&frontmatter.expect("frontmatter"));
    assert_eq!(queue.len(), 1);
}

#[test]
fn continue_to_next_step_in_same_workflow() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Completed first step with [[workflow-execution]] and [[step-progression]].";
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response
    }))
    .expect("continue workflow");

    assert!(output.contains("Step 2/3"), "unexpected output: {output}");
    assert!(!output.contains("ERROR"), "unexpected output: {output}");

    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 1);
    assert!(
        content.contains("Step 1/3 Response"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[workflow-execution]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn continue_to_next_workflow_when_current_completes() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let queue_json =
        serde_json::to_string(&["test-workflow-3", "test-workflow-2"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let result1 = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step 1 done with [[workflow-transition]]."
    }))
    .expect("continue first step");
    assert!(result1.contains("Step 2/2"), "unexpected output: {result1}");

    let result2 = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step 2 done with [[queue-advancement]]."
    }))
    .expect("transition to next workflow");

    assert!(result2.contains("Step 1/2"), "unexpected output: {result2}");
    assert!(!result2.contains("ERROR"), "unexpected output: {result2}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 1);
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 0);
}

#[test]
fn continue_with_completion_status_finalizes_session() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Step 1 work with [[task-completion]].";
    let conclusion = "Session complete with [[session-summary]] and [[lessons-learned]].";

    invoke_workflow(json!({
        "sessionId": session_id,
        "response": response
    }))
    .expect("record first response");
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "status": "completed",
        "conclusion": conclusion
    }))
    .expect("complete workflow session");

    assert!(output.contains('✅'), "unexpected output: {output}");
    assert!(output.contains("completed"), "unexpected output: {output}");

    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
    assert!(
        frontmatter.get("completed").is_some(),
        "missing completed timestamp"
    );
    assert!(
        content.contains("Conclusion"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[session-summary]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn continue_with_cancellation_status_terminates_early() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Cancelled work with [[cancellation-reason]].",
        "status": "cancelled"
    }))
    .expect("cancel workflow session");

    assert!(output.contains('❌'), "unexpected output: {output}");
    assert!(output.contains("cancelled"), "unexpected output: {output}");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("cancelled")
    );
    assert!(
        frontmatter.get("cancelled").is_some(),
        "missing cancelled timestamp"
    );
}

#[test]
fn continue_without_concepts_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": "This response has no double-bracket concepts."
    }))
    .expect("concept validation output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("[[WikiLink]]"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("knowledge graph"),
        "unexpected output: {output}"
    );
}

#[test]
fn continue_with_invalid_session_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let invalid_session_id = "workflow-1234";
    let output = invoke_workflow(json!({
        "sessionId": invalid_session_id,
        "response": "Response with [[error-validation]]."
    }))
    .expect("invalid session output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains(invalid_session_id),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("doesn't exist"),
        "unexpected output: {output}"
    );
}

#[test]
fn continue_creates_proper_frontmatter_metadata() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Work with [[metadata-tracking]]."
    }))
    .expect("continue workflow");

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.get("title").is_some(), "missing title");
    assert_eq!(
        frontmatter.get("type").and_then(YamlValue::as_str),
        Some("workflow")
    );
    assert_eq!(
        frontmatter.get("permalink").and_then(YamlValue::as_str),
        Some(session_id.as_str())
    );
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
    assert!(frontmatter.get("queue").is_some(), "missing queue");
    assert!(
        frontmatter.get("currentWorkflow").is_some(),
        "missing currentWorkflow"
    );
    assert!(
        frontmatter.get("currentStep").is_some(),
        "missing currentStep"
    );
}

#[test]
fn continue_preserves_thoughts_in_content() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Response with [[step-output]].";
    let thoughts = "Meta thoughts with [[ambient-thinking]] and [[metacognition]].";

    invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "thoughts": thoughts
    }))
    .expect("continue with thoughts");

    let (_, content) = read_workflow_session(&session_id, &env.cfg);
    assert!(content.contains(response), "unexpected content: {content}");
    assert!(content.contains(thoughts), "unexpected content: {content}");
    assert!(
        content.contains("Thoughts:"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[ambient-thinking]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn continue_requires_conclustion_when_queue_exhausted() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-3"));
    let response = "Work with [[completion-state]].";

    invoke_workflow(json!({
        "sessionId": session_id,
        "response": response
    }))
    .expect("advance to final step");
    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response
    }))
    .expect("queue exhaustion output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains("All workflows complete"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("conclusion parameter"),
        "unexpected output: {output}"
    );
}

#[test]
fn continue_updates_current_step_index_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let queue_json =
        serde_json::to_string(&["test-workflow-1", "test-workflow-2"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));

    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    assert_eq!(
        frontmatter_usize(&frontmatter.expect("frontmatter"), "currentStep"),
        0
    );

    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step work with [[state-tracking]]."
    }))
    .expect("continue step 1");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    assert_eq!(
        frontmatter_usize(&frontmatter.expect("frontmatter"), "currentStep"),
        1
    );

    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step work with [[state-tracking]]."
    }))
    .expect("continue step 2");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    assert_eq!(
        frontmatter_usize(&frontmatter.expect("frontmatter"), "currentStep"),
        2
    );

    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step work with [[state-tracking]]."
    }))
    .expect("transition to next workflow");
    let (frontmatter, _) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 0);
    assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 1);
}

#[test]
fn continue_completes_all_workflows_with_conclusion() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let queue_json =
        serde_json::to_string(&["test-workflow-3", "test-workflow-3"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let response = "Step work with [[workflow-completion]].";
    let conclusion = "Final synthesis with [[final-summary]] and [[key-takeaways]].";

    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("first workflow step 1");
    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("first workflow step 2");
    invoke_workflow(json!({ "sessionId": session_id, "response": response }))
        .expect("second workflow step 1");
    let penultimate = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response
    }))
    .expect("queue exhaustion warning");
    assert!(
        penultimate.contains("All workflows complete"),
        "unexpected output: {penultimate}"
    );

    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "conclusion": conclusion
    }))
    .expect("complete all workflows");

    assert!(output.contains('✅'), "unexpected output: {output}");
    assert!(
        output.contains("All workflows"),
        "unexpected output: {output}"
    );

    let (frontmatter, content) = read_workflow_session(&session_id, &env.cfg);
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("completed")
    );
    assert!(
        content.contains("Conclusion"),
        "unexpected content: {content}"
    );
    assert!(
        content.contains("[[final-summary]]"),
        "unexpected content: {content}"
    );
}

#[test]
fn continue_with_thoughts_but_no_conclusion_at_intermediate_step() {
    let _guard = TEST_MUTEX.lock();
    let env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let response = "Response with [[intermediate-results]].";
    let thoughts = "Optional thinking with [[thought-process]].";

    let output = invoke_workflow(json!({
        "sessionId": session_id,
        "response": response,
        "thoughts": thoughts
    }))
    .expect("continue with thoughts");

    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("Step 2/3"), "unexpected output: {output}");

    let (_, content) = read_workflow_session(&session_id, &env.cfg);
    assert!(content.contains(thoughts), "unexpected content: {content}");
}

#[test]
fn view_displays_queue_status_with_current_position() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let queue_json =
        serde_json::to_string(&["test-workflow-1", "test-workflow-2", "test-workflow-3"])
            .expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    let output =
        invoke_workflow(json!({ "sessionId": session_id, "view": true })).expect("view queue");

    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("Queue:"), "unexpected output: {output}");
    assert!(
        output.contains("test-workflow-1"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("test-workflow-2"),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("test-workflow-3"),
        "unexpected output: {output}"
    );
    assert!(output.contains("Position:"), "unexpected output: {output}");
    assert!(
        output.contains("workflow 1/3"),
        "unexpected output: {output}"
    );
    assert!(output.contains("step 1/3"), "unexpected output: {output}");
}

#[test]
fn view_with_invalid_session_id_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let invalid_session_id = "workflow-nonexistent-12345";
    let output = invoke_workflow(json!({
        "sessionId": invalid_session_id,
        "view": true
    }))
    .expect("invalid view output");

    assert!(output.contains("ERROR"), "unexpected output: {output}");
    assert!(
        output.contains(invalid_session_id),
        "unexpected output: {output}"
    );
    assert!(
        output.contains("doesn't exist"),
        "unexpected output: {output}"
    );
}

#[test]
fn view_after_progressing_shows_updated_position() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let queue_json =
        serde_json::to_string(&["test-workflow-1", "test-workflow-2"]).expect("queue json");
    let (_, session_id) = start_session(json!(queue_json));
    invoke_workflow(json!({
        "sessionId": session_id,
        "response": "Step work with [[view-state]]."
    }))
    .expect("advance workflow for view");

    let output = invoke_workflow(json!({ "sessionId": session_id, "view": true }))
        .expect("view updated workflow");

    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("step 2/3"), "unexpected output: {output}");
    assert!(output.contains("Next:"), "unexpected output: {output}");
}

#[test]
fn view_via_workflow_tools_entry_point() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let (_, session_id) = start_session(json!("test-workflow-1"));
    let output = invoke_workflow(json!({ "sessionId": session_id, "view": true }))
        .expect("view via workflow tool");

    assert!(!output.contains("ERROR"), "unexpected output: {output}");
    assert!(output.contains("Queue:"), "unexpected output: {output}");
    assert!(output.contains("Position:"), "unexpected output: {output}");
}

#[test]
fn view_without_session_id_throws_helpful_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = workflow_operations_env();

    let output = invoke_workflow(json!({ "view": true })).expect("usage guidance");

    assert!(
        output.contains("Workflow usage"),
        "unexpected output: {output}"
    );
    assert!(output.contains("view=true"), "unexpected output: {output}");
    assert!(output.contains("sessionId"), "unexpected output: {output}");
}
