use super::*;
use rmcp::model::TasksCapability;
use rmcp::model::{
    CancelTaskParams, CancelTaskResult, CreateTaskResult, GetPromptRequestMethod,
    GetPromptRequestParams, GetPromptResult, GetTaskInfoParams, GetTaskPayloadResult,
    GetTaskResult, GetTaskResultParams, ListPromptsResult, ListTasksResult, PaginatedRequestParams,
    Prompt,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

fn split_namespaced_prompt_name(name: &str) -> std::result::Result<(&str, &str), ErrorData> {
    name.split_once(':').ok_or_else(|| {
        ErrorData::invalid_params("Prompt names must be namespaced as <server>:<prompt>", None)
    })
}

impl ServerHandler for MaenifoldHandler {
    fn get_info(&self) -> ServerInfo {
        let capabilities = match self.transport {
            TransportKind::Stdio => ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .enable_resources_subscribe()
                .enable_resources_list_changed()
                .enable_tasks_with(TasksCapability::server_default())
                .build(),
            TransportKind::Http => ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .enable_resources_subscribe()
                .enable_tasks_with(TasksCapability::server_default())
                .build(),
        };
        let domains = self
            .gateway
            .as_ref()
            .map(|gateway| gateway.catalog.domains())
            .unwrap_or_default();
        ServerInfo::new(capabilities)
            .with_server_info(
                Implementation::new("maenifold", env!("CARGO_PKG_VERSION"))
                    .with_description(gateway_metadata::server_description(&domains)),
            )
            .with_instructions(gateway_metadata::server_instructions(&domains))
    }
    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListToolsResult, rmcp::ErrorData>> + Send + '_
    {
        if let Some(error) = &self.gateway_startup_error {
            return std::future::ready(Err(rmcp::ErrorData::internal_error(
                format!("Gateway startup failed: {error}"),
                None,
            )));
        }
        let mut tools = if let Some(gateway) = &self.gateway {
            let mut tools: Vec<Tool> = self
                .tools
                .iter()
                .filter(|tool| tool.name == "GetHelp" || tool.name == "ReadMcpResource")
                .cloned()
                .collect();
            tools.extend(gateway.list_tools());
            tools
        } else {
            self.tools.clone()
        };
        tools.push(self::mcp_tool_calls::complete_node_tool());
        std::future::ready(Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        }))
    }
    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListPromptsResult, ErrorData>> + Send + '_ {
        let gateway = self.gateway.clone();
        async move {
            let prompts = gateway
                .map(|gateway| {
                    gateway
                        .pool()
                        .all_gateway_prompts()
                        .into_iter()
                        .flat_map(|(server_name, prompts)| {
                            prompts.into_iter().map(move |mut prompt: Prompt| {
                                prompt.name = format!("{server_name}:{}", prompt.name);
                                prompt
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Ok(ListPromptsResult {
                prompts,
                next_cursor: None,
                meta: None,
            })
        }
    }
    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<GetPromptResult, ErrorData>> + Send + '_ {
        let gateway = self.gateway.clone();
        async move {
            let Some(gateway) = gateway else {
                return Err(ErrorData::method_not_found::<GetPromptRequestMethod>());
            };
            let (server_name, name) = split_namespaced_prompt_name(&request.name)?;
            gateway
                .pool()
                .get_prompt(server_name, name, request.arguments)
                .await
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))
        }
    }
    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> std::result::Result<ListResourcesResult, rmcp::ErrorData> {
        let mut result =
            tokio::task::spawn_blocking(|| with_lock_retry(self::resources::handle_list_resources))
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?
                .map_err(self::resources::map_resource_error)?;
        if let Some(gateway) = &self.gateway {
            result.resources.extend(gateway.list_resources());
        }
        Ok(result)
    }
    fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListResourceTemplatesResult, rmcp::ErrorData>>
    + Send
    + '_ {
        std::future::ready(
            self::resources::handle_list_resource_templates()
                .map_err(self::resources::map_resource_error),
        )
    }
    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        let uri = request.uri.to_string();
        let gateway = self.gateway.clone();
        let gateway_diagnostics = self.gateway_diagnostics.clone();
        async move {
            if self::resources::is_gateway_uri(&uri) {
                return self::resources::handle_gateway_read_resource(&uri, gateway_diagnostics)
                    .await
                    .map_err(self::resources::map_resource_error);
            }
            if uri.starts_with("ui://")
                && let Some(gateway) = gateway
            {
                return gateway.read_resource(&uri).await;
            }
            tokio::task::spawn_blocking(move || {
                with_lock_retry(|| self::resources::handle_read_resource(&uri))
            })
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?
            .map_err(self::resources::map_resource_error)
        }
    }
    fn subscribe(
        &self,
        request: SubscribeRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<(), rmcp::ErrorData>> + Send + '_ {
        std::future::ready(self::resources::handle_subscribe(&request.uri, &context))
    }
    fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<(), rmcp::ErrorData>> + Send + '_ {
        std::future::ready(self::resources::handle_unsubscribe(&request.uri, &context))
    }
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<CallToolResult, rmcp::ErrorData>> + Send + '_
    {
        let tool_name = request.name.to_string();
        let arguments: Value = match request.arguments {
            Some(map) => Value::Object(map),
            None => json!({}),
        };
        let gateway = self.gateway.clone();
        async move {
            self::mcp_tool_calls::handle_tool_call(
                tool_name,
                arguments,
                gateway,
                Some(context.peer),
            )
            .await
        }
    }
    fn enqueue_task(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<CreateTaskResult, ErrorData>> + Send + '_ {
        self::task_hooks::enqueue_task(request)
    }
    fn list_tasks(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListTasksResult, ErrorData>> + Send + '_ {
        std::future::ready(self::task_hooks::list_tasks(request))
    }
    fn get_task_info(
        &self,
        request: GetTaskInfoParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<GetTaskResult, ErrorData>> + Send + '_ {
        std::future::ready(self::task_hooks::get_task_info(request))
    }
    fn get_task_result(
        &self,
        request: GetTaskResultParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<GetTaskPayloadResult, ErrorData>> + Send + '_
    {
        self::task_hooks::get_task_result(request)
    }
    fn cancel_task(
        &self,
        request: CancelTaskParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<CancelTaskResult, ErrorData>> + Send + '_ {
        std::future::ready(self::task_hooks::cancel_task(request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ErrorCode;

    #[test]
    fn get_info_advertises_prompts_and_prompt_names_require_namespace() {
        let stdio = MaenifoldHandler::new(TransportKind::Stdio).expect("stdio handler");
        let http = MaenifoldHandler::new(TransportKind::Http).expect("http handler");

        assert!(stdio.get_info().capabilities.prompts.is_some());
        assert!(http.get_info().capabilities.prompts.is_some());
        assert_eq!(
            split_namespaced_prompt_name("alpha:summarize").expect("namespaced prompt"),
            ("alpha", "summarize")
        );

        let error = split_namespaced_prompt_name("summarize").expect_err("missing namespace");
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert_eq!(
            error.message,
            "Prompt names must be namespaced as <server>:<prompt>"
        );
    }
}
