use crate::error::Result;
use crate::mcp::stdio_pool::{ManagedServer, StdioPool};
use rmcp::model::{GetPromptResult, Prompt};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{RwLock as StdRwLock, RwLockReadGuard};

pub(super) fn all_gateway_prompts(pool: &StdioPool) -> Vec<(String, Vec<Prompt>)> {
    lock_prompts(&pool.gateway_prompts).clone()
}

pub(super) async fn get_prompt(
    pool: &StdioPool,
    server_name: &str,
    name: &str,
    arguments: Option<serde_json::Map<String, Value>>,
) -> Result<GetPromptResult> {
    let prompt_name = name.to_string();
    pool.with_restart(server_name, move |session| {
        let prompt_name = prompt_name.clone();
        let arguments = arguments.clone();
        Box::pin(async move { session.get_prompt(&prompt_name, arguments).await })
    })
    .await
}

pub(super) fn collect_gateway_prompts(
    servers: &HashMap<String, ManagedServer>,
) -> Vec<(String, Vec<Prompt>)> {
    let mut gateway_prompts = Vec::new();
    for (server_name, server) in servers {
        let Some(session) = &server.session else {
            continue;
        };
        let mut prompts = session.prompts().iter().cloned().collect::<Vec<_>>();
        prompts.sort_by(|left, right| left.name.cmp(&right.name));
        gateway_prompts.push((server_name.clone(), prompts));
    }
    gateway_prompts.sort_by(|(left, _), (right, _)| left.cmp(right));
    gateway_prompts
}

fn lock_prompts(
    prompts: &StdRwLock<Vec<(String, Vec<Prompt>)>>,
) -> RwLockReadGuard<'_, Vec<(String, Vec<Prompt>)>> {
    prompts.read().unwrap_or_else(|error| error.into_inner())
}
