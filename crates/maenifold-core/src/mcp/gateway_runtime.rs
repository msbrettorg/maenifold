use crate::acp_config::IntentResolverSettings;
use crate::mcp::config_tools::ConfigMutationState;
use crate::mcp::domain_mapping;
use crate::mcp::gateway_config::invoke_config_command;
use crate::mcp::gateway_gate::invoke_gate;
use crate::mcp::gateway_registry::{CommandTarget, GatewayRegistry};
use crate::mcp::gateway_schema_validation::validate_parameters;
use crate::mcp::gateway_tasks::invoke_tasks;
use crate::mcp::resource_routing::{ResourceRoutingRegistry, map_gateway_resource_error};
use crate::mcp::schema::gateway_tools::{
    GatewayExecuteParams, GatewayIndexParams, gateway_batch_tool, gateway_execute_tool,
    gateway_index_tool, gateway_intent_tool,
};
use crate::mcp::stdio_pool::StdioPool;
use crate::memory::learn_requested;
use rmcp::RoleServer;
use rmcp::model::{CallToolResult, Content, ReadResourceResult, Resource, Tool};
use rmcp::service::Peer;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(test)]
#[path = "gateway_runtime_missing_required_tests.rs"]
mod missing_required_tests;
#[path = "gateway_runtime_support.rs"]
mod support;

#[derive(Clone)]
pub(crate) struct GatewayState {
    pool: Arc<StdioPool>,
    resource_routing: Arc<ResourceRoutingRegistry>,
    registry: GatewayRegistry,
    batch_state: crate::mcp::gateway_batch_types::BatchState,
    intent_settings: IntentResolverSettings,
    workspace_root: PathBuf,
    config_mutation_state: ConfigMutationState,
    startup_error: Option<String>,
}

impl GatewayState {
    pub(crate) fn new(
        pool: Arc<StdioPool>,
        resource_routing: Arc<ResourceRoutingRegistry>,
        registry: GatewayRegistry,
        intent_settings: IntentResolverSettings,
        workspace_root: PathBuf,
        config_mutation_state: ConfigMutationState,
    ) -> Self {
        Self {
            pool,
            resource_routing,
            registry,
            batch_state: crate::mcp::gateway_batch_types::BatchState::new(),
            intent_settings,
            workspace_root,
            config_mutation_state,
            startup_error: None,
        }
    }

    pub(crate) fn degraded(
        pool: Arc<StdioPool>,
        intent_settings: IntentResolverSettings,
        workspace_root: PathBuf,
        config_mutation_state: ConfigMutationState,
        startup_error: String,
    ) -> Result<Self, crate::error::MaenifoldError> {
        let registry = GatewayRegistry::build(Vec::new(), &pool.all_gateway_descriptions())?;
        let resource_routing =
            ResourceRoutingRegistry::build(pool.as_ref()).unwrap_or_else(|err| {
                tracing::warn!(err = %err, "gateway resource routing degraded to empty registry");
                ResourceRoutingRegistry::empty()
            });
        Ok(Self {
            pool,
            resource_routing: Arc::new(resource_routing),
            registry,
            batch_state: crate::mcp::gateway_batch_types::BatchState::new(),
            intent_settings,
            workspace_root,
            config_mutation_state,
            startup_error: Some(startup_error),
        })
    }

    pub(crate) fn registry(&self) -> &GatewayRegistry {
        &self.registry
    }

    pub(crate) fn batch_state(&self) -> &crate::mcp::gateway_batch_types::BatchState {
        &self.batch_state
    }

    pub(crate) fn pool(&self) -> Arc<StdioPool> {
        Arc::clone(&self.pool)
    }

    pub(crate) fn intent_settings(&self) -> &IntentResolverSettings {
        &self.intent_settings
    }

    pub(crate) fn config_mutation_state(&self) -> &ConfigMutationState {
        &self.config_mutation_state
    }

    pub(crate) fn startup_error(&self) -> Option<&str> {
        self.startup_error.as_deref()
    }

    #[allow(dead_code)]
    pub(crate) fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub(crate) fn list_tools(&self) -> Vec<Tool> {
        vec![
            support::tool_from_definition(gateway_index_tool()),
            support::tool_from_definition(gateway_execute_tool()),
            support::tool_from_definition(gateway_batch_tool()),
            support::tool_from_definition(gateway_intent_tool()),
        ]
    }

    pub(crate) fn list_resources(&self) -> Vec<Resource> {
        self.resource_routing.gateway_resources().to_vec()
    }

    pub(crate) fn owns_resource_uri(&self, uri: &str) -> bool {
        crate::mcp::resources::is_gateway_uri(uri) || self.resource_routing.owns(uri)
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
        client_peer: Option<Peer<RoleServer>>,
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
                self.batch_state(),
                arguments,
            )
            .await
            .map(Some);
        }
        if tool_name == "GatewayIntent" {
            return crate::mcp::gateway_intent::handle_gateway_intent(self, arguments, client_peer)
                .await
                .map(Some);
        }
        if let Some(domain) = domain_mapping::tool_domain(tool_name)
            .or_else(|| self.registry.hidden_tool_domain(tool_name))
        {
            return Err(rmcp::ErrorData::invalid_params(
                support::hidden_tool_message(tool_name, domain),
                None,
            ));
        }
        Ok(None)
    }

    fn handle_gateway_index(
        &self,
        arguments: Value,
    ) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let args: GatewayIndexParams = serde_json::from_value(arguments).map_err(|err| {
            rmcp::ErrorData::invalid_params(format!("Invalid GatewayIndex arguments: {err}"), None)
        })?;
        let index = self
            .registry
            .render_index(
                args.domain.as_deref(),
                args.command.as_deref(),
                args.include_commands.unwrap_or(false),
                args.include_schemas.unwrap_or(false),
                args.include_examples.unwrap_or(false),
            )
            .map_err(|err| support::map_gateway_error("GatewayIndex", err))?;
        let index = Self::attach_startup_error(index, self.startup_error());
        Ok(CallToolResult::success(vec![Content::text(
            support::render_json(index),
        )]))
    }

    fn attach_startup_error(mut value: Value, startup_error: Option<&str>) -> Value {
        if let (Some(error), Value::Object(map)) = (startup_error, &mut value) {
            map.insert(
                "gatewayStatus".to_string(),
                Value::String("degraded".to_string()),
            );
            map.insert(
                "gatewayStartupError".to_string(),
                Value::String(error.to_string()),
            );
        }
        value
    }

    pub(crate) async fn handle_gateway_execute(
        &self,
        arguments: Value,
    ) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let args: GatewayExecuteParams = serde_json::from_value(arguments).map_err(|err| {
            rmcp::ErrorData::invalid_params(
                format!("Invalid GatewayExecute arguments: {err}"),
                None,
            )
        })?;
        let parameters = Value::Object(args.parameters);
        let command_record = self
            .registry
            .command_record(&args.domain, &args.command)
            .map_err(|err| support::map_gateway_error("GatewayExecute", err))?;
        let command_ref = format!("{}/{}", args.domain, args.command);
        match command_record.target.clone() {
            CommandTarget::Native { tool_name } => {
                if !learn_requested(&parameters) {
                    validate_parameters(&command_ref, &parameters, &command_record.entry.schema)
                        .map_err(|err| rmcp::ErrorData::invalid_params(err.to_string(), None))?;
                }
                support::invoke_native(tool_name, parameters).await
            }
            CommandTarget::ConfigCommand { command } => {
                if !learn_requested(&parameters) {
                    validate_parameters(&command_ref, &parameters, &command_record.entry.schema)
                        .map_err(|err| rmcp::ErrorData::invalid_params(err.to_string(), None))?;
                }
                invoke_config_command(self.config_mutation_state(), command, parameters).await
            }
            CommandTarget::Gate { command } => invoke_gate(command, parameters).await,
            CommandTarget::Tasks { command } => invoke_tasks(command, parameters).await,
            CommandTarget::Upstream {
                server_name,
                tool_name,
            } => {
                self.registry
                    .validate_upstream(&server_name, &tool_name, &parameters)
                    .map_err(|err| support::map_gateway_error(&tool_name, err))?;
                match self
                    .pool
                    .call_tool(&server_name, &tool_name, parameters)
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(err) => {
                        support::log_gateway_error(Some(&server_name), &tool_name, &err);
                        Err(support::map_gateway_error(&tool_name, err))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) async fn invoke_native(
    tool_name: String,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    support::invoke_native(tool_name, parameters).await
}
