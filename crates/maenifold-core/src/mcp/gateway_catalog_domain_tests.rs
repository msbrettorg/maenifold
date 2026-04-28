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
            name == "GetHelp" || tool_domain(name).is_some(),
            "{name} missing gateway domain"
        );
    }
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
}

#[tokio::test]
async fn gateway_active_surface_has_exact_tool_count_by_intent_mode() {
    let baseline = exposed_gateway_surface(IntentResolverSettings::default()).await;
    assert_eq!(
        baseline,
        vec![
            "GetHelp",
            "ReadMcpResource",
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
            "GatewayIndex",
            "GatewayExecute",
            "GatewayBatch",
            "GatewayIntent"
        ]
    );
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
        .filter(|tool| tool.name == "GetHelp" || tool.name == "ReadMcpResource")
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
