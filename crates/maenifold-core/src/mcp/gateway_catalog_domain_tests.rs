use super::domain_mapping::{TOOL_DOMAINS, gate_tool_safety, tool_domain};
use super::gateway_catalog::{CommandTarget, GatewayCatalog};
use super::gateway_consolidation::build_consolidated_tools;
use super::gateway_handler::GatewayState;
use super::resource_routing::ResourceRoutingRegistry;
use super::schema::mcp_tool_from_registry_name;
use super::stdio_pool::StdioPool;
use super::tool_routing::ToolRoutingRegistry;
use super::{MaenifoldHandler, TransportKind};
use crate::acp_config::IntentResolverSettings;
use crate::registry::ToolRegistry;
use rmcp::model::Tool;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn forward_mapping_targets_existing_registry_tools_by_domain() {
    let canonical: HashSet<_> = ToolRegistry::global()
        .canonical_names()
        .into_iter()
        .collect();
    let mut by_domain = BTreeMap::<&str, Vec<&str>>::new();
    for &(tool, domain) in TOOL_DOMAINS {
        by_domain.entry(domain).or_default().push(tool);
    }
    for domain in [
        "maenifold.memory",
        "maenifold.graph",
        "maenifold.workflow",
        "maenifold.thinking",
        "maenifold.system",
    ] {
        let tools = by_domain.get(domain).expect("domain should have tools");
        assert!(!tools.is_empty(), "{domain} should route at least one tool");
        for tool in tools {
            assert!(
                canonical.contains(tool),
                "{tool} in {domain} missing registry handler"
            );
        }
    }
}

#[test]
fn reverse_mapping_exhaustively_covers_canonical_native_tools() {
    for name in ToolRegistry::global().canonical_names() {
        assert!(
            is_top_level_gateway_tool(name) || tool_domain(name).is_some(),
            "{name} missing gateway domain"
        );
    }
}

fn is_top_level_gateway_tool(name: &str) -> bool {
    if name.starts_with("gate_") {
        return true;
    }
    matches!(
        name,
        "GetHelp"
            | "ReadMcpResource"
            | "DiscoverMcpTools"
            | "GatewayIndex"
            | "GatewayExecute"
            | "GatewayBatch"
            | "GatewayIntent"
    )
}

#[tokio::test]
async fn routed_command_schemas_match_native_mcp_schemas() {
    let catalog = native_catalog().await;
    for &(tool, domain) in TOOL_DOMAINS {
        let command = catalog
            .commands(domain)
            .expect("domain should exist")
            .into_iter()
            .find(|entry| entry.name == tool)
            .expect("command should be indexed");
        let native = mcp_tool_from_registry_name(tool).expect("native schema should exist");
        assert_eq!(command.schema, native.input_schema, "{tool} schema drifted");
        assert_eq!(
            catalog
                .resolve(domain, tool)
                .expect("command should resolve"),
            CommandTarget::Native {
                tool_name: tool.to_string()
            }
        );
    }
}

#[tokio::test]
async fn wrong_domain_command_error_suggests_correct_domain_and_available_commands() {
    let catalog = native_catalog().await;
    let error = catalog
        .resolve("maenifold.memory", "FindSimilarConcepts")
        .expect_err("command should not resolve in the wrong domain");
    let text = error.to_string();

    assert!(
        text.contains(
            "Unknown gateway command 'FindSimilarConcepts' for domain 'maenifold.memory'."
        )
    );
    assert!(text.contains("Did you mean: maenifold.graph/FindSimilarConcepts?"));
    assert!(text.contains("Available commands in 'maenifold.memory':"));
    assert!(text.contains("ReadMemory"));
}

#[tokio::test]
async fn get_help_is_not_routed_but_remains_in_gateway_active_surface() {
    let catalog = native_catalog().await;
    assert_eq!(tool_domain("GetHelp"), None);
    for domain in catalog.domains() {
        let commands = catalog.commands(&domain.name).expect("domain commands");
        assert!(!commands.iter().any(|command| command.name == "GetHelp"));
    }
    let handler = MaenifoldHandler::new(TransportKind::Http).expect("handler");
    let gateway_tools = empty_gateway_tools().await;
    let mut exposed = exposed_native_tools(&handler);
    exposed.extend(gateway_tools.into_iter().map(|tool| tool.name.to_string()));
    assert_eq!(
        exposed,
        vec![
            "GetHelp",
            "ReadMcpResource",
            "DiscoverMcpTools",
            "GatewayIndex",
            "GatewayExecute",
            "GatewayBatch"
        ]
    );
}

#[tokio::test]
async fn flat_native_tools_are_hidden_when_gateway_is_active() {
    let handler = MaenifoldHandler::new(TransportKind::Http).expect("handler");
    let mut exposed = exposed_native_tools(&handler);
    exposed.extend(
        empty_gateway_tools()
            .await
            .into_iter()
            .map(|tool| tool.name.to_string()),
    );
    assert!(!exposed.iter().any(|name| name == "ReadMemory"));
    assert!(!exposed.iter().any(|name| name == "WriteMemory"));
    assert!(exposed.iter().any(|name| name == "GetHelp"));
    assert!(exposed.iter().any(|name| name == "ReadMcpResource"));
    assert!(exposed.iter().any(|name| name == "DiscoverMcpTools"));
}

#[tokio::test]
async fn gateway_active_surface_has_exact_tool_count_by_intent_mode() {
    let baseline = exposed_gateway_surface(IntentResolverSettings::default()).await;
    assert_eq!(
        baseline,
        vec![
            "GetHelp",
            "ReadMcpResource",
            "DiscoverMcpTools",
            "GatewayIndex",
            "GatewayExecute",
            "GatewayBatch"
        ]
    );

    let mut settings = IntentResolverSettings::default();
    settings.enabled = true;
    settings.agent = Some("resolver".to_string());
    let with_intent = exposed_gateway_surface(settings).await;
    assert_eq!(
        with_intent,
        vec![
            "GetHelp",
            "ReadMcpResource",
            "DiscoverMcpTools",
            "GatewayIndex",
            "GatewayExecute",
            "GatewayBatch",
            "GatewayIntent"
        ]
    );
}

#[tokio::test]
async fn gateway_active_surface_has_canonical_unique_tool_names() {
    let mut settings = IntentResolverSettings::default();
    settings.enabled = true;
    settings.agent = Some("resolver".to_string());

    let names = exposed_gateway_surface(settings).await;
    let unique: HashSet<_> = names.iter().cloned().collect();
    let expected: HashSet<_> = [
        "GatewayIndex",
        "GatewayExecute",
        "GatewayBatch",
        "GatewayIntent",
        "DiscoverMcpTools",
        "ReadMcpResource",
        "GetHelp",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    assert_eq!(names.len(), 7);
    assert_eq!(unique.len(), 7);
    assert_eq!(unique, expected);
}

#[test]
fn gate_upstream_safety_uses_first_party_contract_when_named_gate() {
    assert_eq!(gate_tool_safety("status").to_string(), "read");
    assert_eq!(gate_tool_safety("history").to_string(), "read");
    assert_eq!(gate_tool_safety("task_body").to_string(), "read");
    assert_eq!(gate_tool_safety("sync").to_string(), "mutate");
    assert_eq!(gate_tool_safety("append").to_string(), "mutate");
    assert_eq!(gate_tool_safety("pass").to_string(), "mutate");
    assert_eq!(gate_tool_safety("fail").to_string(), "mutate");
    assert_eq!(gate_tool_safety("cancel").to_string(), "destructive");
}

#[tokio::test]
async fn gate_upstream_catalog_entries_are_annotated_when_available() {
    let gate_tools = vec![tool("status"), tool("append"), tool("cancel")];
    let (consolidated, passthrough) =
        build_consolidated_tools(vec![("gate".to_string(), gate_tools)]).expect("gate catalog");
    let pool = StdioPool::start(Vec::new()).await.expect("empty pool");
    let routing = ToolRoutingRegistry::build(&[], &pool).expect("routing");
    let catalog = GatewayCatalog::build(&consolidated, &passthrough, &routing).expect("catalog");
    let commands = catalog.commands("gate").expect("gate domain");
    assert!(
        commands
            .iter()
            .any(|command| command.name == "status" && command.safety_class == "read")
    );
    assert!(
        commands
            .iter()
            .any(|command| command.name == "append" && command.safety_class == "mutate")
    );
    assert!(
        commands
            .iter()
            .any(|command| command.name == "cancel" && command.safety_class == "destructive")
    );
}

async fn native_catalog() -> GatewayCatalog {
    let pool = StdioPool::start(Vec::new()).await.expect("empty pool");
    let routing = ToolRoutingRegistry::build(&[], &pool).expect("routing");
    GatewayCatalog::build(&[], &[], &routing).expect("native catalog")
}

async fn empty_gateway_tools() -> Vec<Tool> {
    gateway_tools(IntentResolverSettings::default()).await
}

async fn gateway_tools(settings: IntentResolverSettings) -> Vec<Tool> {
    let pool = Arc::new(StdioPool::start(Vec::new()).await.expect("empty pool"));
    let routing = ResourceRoutingRegistry::build(pool.as_ref()).expect("resource routing");
    let catalog = native_catalog().await;
    GatewayState::new(
        pool,
        Arc::new(routing),
        catalog,
        Vec::new(),
        settings,
        PathBuf::from("."),
    )
    .list_tools()
}

async fn exposed_gateway_surface(settings: IntentResolverSettings) -> Vec<String> {
    let handler = MaenifoldHandler::new(TransportKind::Http).expect("handler");
    let mut exposed = exposed_native_tools(&handler);
    exposed.extend(
        gateway_tools(settings)
            .await
            .into_iter()
            .map(|tool| tool.name.to_string()),
    );
    exposed
}

fn exposed_native_tools(handler: &MaenifoldHandler) -> Vec<String> {
    handler
        .tools
        .iter()
        .filter(|tool| {
            tool.name == "GetHelp"
                || tool.name == "ReadMcpResource"
                || tool.name == "DiscoverMcpTools"
        })
        .map(|tool| tool.name.to_string())
        .collect()
}

fn tool(name: &str) -> Tool {
    Tool::new(
        name.to_string(),
        format!("{name} description"),
        Arc::new(Map::<String, Value>::new()),
    )
}

#[cfg(test)]
mod gate_create_tests {
    use crate::mcp::gateway_gate;
    use crate::util::paths::slugify_title;
    use rmcp::model::CallToolResult;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    // Edge-case matrix for gate_Create optional/nullable fields (T-244.112 discipline):
    // - body: omitted => scaffold H2 sections; provided => persisted as-is after frontmatter
    // - sprint: omitted => derive from taskId sprint; provided => use explicit integer
    // - parent: omitted => no frontmatter key; provided => write parent: <id>
    // - priority: omitted => default P2; provided => accept P0..P4 only; reject everything else
    // - title: empty/whitespace => reject after trim; unsafe characters => slugify without path escape
    // - pipeline: builtin ids accepted; custom ids registered under assets/gates/ accepted
    // - root: missing repo root / missing todo directory => actionable validation errors

    #[tokio::test]
    async fn gate_create_command_is_listed_in_maenifold_gate_domain_catalog() {
        let catalog = super::native_catalog().await;
        let commands = catalog.commands("maenifold.gate").expect("gate domain");
        assert!(
            commands.iter().any(|command| command.name == "gate_Create"),
            "gate_Create missing from maenifold.gate catalog: {:?}",
            commands.iter().map(|command| &command.name).collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn gate_create_happy_path_writes_frontmatter_and_auto_syncs_status() {
        let root = gate_root();
        let title = "Create the gateway task";
        let result = invoke_create(
            root.path(),
            "T-99.1",
            title,
            "tdd",
            json!({
                "body": "## Problem\n\nNeed a gateway create command.\n",
            }),
        )
        .await;

        assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
        let rendered = parsed_json(&result);
        let expected_path = root.path().join(format!(
            "todo/T-99.1-{}.md",
            slugify_title(title)
        ));

        assert_eq!(rendered["taskId"], "T-99.1");
        assert_eq!(rendered["pipeline"], "tdd");
        assert_eq!(rendered["sprint"], 99);
        assert_eq!(
            rendered["path"].as_str().map(PathBuf::from),
            Some(expected_path.clone())
        );

        let markdown = fs::read_to_string(&expected_path).expect("created task file");
        let frontmatter = frontmatter(&markdown);
        assert_eq!(frontmatter["id"], "T-99.1");
        assert_eq!(frontmatter["title"], title);
        assert_eq!(frontmatter["pipeline"], "tdd");
        assert_eq!(frontmatter["priority"], "P2");
        assert_eq!(frontmatter["sprint"], 99);

        let status = gateway_gate::invoke_gate(
            "gate_Status".to_string(),
            json!({ "root": root.path().display().to_string(), "includeDone": true }),
        )
        .await
        .expect("status result");
        assert!(!status.is_error.unwrap_or(false), "{}", text(&status));
        let rows = parsed_json(&status).as_array().cloned().expect("status rows");
        assert!(
            rows.iter().any(|row| row["taskId"] == "T-99.1"),
            "gate_Status should surface newly-created task: {rows:?}"
        );
    }

    #[tokio::test]
    async fn gate_create_rejects_duplicate_task_ids_in_todo_and_done() {
        for location in ["todo", "done"] {
            let root = gate_root();
            write_existing_task(root.path(), location, "T-99.1", "existing duplicate", "tdd");

            let result = invoke_create(root.path(), "T-99.1", "duplicate", "tdd", json!({})).await;

            assert!(result.is_error.unwrap_or(false), "{location} duplicate should fail");
            let message = text(&result);
            assert!(message.contains("T-99.1"), "{message}");
            assert!(
                message.contains("already exists") || message.contains("duplicate"),
                "{message}"
            );
            assert!(message.contains(location), "{message}");
        }
    }

    #[tokio::test]
    async fn gate_create_rejects_invalid_task_id_inputs() {
        for invalid in ["foo", "T-1", "T-1.2.3", "", " T-1.2", "t-1.2"] {
            let root = gate_root();
            let result = invoke_create(root.path(), invalid, "invalid id", "tdd", json!({})).await;

            assert!(result.is_error.unwrap_or(false), "{invalid:?} should fail");
            let message = text(&result);
            assert!(message.contains("taskId") || message.contains("task id"), "{message}");
            assert!(message.contains("T-") || message.contains("regex"), "{message}");
        }
    }

    #[tokio::test]
    async fn gate_create_rejects_unknown_pipeline_and_invalid_priority_values() {
        let root = gate_root();
        let unknown = invoke_create(
            root.path(),
            "T-99.1",
            "unknown pipeline",
            "frobnicate",
            json!({}),
        )
        .await;
        assert!(unknown.is_error.unwrap_or(false));
        let pipeline_message = text(&unknown);
        assert!(pipeline_message.contains("frobnicate"), "{pipeline_message}");
        for expected in ["sdd", "tdd", "gsd", "lightweight"] {
            assert!(pipeline_message.contains(expected), "{pipeline_message}");
        }

        let invalid_priority = invoke_create(
            root.path(),
            "T-99.2",
            "bad priority",
            "tdd",
            json!({ "priority": "PX" }),
        )
        .await;
        assert!(invalid_priority.is_error.unwrap_or(false));
        let priority_message = text(&invalid_priority);
        assert!(priority_message.contains("priority"), "{priority_message}");
        assert!(priority_message.contains("P0") || priority_message.contains("P4"), "{priority_message}");
    }

    #[tokio::test]
    async fn gate_create_rejects_empty_and_whitespace_titles() {
        for title in ["", "   "] {
            let root = gate_root();
            let result = invoke_create(root.path(), "T-99.1", title, "tdd", json!({})).await;
            assert!(result.is_error.unwrap_or(false), "{title:?} should fail");
            let message = text(&result);
            assert!(message.contains("title"), "{message}");
            assert!(message.contains("empty") || message.contains("required"), "{message}");
        }
    }

    #[tokio::test]
    async fn gate_create_slugifies_unsafe_titles_without_path_escape() {
        for (index, title) in ["foo/bar", "../etc", "foo\u{0000}bar"].into_iter().enumerate() {
            let root = gate_root();
            let task_id = format!("T-99.{}", index + 1);
            let result = invoke_create(root.path(), &task_id, title, "tdd", json!({})).await;

            assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
            let rendered = parsed_json(&result);
            let expected = root
                .path()
                .join(format!("todo/{task_id}-{}.md", slugify_title(title)));
            let actual = PathBuf::from(
                rendered["path"]
                    .as_str()
                    .expect("returned file path should be a string"),
            );

            assert_eq!(actual, expected);
            assert_eq!(actual.parent(), Some(root.path().join("todo").as_path()));
            assert!(actual.starts_with(root.path().join("todo")));
            assert!(actual.exists(), "created path should exist: {}", actual.display());
        }
    }

    #[tokio::test]
    async fn gate_create_accepts_registered_custom_pipeline_and_respects_body_sprint_parent_and_priority_fields()
     {
        let root = gate_root();
        write_custom_pipeline(root.path(), "custom-create-pipeline");
        let body = "## Custom Body\n\nShip exactly this body.\n";

        for (index, priority) in ["P0", "P1", "P2", "P3", "P4"].into_iter().enumerate() {
            let task_id = format!("T-99.{}", index + 1);
            let result = invoke_create(
                root.path(),
                &task_id,
                "custom pipeline task",
                "custom-create-pipeline",
                json!({
                    "body": body,
                    "priority": priority,
                    "sprint": 501,
                    "parent": "T-99.0"
                }),
            )
            .await;

            assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
            let rendered = parsed_json(&result);
            assert_eq!(rendered["pipeline"], "custom-create-pipeline");
            assert_eq!(rendered["sprint"], 501);

            let markdown = fs::read_to_string(
                rendered["path"]
                    .as_str()
                    .expect("result path should be a string"),
            )
            .expect("created markdown");
            let yaml = frontmatter(&markdown);
            assert_eq!(yaml["pipeline"], "custom-create-pipeline");
            assert_eq!(yaml["priority"], priority);
            assert_eq!(yaml["sprint"], 501);
            assert_eq!(yaml["parent"], "T-99.0");
            assert!(
                markdown.contains(body),
                "provided body should be preserved verbatim: {markdown}"
            );
            assert!(!markdown.contains("## Plan"), "{markdown}");
        }
    }

    #[tokio::test]
    async fn gate_create_scaffolds_default_body_and_derives_default_sprint_and_priority() {
        let root = gate_root();
        let result = invoke_create(root.path(), "T-321.7", "scaffold me", "tdd", json!({})).await;

        assert!(!result.is_error.unwrap_or(false), "{}", text(&result));
        let rendered = parsed_json(&result);
        assert_eq!(rendered["sprint"], 321);

        let markdown = fs::read_to_string(
            rendered["path"]
                .as_str()
                .expect("result path should be a string"),
        )
        .expect("created markdown");
        let yaml = frontmatter(&markdown);
        assert_eq!(yaml["priority"], "P2");
        assert_eq!(yaml["sprint"], 321);
        assert!(yaml.get("parent").is_none(), "parent should be omitted");
        for heading in ["## Problem", "## Plan", "## Acceptance", "## References"] {
            assert!(markdown.contains(heading), "{markdown}");
        }
    }

    #[tokio::test]
    async fn gate_create_rejects_missing_root_and_missing_todo_folder() {
        let missing_root = tempfile::tempdir().expect("tempdir");
        let nonexistent = missing_root.path().join("does-not-exist");
        let missing_root_result =
            invoke_create(&nonexistent, "T-99.1", "missing root", "tdd", json!({})).await;
        assert!(missing_root_result.is_error.unwrap_or(false));
        let missing_root_message = text(&missing_root_result);
        assert!(
            missing_root_message.contains("No such file")
                || missing_root_message.contains("does not exist")
                || missing_root_message.contains("canonicalize"),
            "{missing_root_message}"
        );

        let root = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(root.path().join("done")).expect("done dir");
        let missing_todo_result =
            invoke_create(root.path(), "T-99.1", "missing todo", "tdd", json!({})).await;
        assert!(missing_todo_result.is_error.unwrap_or(false));
        let missing_todo_message = text(&missing_todo_result);
        assert!(missing_todo_message.contains("todo"), "{missing_todo_message}");
        assert!(
            missing_todo_message.contains("missing") || missing_todo_message.contains("required"),
            "{missing_todo_message}"
        );
    }

    async fn invoke_create(
        root: &Path,
        task_id: &str,
        title: &str,
        pipeline: &str,
        extra: serde_json::Value,
    ) -> CallToolResult {
        let mut parameters = serde_json::Map::from_iter([
            ("root".to_string(), json!(root.display().to_string())),
            ("taskId".to_string(), json!(task_id)),
            ("title".to_string(), json!(title)),
            ("pipeline".to_string(), json!(pipeline)),
        ]);
        if let Some(extra) = extra.as_object() {
            parameters.extend(extra.clone());
        }
        gateway_gate::invoke_gate("gate_Create".to_string(), serde_json::Value::Object(parameters))
            .await
            .expect("gate create should return CallToolResult")
    }

    fn gate_root() -> TempDir {
        let root = tempfile::tempdir().expect("temp root");
        fs::create_dir_all(root.path().join("todo")).expect("todo");
        fs::create_dir_all(root.path().join("done")).expect("done");
        fs::create_dir_all(root.path().join("assets/gates")).expect("assets/gates");
        root
    }

    fn write_existing_task(root: &Path, location: &str, task_id: &str, title: &str, pipeline: &str) {
        let path = root
            .join(location)
            .join(format!("{task_id}-{}.md", slugify_title(title)));
        fs::write(path, markdown(task_id, title, pipeline, Some("P2"), Some(99), None, "## Problem\n\nExisting task.\n"))
            .expect("write existing task");
    }

    fn write_custom_pipeline(root: &Path, pipeline: &str) {
        let definition = json!({
            "schemaVersion": 1,
            "id": pipeline,
            "name": "Custom Create Pipeline",
            "initial": "pending",
            "states": {
                "pending": { "on": { "pass": "done" } },
                "done": { "terminal": true }
            }
        });
        fs::write(
            root.join("assets/gates").join(format!("{pipeline}.json")),
            serde_json::to_string_pretty(&definition).expect("serialize definition"),
        )
        .expect("write custom pipeline");
    }

    fn markdown(
        task_id: &str,
        title: &str,
        pipeline: &str,
        priority: Option<&str>,
        sprint: Option<u64>,
        parent: Option<&str>,
        body: &str,
    ) -> String {
        let mut frontmatter = vec![
            "---".to_string(),
            format!("id: {task_id}"),
            format!("title: {title:?}"),
            format!("pipeline: {pipeline}"),
        ];
        if let Some(priority) = priority {
            frontmatter.push(format!("priority: {priority}"));
        }
        if let Some(sprint) = sprint {
            frontmatter.push(format!("sprint: {sprint}"));
        }
        if let Some(parent) = parent {
            frontmatter.push(format!("parent: {parent}"));
        }
        frontmatter.push("---".to_string());
        format!("{}\n\n{body}", frontmatter.join("\n"))
    }

    fn frontmatter(markdown: &str) -> serde_json::Value {
        let mut lines = markdown.lines();
        assert_eq!(lines.next(), Some("---"), "task should begin with YAML frontmatter");
        let mut yaml = Vec::new();
        for line in lines.by_ref() {
            if line == "---" {
                break;
            }
            yaml.push(line);
        }
        serde_yml::from_str::<serde_json::Value>(&yaml.join("\n")).expect("valid yaml frontmatter")
    }

    fn parsed_json(result: &CallToolResult) -> serde_json::Value {
        serde_json::from_str(&text(result)).expect("gateway response should be valid JSON")
    }

    fn text(result: &CallToolResult) -> String {
        result.content[0]
            .raw
            .as_text()
            .expect("text content")
            .text
            .clone()
    }
}
