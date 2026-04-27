use crate::acp_config::IntentResolverSettings;
use crate::mcp::domain_mapping;
use crate::mcp::gateway_catalog::{CommandTarget, GatewayCatalog};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use crate::mcp::gateway_dispatch::validate_dispatch_parameters;
use crate::mcp::gateway_errors::*;
use crate::mcp::gateway_gate::invoke_gate;
use crate::mcp::resource_routing::{ResourceRoutingRegistry, map_gateway_resource_error};
use crate::mcp::schema::gateway_tools::{
    GatewayExecuteParams, GatewayIndexParams, gateway_batch_tool, gateway_execute_tool,
    gateway_index_tool, gateway_intent_tool,
};
use crate::mcp::stdio_pool::StdioPool;
use crate::mcp::tool_utils::format_tool_error;
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use rmcp::model::{CallToolResult, Content, ReadResourceResult, Resource, Tool};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct GatewayState {
    pool: Arc<StdioPool>,
    resource_routing: Arc<ResourceRoutingRegistry>,
    pub(crate) catalog: GatewayCatalog,
    consolidated_tools: Vec<ConsolidatedServerTool>,
    hidden_tool_domains: HashMap<String, String>,
    batch_state: crate::mcp::gateway_batch_types::BatchState,
    intent_settings: IntentResolverSettings,
    workspace_root: PathBuf,
}

impl GatewayState {
    pub(crate) fn new(
        pool: Arc<StdioPool>,
        resource_routing: Arc<ResourceRoutingRegistry>,
        catalog: GatewayCatalog,
        consolidated_tools: Vec<ConsolidatedServerTool>,
        intent_settings: IntentResolverSettings,
        workspace_root: PathBuf,
    ) -> Self {
        let mut hidden_tool_domains = HashMap::new();
        for server in &consolidated_tools {
            hidden_tool_domains.insert(
                legacy_facade_name(&server.server_name),
                server.server_name.clone(),
            );
            for tool in &server.tool_catalog {
                hidden_tool_domains.insert(tool.name.to_string(), server.server_name.clone());
            }
        }
        Self {
            pool,
            resource_routing,
            catalog,
            consolidated_tools,
            hidden_tool_domains,
            batch_state: crate::mcp::gateway_batch_types::BatchState::new(),
            intent_settings,
            workspace_root,
        }
    }

    pub(crate) fn pool(&self) -> Arc<StdioPool> {
        Arc::clone(&self.pool)
    }

    #[allow(dead_code)]
    pub(crate) fn catalog(&self) -> &GatewayCatalog {
        &self.catalog
    }

    pub(crate) fn intent_settings(&self) -> &IntentResolverSettings {
        &self.intent_settings
    }

    pub(crate) fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub(crate) fn list_tools(&self) -> Vec<Tool> {
        let mut tools = vec![
            tool_from_definition(gateway_index_tool()),
            tool_from_definition(gateway_execute_tool()),
            tool_from_definition(gateway_batch_tool()),
        ];
        if intent_enabled(&self.intent_settings) {
            tools.push(tool_from_definition(gateway_intent_tool()));
        }
        tools
    }

    pub(crate) fn list_resources(&self) -> Vec<Resource> {
        self.resource_routing.gateway_resources().to_vec()
    }

    pub(crate) async fn read_resource(
        &self,
        uri: &str,
    ) -> std::result::Result<ReadResourceResult, rmcp::ErrorData> {
        let Some(server_name) = self.resource_routing.lookup(uri).map(str::to_string) else {
            return Err(rmcp::ErrorData::resource_not_found(
                format!("No gateway resource owner found for '{uri}'"),
                None,
            ));
        };
        self.pool
            .read_resource(&server_name, uri)
            .await
            .map_err(|err| map_gateway_resource_error(&server_name, uri, err))
    }

    pub(crate) async fn handle_tool_call(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> std::result::Result<Option<CallToolResult>, rmcp::ErrorData> {
        if tool_name == "GatewayIndex" {
            return self.handle_gateway_index(arguments).map(Some);
        }
        if tool_name == "GatewayExecute" {
            return self.handle_gateway_execute(arguments).await.map(Some);
        }
        if tool_name == "GatewayBatch" {
            return crate::mcp::gateway_batch::handle_gateway_batch_arguments(
                self,
                &self.batch_state,
                arguments,
            )
            .await
            .map(Some);
        }
        if tool_name == "GatewayIntent" {
            return crate::mcp::gateway_intent::handle_gateway_intent(self, arguments)
                .await
                .map(Some);
        }

        if let Some(domain) = domain_mapping::tool_domain(tool_name) {
            return Err(rmcp::ErrorData::invalid_params(
                hidden_tool_message(tool_name, domain),
                None,
            ));
        }
        if let Some(domain) = self.hidden_tool_domains.get(tool_name) {
            return Err(rmcp::ErrorData::invalid_params(
                hidden_tool_message(tool_name, domain),
                None,
            ));
        }

        Ok(None)
    }

    fn handle_gateway_index(
        &self,
        arguments: Value,
    ) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let args: GatewayIndexParams = serde_json::from_value(arguments)
            .map_err(|err| map_gateway_error("GatewayIndex", invalid_params(err)))?;
        let index = self
            .catalog
            .render_index(
                args.domain.as_deref(),
                args.command.as_deref(),
                args.include_schemas.unwrap_or(false),
                args.include_examples.unwrap_or(false),
            )
            .map_err(|err| map_gateway_error("GatewayIndex", err))?;
        Ok(CallToolResult::success(vec![Content::text(render_json(
            index,
        ))]))
    }

    pub(crate) async fn handle_gateway_execute(
        &self,
        arguments: Value,
    ) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let args: GatewayExecuteParams = serde_json::from_value(arguments)
            .map_err(|err| map_gateway_error("GatewayExecute", invalid_params(err)))?;
        let parameters = Value::Object(args.parameters);
        match self
            .catalog
            .resolve(&args.domain, &args.command)
            .map_err(|err| map_gateway_error("GatewayExecute", err))?
        {
            CommandTarget::Native { tool_name } => invoke_native(tool_name, parameters).await,
            CommandTarget::Gate { command } => invoke_gate(command, parameters).await,
            CommandTarget::Upstream {
                server_name,
                tool_name,
            } => {
                let server = self.consolidated_server(&server_name)?;
                validate_dispatch_parameters(server, &tool_name, &parameters)
                    .map_err(|err| map_gateway_error(&tool_name, err))?;
                match self
                    .pool
                    .call_tool(&server_name, &tool_name, parameters)
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(err) => {
                        log_gateway_error(Some(&server_name), &tool_name, &err);
                        Err(map_gateway_error(&tool_name, err))
                    }
                }
            }
        }
    }

    pub(crate) fn consolidated_server(
        &self,
        server_name: &str,
    ) -> std::result::Result<&ConsolidatedServerTool, rmcp::ErrorData> {
        self.consolidated_tools
            .iter()
            .find(|server| server.server_name == server_name)
            .ok_or_else(|| {
                rmcp::ErrorData::internal_error(
                    format!("Gateway catalog resolved missing server '{server_name}'"),
                    None,
                )
            })
    }
}

fn intent_enabled(settings: &IntentResolverSettings) -> bool {
    settings.enabled
        && settings
            .agent
            .as_deref()
            .is_some_and(|agent| !agent.trim().is_empty())
}

fn render_json(value: Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}

async fn invoke_native(
    tool_name: String,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let display_name = tool_name.clone();
    let result = tokio::task::spawn_blocking(move || {
        ToolRegistry::global().try_invoke(&tool_name, &parameters, OutputFormat::Json)
    })
    .await
    .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;
    match result {
        Ok(output) => {
            // T-240.5: surface domain failures as MCP errors so clients can route on isError.
            // The body remains byte-identical; only the is_error flag changes when the
            // embedded JSON envelope reports {"success": false, ...}.
            if envelope_indicates_failure(&output) {
                Ok(CallToolResult::error(vec![Content::text(output)]))
            } else {
                Ok(CallToolResult::success(vec![Content::text(output)]))
            }
        }
        Err(err) => Ok(CallToolResult::error(vec![Content::text(
            format_tool_error(&display_name, &err),
        )])),
    }
}

// T-240.5: detect domain-failure envelopes produced by `util::json::failure`.
fn envelope_indicates_failure(text: &str) -> bool {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.get("success").and_then(Value::as_bool).map(|b| !b))
        .unwrap_or(false)
}

fn tool_from_definition(definition: crate::mcp::McpToolDefinition) -> Tool {
    let schema = match definition.input_schema {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    Tool::new(definition.name, definition.description, Arc::new(schema))
}

#[cfg(test)]
mod envelope_tests {
    use super::envelope_indicates_failure;

    #[test]
    fn detects_domain_failure_envelope() {
        let body = r#"{"success":false,"data":null,"error":{"code":"X","message":"Y"}}"#;
        assert!(envelope_indicates_failure(body));
    }

    #[test]
    fn passes_success_envelope_through() {
        let body = r#"{"success":true,"data":{"foo":1},"error":null}"#;
        assert!(!envelope_indicates_failure(body));
    }

    #[test]
    fn ignores_non_envelope_text() {
        assert!(!envelope_indicates_failure("plain markdown"));
        assert!(!envelope_indicates_failure("{}"));
    }
}
