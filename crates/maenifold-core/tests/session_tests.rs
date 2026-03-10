// T-W1-5: RTM FR-17.9

use chrono::{Datelike, TimeZone, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{Frontmatter, read_markdown, write_markdown};
use maenifold_core::sessions::{
    validate_assumption_append_input, validate_assumption_read_input, validate_assumption_status,
    validate_assumption_update_input,
};
use maenifold_core::{AssetManager, Config, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestHarness {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    config: Config,
}

impl TestHarness {
    fn new() -> Self {
        let guard = TEST_MUTEX.lock();
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("dirs");
        AssetManager::initialize_assets(&config).expect("assets");
        Self {
            _guard: guard,
            _tmp: tmp,
            config,
        }
    }

    fn invoke_markdown(&self, tool: &str, payload: Value) -> String {
        ToolRegistry::global()
            .try_invoke(tool, &payload, OutputFormat::Markdown)
            .expect("tool invocation")
    }

    fn read(&self, uri: &str) -> (Frontmatter, String) {
        let (frontmatter, content, _) = read_markdown(uri, &self.config).expect("read markdown");
        (frontmatter.expect("frontmatter"), content)
    }

    fn write(&self, uri: &str, frontmatter: &Frontmatter, content: &str) {
        write_markdown(uri, Some(frontmatter), content, &self.config).expect("write markdown");
    }

    fn create_workflow_session(&self) -> String {
        let output =
            self.invoke_markdown("Workflow", json!({ "workflowId": "deductive-reasoning" }));
        extract_session_id(&output)
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn extract_session_id(response_message: &str) -> String {
    response_message
        .lines()
        .find_map(|line| {
            if line.contains("Created session:") || line.contains("to session:") {
                line.split(':')
                    .next_back()
                    .map(|value| value.trim().to_string())
            } else {
                None
            }
        })
        .expect("session id")
}

fn extract_memory_uri(response_message: &str) -> String {
    let start = response_message
        .find("memory://")
        .expect("memory uri start");
    let remainder = &response_message[start..];
    let end = remainder
        .find(char::is_whitespace)
        .unwrap_or(remainder.len());
    remainder[..end].to_string()
}

fn session_uri(kind: &str, session_id: &str) -> String {
    let millis = session_id
        .split('-')
        .nth(1)
        .and_then(|value| value.parse::<i64>().ok())
        .expect("timestamp millis");
    let timestamp = Utc
        .timestamp_millis_opt(millis)
        .single()
        .expect("timestamp");
    format!(
        "memory://thinking/{kind}/{:04}/{:02}/{:02}/{session_id}",
        timestamp.year(),
        timestamp.month(),
        timestamp.day()
    )
}

fn frontmatter_string(frontmatter: &Frontmatter, key: &str) -> String {
    match frontmatter.get(key) {
        Some(YamlValue::String(value)) => value.clone(),
        Some(YamlValue::Number(value)) => value.to_string(),
        Some(YamlValue::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn frontmatter_sequence(frontmatter: &Frontmatter, key: &str) -> Vec<String> {
    match frontmatter.get(key) {
        Some(YamlValue::Sequence(values)) => values
            .iter()
            .filter_map(YamlValue::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn new_session_with_concepts_creates_session_file() {
    let harness = TestHarness::new();
    let response =
        "This is a test thought with [[sequential-thinking]] and [[session-management]] tags.";

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": response,
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 3
        }),
    );

    assert!(result.contains("Created session:"));
    assert!(result.contains("Continue with thought 1/3"));

    let session_id = extract_session_id(&result);
    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);

    assert!(frontmatter_string(&frontmatter, "title").contains("Sequential Thinking Session"));
    assert_eq!(frontmatter_string(&frontmatter, "type"), "sequential");
    assert_eq!(frontmatter_string(&frontmatter, "status"), "active");
    assert_eq!(frontmatter_string(&frontmatter, "permalink"), session_id);
    assert!(content.contains("Thought 0/3"));
    assert!(content.contains("test thought with [[sequential-thinking]]"));
}

#[test]
fn new_session_with_concepts_in_response_passes_validation() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Analyzing [[machine-learning]] algorithms with [[neural-networks]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 0,
            "totalThoughts": 1,
            "conclusion": "Concluded that [[deep-learning]] is important."
        }),
    );

    assert!(result.contains("Created session:"));
    assert!(result.contains("Thinking complete"));
    assert!(!result.starts_with("ERROR:"));
}

#[test]
fn new_session_without_concepts_fails_validation() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "This thought has no concepts at all.",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("Must include [[WikiLinks]]"));
}

#[test]
fn continue_session_with_valid_session_id_appends_thought() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "First thought with [[data-analysis]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let continue_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Second thought with [[pattern-recognition]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "conclusion": "Final synthesis with [[concept-synthesis]]."
        }),
    );

    assert!(continue_result.contains("Added thought 1 to session:"));
    assert!(continue_result.contains("Thinking complete"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);
    assert!(content.contains("Thought 0/2"));
    assert!(content.contains("Thought 1/2"));
    assert!(content.contains("First thought with [[data-analysis]]"));
    assert!(content.contains("Second thought with [[pattern-recognition]]"));
    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "2");
}

#[test]
fn new_session_with_parent_workflow_id_creates_bidirectional_links() {
    let harness = TestHarness::new();
    let parent_session_id = harness.create_workflow_session();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thinking within workflow with [[analysis]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "parentWorkflowId": parent_session_id
        }),
    );

    let session_id = extract_session_id(&result);
    let child_uri = session_uri("sequential", &session_id);
    let parent_uri = session_uri("workflow", &parent_session_id);

    let (child_frontmatter, _) = harness.read(&child_uri);
    let (parent_frontmatter, _) = harness.read(&parent_uri);

    assert!(frontmatter_string(&child_frontmatter, "parent").contains(&parent_session_id));
    assert!(
        frontmatter_sequence(&parent_frontmatter, "related")
            .iter()
            .any(|value| value.contains(&session_id))
    );
}

#[test]
fn new_session_with_closed_parent_workflow_returns_error() {
    let harness = TestHarness::new();
    let parent_session_id = harness.create_workflow_session();
    let parent_uri = session_uri("workflow", &parent_session_id);
    let (mut parent_frontmatter, parent_content) = harness.read(&parent_uri);
    parent_frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    harness.write(&parent_uri, &parent_frontmatter, &parent_content);

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thinking within closed [[workflow-state]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "parentWorkflowId": parent_session_id
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("is completed"));
}

#[test]
fn session_with_cancel_updates_status_to_cancelled() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Initial thought with [[session-lifecycle]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 5
        }),
    );
    let session_id = extract_session_id(&create_result);

    let cancel_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Cancelling thoughts with [[workflow-cancellation]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 1,
            "totalThoughts": 5,
            "sessionId": session_id,
            "cancel": true
        }),
    );

    assert!(cancel_result.contains("Thinking cancelled"));
    assert!(cancel_result.contains("Added thought 1 to session:"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, _) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "cancelled");
    assert!(frontmatter.contains_key("cancelled"));
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "2");
}

#[test]
fn session_with_cancel_without_response_updates_status_to_cancelled() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Initial thought with [[cancel-workflow]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 5
        }),
    );
    let session_id = extract_session_id(&create_result);

    let cancel_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "sessionId": session_id,
            "cancel": true
        }),
    );

    assert!(cancel_result.contains("Thinking cancelled"));
    assert!(!cancel_result.starts_with("ERROR:"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, _) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "cancelled");
    assert!(frontmatter.contains_key("cancelled"));
    assert!(frontmatter.contains_key("thoughtCount"));
}

#[test]
fn session_with_conclusion_completes_session() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought 1 with [[requirements-analysis]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let complete_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought 2 with [[solution-design]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "conclusion": "This analysis demonstrates [[synthesis]] of concepts."
        }),
    );

    assert!(complete_result.contains("Thinking complete"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");
    assert!(frontmatter.contains_key("completed"));
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "2");
    assert!(content.contains("Conclusion"));
    assert!(content.contains("[[synthesis]]"));
}

#[test]
fn session_with_revision_creates_revision_heading() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Original thought with [[initial-idea]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let revision_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Revised thought with [[refined-approach]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "isRevision": true,
            "revisesThought": 0
        }),
    );

    assert!(revision_result.contains("Added thought 1 to session:"));
    let uri = session_uri("sequential", &session_id);
    let (_, content) = harness.read(&uri);
    assert!(content.contains("(Revises: 0)"));
}

#[test]
fn session_with_branch_creates_branch_heading() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Main branch thought with [[branching-strategy]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Main branch thought 2 with [[revision-control]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "conclusion": "Main branch conclusion with [[thought-divergence]]."
        }),
    );

    let branch_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Branched thought with [[alternative-concept]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "sessionId": session_id,
            "branchFromThought": 0,
            "branchId": "alternative-analysis"
        }),
    );

    assert!(branch_result.contains("Created session:"));
    let uri = session_uri("sequential", &session_id);
    let (_, content) = harness.read(&uri);
    assert!(content.contains("(Branch: alternative-analysis)"));
}

#[test]
fn session_with_needs_more_thoughts_expands_total_thoughts() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "First thought with [[api-design]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let expand_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Second thought with [[implementation-details]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "needsMoreThoughts": true
        }),
    );

    assert!(expand_result.contains("Continue with thought 2/?"));
    assert!(expand_result.contains("extending beyond initial estimate"));
}

#[test]
fn new_session_with_invalid_parent_workflow_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thinking with [[workflow-validation]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "parentWorkflowId": "nonexistent-workflow-12345"
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("not found"));
}

#[test]
fn continue_session_with_parent_workflow_on_thought_2_returns_error() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "First thought with [[parent-workflow-linking]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);
    let parent_session_id = harness.create_workflow_session();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Second thought with [[error-handling]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 2,
            "sessionId": session_id,
            "parentWorkflowId": parent_session_id
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("Parent workflow can only be set on first thought"));
}

#[test]
fn complete_session_lifecycle_create_continue_complete_succeeds() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Analyzing problem space with [[architecture]] and [[design-patterns]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 3,
            "analysisType": "architecture"
        }),
    );
    let session_id = extract_session_id(&create_result);

    let continue_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Implementing solution with [[implementation]] details and [[testing]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 3,
            "sessionId": session_id,
            "thoughts": "Considering [[performance]] implications."
        }),
    );
    assert!(continue_result.contains("Added thought 1 to session:"));

    let complete_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Reviewing and refining with [[quality-assurance]].",
            "nextThoughtNeeded": false,
            "thoughtNumber": 2,
            "totalThoughts": 3,
            "sessionId": session_id,
            "conclusion": "Concluded that [[modular-design]] with [[robust-testing]] is the best approach."
        }),
    );
    assert!(complete_result.contains("Thinking complete"));

    let uri = session_uri("sequential", &session_id);
    let (frontmatter, content) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");
    assert_eq!(frontmatter_string(&frontmatter, "type"), "sequential");
    assert_eq!(
        frontmatter_string(&frontmatter, "analysisType"),
        "architecture"
    );
    assert!(frontmatter.contains_key("completed"));
    assert_eq!(frontmatter_string(&frontmatter, "thoughtCount"), "3");
    assert!(content.contains("Thought 0/3"));
    assert!(content.contains("Thought 1/3"));
    assert!(content.contains("Thought 2/3"));
    assert!(content.contains("Analyzing problem space"));
    assert!(content.contains("Implementing solution"));
    assert!(content.contains("Reviewing and refining"));
    assert!(content.contains("Conclusion"));
    assert!(content.contains("[[modular-design]]"));
}

#[test]
fn new_session_extracts_concepts_correctly() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Exploring [[machine-learning]], [[neural-networks]], and [[deep-learning]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 1,
            "thoughts": "Related to [[AI]], [[algorithms]], and [[data-science]]."
        }),
    );

    let session_id = extract_session_id(&result);
    let uri = session_uri("sequential", &session_id);
    let (_, content) = harness.read(&uri);
    assert!(content.contains("machine-learning"));
    assert!(content.contains("Thoughts:"));
    assert!(content.contains("AI"));
}

#[test]
fn continue_session_with_nonexistent_session_id_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought with [[validation-testing]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 3,
            "sessionId": "session-9999999999999-12345"
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("missing"));
}

#[test]
fn continue_session_with_invalid_session_id_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought with [[session-validation]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 1,
            "totalThoughts": 3,
            "sessionId": "session-9999999999999"
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("Invalid sessionId format"));
}

#[test]
fn session_with_branch_from_thought_without_branch_id_returns_error() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Main thought with [[multi-agent-coordination]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );
    let session_id = extract_session_id(&create_result);

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Branched thought with [[alternative-approach]].",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2,
            "sessionId": session_id,
            "branchFromThought": 0,
            "branchId": null
        }),
    );

    assert!(result.starts_with("ERROR:"));
    assert!(result.contains("branchId required"));
    assert!(result.contains("multi-agent coordination"));
}

#[test]
fn append_assumption_creates_file_with_correct_structure() {
    let harness = TestHarness::new();
    let assumption = "The dialogue tool will remain MCP-only";
    let confidence = "medium";
    let validation_plan = "Validate once dialogue MCP hooks are reintroduced";
    let context = "workflow://thinking/session-1756610546730";
    let now = Utc::now();
    let date_dir = harness
        .config
        .memory_path
        .join("assumptions")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()));
    let files_before = std::fs::read_dir(&date_dir)
        .ok()
        .map(|entries| entries.count())
        .unwrap_or(0);

    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": assumption,
            "context": context,
            "validationPlan": validation_plan,
            "confidence": confidence,
            "concepts": ["dialogue", "workflow-dispatch"]
        }),
    );

    assert!(result.starts_with("✅ Assumption recorded at memory://assumptions/"));
    assert!(result.contains(&format!("**Statement**: {assumption}")));
    assert!(result.contains(&format!("**Confidence**: {confidence}")));
    assert!(result.contains("**Status**: active"));
    assert!(result.contains(&format!("**Context**: {context}")));
    assert!(result.contains("💡 Next: Consider running Sync()"));

    let files_after = std::fs::read_dir(&date_dir).expect("date dir").count();
    assert_eq!(files_after, files_before + 1);

    let uri = extract_memory_uri(&result);
    let (frontmatter, content) = harness.read(&uri);
    assert_eq!(frontmatter_string(&frontmatter, "status"), "active");
    assert_eq!(frontmatter_string(&frontmatter, "confidence"), confidence);
    assert_eq!(frontmatter_string(&frontmatter, "context"), context);
    assert_eq!(
        frontmatter_string(&frontmatter, "validation_plan"),
        validation_plan
    );
    assert!(content.contains("[[dialogue]]"));
    assert!(content.contains("[[workflow-dispatch]]"));
}

#[test]
fn append_assumption_requires_concepts() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption",
            "concepts": []
        }),
    );

    assert!(result.starts_with("ERROR: At least one [[WikiLink]] tag must be provided"));
}

#[test]
fn append_assumption_rejects_concepts_with_brackets() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption",
            "concepts": ["[[dialogue]]"]
        }),
    );

    assert!(result.starts_with("ERROR: Concept tags should not include brackets"));
}

#[test]
fn read_assumption_returns_correct_content() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption for reading",
            "validationPlan": "Verify with [[test-concept]] follow-up",
            "concepts": ["test-concept"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );

    assert!(result.starts_with(&format!("📋 **Assumption**: {uri}")));
    assert!(result.contains("**Status**: active"));
    assert!(result.contains("**Created**:"));
    assert!(result.contains("**Modified**:"));
    assert!(result.contains("**Validation Plan**: Verify with [[test-concept]] follow-up"));
    assert!(result.contains("## Content"));
    assert!(result.contains("Test assumption for reading"));
    assert!(result.contains("[[test-concept]]"));
}

#[test]
fn update_assumption_changes_status() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption for updating",
            "concepts": ["test-update"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let update_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "status": "validated",
            "notes": "Test validation complete"
        }),
    );

    assert!(update_result.starts_with("✅ Assumption updated at"));
    assert!(update_result.contains("**New Status**: validated"));

    let read_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );

    assert!(read_result.contains("**Status**: validated"));
    assert!(read_result.contains("Test validation complete"));
}

#[test]
fn update_assumption_persists_validation_plan() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption for validation plan updates",
            "concepts": ["test-update"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let update_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "validationPlan": "Confirm via [[test-update]] review"
        }),
    );

    assert!(update_result.starts_with("✅ Assumption updated at"));

    let read_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );

    assert!(read_result.contains("**Validation Plan**: Confirm via [[test-update]] review"));
}

#[test]
fn update_assumption_rejects_invalid_status() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption",
            "concepts": ["test"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "status": "invalid-status"
        }),
    );

    assert!(result.starts_with("ERROR: Invalid status"));
    assert!(result.contains("active, validated, invalidated, refined"));
}

#[test]
fn update_assumption_adds_timestamped_notes() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test assumption for notes",
            "concepts": ["test-notes"]
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let update_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "notes": "Additional context added"
        }),
    );

    assert!(update_result.starts_with("✅ Assumption updated at"));
    let read_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );
    assert!(read_result.contains("## Update:"));
    assert!(read_result.contains("Additional context added"));
}

#[test]
fn read_non_existent_assumption_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": "memory://assumptions/2025/01/assumption-nonexistent"
        }),
    );

    assert!(result.starts_with("ERROR: Assumption not found"));
}

#[test]
fn update_non_existent_assumption_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": "memory://assumptions/2025/01/assumption-nonexistent",
            "status": "validated"
        }),
    );

    assert!(result.starts_with("ERROR: Assumption not found"));
}

#[test]
fn invalid_action_returns_error() {
    let harness = TestHarness::new();
    let result = harness.invoke_markdown("AssumptionLedger", json!({ "action": "invalid-action" }));

    assert!(result.starts_with("ERROR: Invalid action"));
    assert!(result.contains("'append', 'update', or 'read'"));
}

#[test]
fn assumption_file_uses_correct_date_based_path() {
    let harness = TestHarness::new();
    let now = Utc::now();
    let year = format!("{:04}", now.year());
    let month = format!("{:02}", now.month());
    let date_dir = harness
        .config
        .memory_path
        .join("assumptions")
        .join(&year)
        .join(&month);
    let files_before = std::fs::read_dir(&date_dir)
        .ok()
        .map(|entries| entries.count())
        .unwrap_or(0);

    let result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test date path",
            "concepts": ["test"]
        }),
    );

    assert!(result.contains(&format!("memory://assumptions/{year}/{month}/assumption-")));
    let files_after = std::fs::read_dir(&date_dir).expect("date dir").count();
    assert_eq!(files_after, files_before + 1);
}

#[test]
fn update_assumption_changes_confidence() {
    let harness = TestHarness::new();
    let create_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "append",
            "assumption": "Test confidence update",
            "concepts": ["test"],
            "confidence": "low"
        }),
    );
    let uri = extract_memory_uri(&create_result);

    let update_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "update",
            "uri": uri,
            "confidence": "high"
        }),
    );

    assert!(update_result.contains("**New Confidence**: high"));
    let read_result = harness.invoke_markdown(
        "AssumptionLedger",
        json!({
            "action": "read",
            "uri": uri
        }),
    );
    assert!(read_result.contains("**Confidence**: high"));
}

#[test]
fn validate_append_input_null_assumption_returns_error() {
    let concepts = vec!["concept".to_string()];
    let result = validate_assumption_append_input(None, Some(&concepts));
    assert!(result.is_some());
    assert!(
        result
            .expect("error")
            .contains("Assumption text is required")
    );
}

#[test]
fn validate_append_input_empty_assumption_returns_error() {
    let concepts = vec!["concept".to_string()];
    let result = validate_assumption_append_input(Some(""), Some(&concepts));
    assert!(result.is_some());
    assert!(
        result
            .expect("error")
            .contains("Assumption text is required")
    );
}

#[test]
fn validate_append_input_whitespace_assumption_returns_error() {
    let concepts = vec!["concept".to_string()];
    let result = validate_assumption_append_input(Some("   "), Some(&concepts));
    assert!(result.is_some());
    assert!(
        result
            .expect("error")
            .contains("Assumption text is required")
    );
}

#[test]
fn validate_append_input_null_concepts_returns_error() {
    let result = validate_assumption_append_input(Some("Valid assumption"), None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("[[WikiLink]]"));
}

#[test]
fn validate_append_input_empty_concepts_array_returns_error() {
    let concepts: Vec<String> = Vec::new();
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("[[WikiLink]]"));
}

#[test]
fn validate_append_input_concept_with_full_brackets_returns_error() {
    let concepts = vec!["[[foo]]".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("brackets"));
}

#[test]
fn validate_append_input_concept_with_only_open_bracket_returns_error() {
    let concepts = vec!["[[dialogue".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("brackets"));
}

#[test]
fn validate_append_input_concept_with_only_close_bracket_returns_error() {
    let concepts = vec!["dialogue]]".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("brackets"));
}

#[test]
fn validate_append_input_valid_single_concept_returns_null() {
    let concepts = vec!["dialogue".to_string()];
    assert!(
        validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts)).is_none()
    );
}

#[test]
fn validate_append_input_valid_multiple_concepts_returns_null() {
    let concepts = vec![
        "dialogue".to_string(),
        "workflow-dispatch".to_string(),
        "assumption-ledger".to_string(),
    ];
    assert!(
        validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts)).is_none()
    );
}

#[test]
fn validate_append_input_bracket_error_on_second_concept_returns_error() {
    let concepts = vec!["good-concept".to_string(), "[[bad-concept]]".to_string()];
    let result = validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts));
    assert!(result.is_some());
    assert!(result.expect("error").contains("brackets"));
}

#[test]
fn validate_update_input_null_uri_returns_error() {
    let result = validate_assumption_update_input(None, None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_update_input_empty_uri_returns_error() {
    let result = validate_assumption_update_input(Some(""), None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_update_input_whitespace_uri_returns_error() {
    let result = validate_assumption_update_input(Some("   "), None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_update_input_valid_uri_null_status_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            None,
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_empty_status_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some(""),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_status_active_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("active"),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_status_validated_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("validated"),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_status_invalidated_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("invalidated"),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_status_refined_returns_null() {
    assert!(
        validate_assumption_update_input(
            Some("memory://assumptions/2026/02/assumption-abc123"),
            Some("refined"),
        )
        .is_none()
    );
}

#[test]
fn validate_update_input_valid_uri_invalid_status_returns_error() {
    let result = validate_assumption_update_input(
        Some("memory://assumptions/2026/02/assumption-abc123"),
        Some("bogus"),
    );
    assert!(result.is_some());
    assert!(result.expect("error").contains("Invalid status"));
}

#[test]
fn validate_read_input_null_uri_returns_error() {
    let result = validate_assumption_read_input(None);
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_read_input_empty_uri_returns_error() {
    let result = validate_assumption_read_input(Some(""));
    assert!(result.is_some());
    assert!(result.expect("error").contains("URI is required"));
}

#[test]
fn validate_read_input_valid_uri_returns_null() {
    assert!(
        validate_assumption_read_input(Some("memory://assumptions/2026/02/assumption-abc123"))
            .is_none()
    );
}

#[test]
fn validate_status_active_returns_null() {
    assert!(validate_assumption_status("active").is_none());
}

#[test]
fn validate_status_validated_returns_null() {
    assert!(validate_assumption_status("validated").is_none());
}

#[test]
fn validate_status_invalidated_returns_null() {
    assert!(validate_assumption_status("invalidated").is_none());
}

#[test]
fn validate_status_refined_returns_null() {
    assert!(validate_assumption_status("refined").is_none());
}

#[test]
fn validate_status_active_mixed_case_returns_null() {
    assert!(validate_assumption_status("Active").is_none());
}

#[test]
fn validate_status_validated_upper_case_returns_null() {
    assert!(validate_assumption_status("VALIDATED").is_none());
}

#[test]
fn validate_status_invalid_status_returns_error() {
    let result = validate_assumption_status("bogus");
    assert!(result.is_some());
    assert!(result.expect("error").contains("Invalid status"));
}

#[test]
fn validate_status_invalid_status_error_message_contains_all_valid_values() {
    let result = validate_assumption_status("bogus").expect("error");
    assert!(result.contains("active"));
    assert!(result.contains("validated"));
    assert!(result.contains("invalidated"));
    assert!(result.contains("refined"));
}
