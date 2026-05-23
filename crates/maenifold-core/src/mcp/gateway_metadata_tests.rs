use super::gateway_catalog::GatewayCatalog;
use super::gateway_consolidation::build_consolidated_tools;
use super::gateway_handler::GatewayState;
use super::resource_routing::ResourceRoutingRegistry;
use super::stdio_pool::StdioPool;
use super::tool_routing::ToolRoutingRegistry;
use super::{MaenifoldHandler, TransportKind};
use crate::acp_config::IntentResolverSettings;
use rmcp::ServerHandler;
use rmcp::model::Tool;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn upstream_domain_description_uses_initialize_metadata() {
    let (consolidated, passthrough) =
        build_consolidated_tools(vec![("billing".to_string(), vec![tool("usage_export")])])
            .expect("consolidated");
    let pool = StdioPool::start(
        Vec::<crate::mcp::gateway_session::GatewayServerConfig>::new(),
        std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
    )
    .await
    .expect("empty pool");
    let routing = ToolRoutingRegistry::build(&[], &pool).expect("routing");
    let mut descriptions = BTreeMap::new();
    descriptions.insert(
        "billing".to_string(),
        "Billing MCP server.\n\nInstructions: Use for cost exports.".to_string(),
    );

    let catalog = GatewayCatalog::build_with_upstream_descriptions(
        &consolidated,
        &passthrough,
        &routing,
        &descriptions,
    )
    .expect("catalog");
    let billing = catalog
        .domains()
        .into_iter()
        .find(|domain| domain.name == "billing")
        .expect("billing domain");

    assert_eq!(
        billing.description,
        "Billing MCP server.\n\nInstructions: Use for cost exports."
    );
}

#[tokio::test]
async fn upstream_domain_fallback_names_domain_and_gatewayindex() {
    let (consolidated, passthrough) =
        build_consolidated_tools(vec![("opaque".to_string(), vec![tool("lookup")])])
            .expect("consolidated");
    let pool = StdioPool::start(
        Vec::<crate::mcp::gateway_session::GatewayServerConfig>::new(),
        std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
    )
    .await
    .expect("empty pool");
    let routing = ToolRoutingRegistry::build(&[], &pool).expect("routing");
    let catalog = GatewayCatalog::build_with_upstream_descriptions(
        &consolidated,
        &passthrough,
        &routing,
        &BTreeMap::new(),
    )
    .expect("catalog");
    let opaque = catalog
        .domains()
        .into_iter()
        .find(|domain| domain.name == "opaque")
        .expect("opaque domain");

    assert!(opaque.description.contains("opaque"));
    assert!(opaque.description.contains("GatewayIndex"));
    assert!(!opaque.description.contains("Gateway MCP server commands"));
}

#[tokio::test]
async fn initialize_metadata_advertises_live_gateway_domains() {
    let gateway = Arc::new(
        gateway_state_with_catalog(
            GatewayCatalog::build(&[], &[], &empty_routing().await).expect("catalog"),
        )
        .await,
    );
    let mut handler = MaenifoldHandler::new(TransportKind::Http).expect("handler");
    handler.set_gateway(gateway);

    let info = handler.get_info();
    let description = info.server_info.description.expect("server description");
    let instructions = info.instructions.expect("instructions");

    assert!(description.contains("maenifold.memory"));
    assert!(description.contains("GatewayIndex"));
    // Per-domain bullets carry the domain's own self-description.
    assert!(description.contains("- maenifold.memory"));
    assert!(description.contains("Maenifold memory file read/write commands"));
    // Cold-start guidance points clients at GatewayIndex for schemas.
    assert!(instructions.contains("GatewayIndex with includeSchemas=true"));
    assert!(instructions.contains("ReadMcpResource only for concrete resource URIs"));
}

/// Each domain bullet must carry the upstream's own initialize description
/// verbatim — that is the point of pass-through. We snap a small,
/// representative upstream description into the bullet and assert it appears
/// in `serverInfo.description`.
#[tokio::test]
async fn server_description_passes_through_upstream_self_description() {
    let (consolidated, passthrough) =
        build_consolidated_tools(vec![("msx-mcp".to_string(), vec![tool("ask_msx")])])
            .expect("consolidated");
    let mut descriptions = BTreeMap::new();
    descriptions.insert(
        "msx-mcp".to_string(),
        "This is a mcp server for msx".to_string(),
    );
    let catalog = GatewayCatalog::build_with_upstream_descriptions(
        &consolidated,
        &passthrough,
        &empty_routing().await,
        &descriptions,
    )
    .expect("catalog");
    let gateway = Arc::new(gateway_state_with_catalog(catalog).await);
    let mut handler = MaenifoldHandler::new(TransportKind::Http).expect("handler");
    handler.set_gateway(gateway);

    let info = handler.get_info();
    let description = info.server_info.description.expect("server description");

    assert!(
        description.contains("- msx-mcp (1 commands): This is a mcp server for msx"),
        "expected bullet with verbatim upstream description, got: {description}"
    );
}

/// A chatty upstream (azure-mcp-server style — ~1.8KB at initialize) must not
/// blow out the serverInfo.description. We cap each domain to ~280 chars and
/// elide with `…`; the full text remains reachable via GatewayIndex /
/// `gateway://domains/{name}` resources.
#[tokio::test]
async fn server_description_truncates_long_upstream_descriptions() {
    let long_self_description = "X".repeat(2_000);
    let (consolidated, passthrough) =
        build_consolidated_tools(vec![("chatty".to_string(), vec![tool("noop")])])
            .expect("consolidated");
    let mut descriptions = BTreeMap::new();
    descriptions.insert("chatty".to_string(), long_self_description.clone());
    let catalog = GatewayCatalog::build_with_upstream_descriptions(
        &consolidated,
        &passthrough,
        &empty_routing().await,
        &descriptions,
    )
    .expect("catalog");
    let gateway = Arc::new(gateway_state_with_catalog(catalog).await);
    let mut handler = MaenifoldHandler::new(TransportKind::Http).expect("handler");
    handler.set_gateway(gateway);

    let info = handler.get_info();
    let description = info.server_info.description.expect("server description");

    assert!(
        description.contains("- chatty (1 commands)"),
        "missing chatty bullet header: {description}"
    );
    assert!(
        description.contains('…'),
        "expected ellipsis indicating truncation, got: {description}"
    );
    assert!(
        !description.contains(&long_self_description),
        "raw 2000-char description must be truncated before being placed in serverInfo.description"
    );
}

async fn empty_routing() -> ToolRoutingRegistry {
    let pool = StdioPool::start(
        Vec::<crate::mcp::gateway_session::GatewayServerConfig>::new(),
        std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
    )
    .await
    .expect("empty pool");
    ToolRoutingRegistry::build(&[], &pool).expect("routing")
}

async fn gateway_state_with_catalog(catalog: GatewayCatalog) -> GatewayState {
    let pool = Arc::new(
        StdioPool::start(
            Vec::<crate::mcp::gateway_session::GatewayServerConfig>::new(),
            std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
        )
        .await
        .expect("empty pool"),
    );
    let routing = ResourceRoutingRegistry::build(pool.as_ref()).expect("resource routing");
    GatewayState::new(
        pool,
        Arc::new(routing),
        catalog,
        Vec::new(),
        IntentResolverSettings::default(),
        PathBuf::from("."),
    )
}

fn tool(name: &str) -> Tool {
    Tool::new(
        name.to_string(),
        format!("{name} description"),
        Arc::new(Map::<String, Value>::new()),
    )
}
