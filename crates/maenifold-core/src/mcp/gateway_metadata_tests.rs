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
    assert!(instructions.contains("GatewayIndex first"));
    assert!(instructions.contains("DiscoverMcpTools only for per-server"));
    assert!(instructions.contains("ReadMcpResource only for concrete resource URIs"));
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
