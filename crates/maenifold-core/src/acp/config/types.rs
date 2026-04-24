// T-01.16: server/agent entry types extracted from acp/config.rs to keep
// each module ≤300 LOC. No behavioural changes.

use std::collections::HashMap;
use std::time::Duration;

use agent_client_protocol as acp;
use serde::{Deserialize, Serialize};

use crate::api::mcp_discovery::DEFAULT_DISCOVERY_TIMEOUT;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum McpServerEntry {
    #[serde(rename = "stdio")]
    Stdio {
        name: String,
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        env: HashMap<String, String>,
        #[serde(default)]
        gateway: bool,
    },
    #[serde(rename = "http")]
    Http { name: String, url: String },
}

impl McpServerEntry {
    pub fn name(&self) -> &str {
        match self {
            McpServerEntry::Stdio { name, .. } | McpServerEntry::Http { name, .. } => name,
        }
    }

    #[allow(dead_code)]
    /// Returns true if this server should expose its tools through the HTTP gateway.
    pub fn is_gateway(&self) -> bool {
        match self {
            McpServerEntry::Stdio { gateway, .. } => *gateway,
            McpServerEntry::Http { .. } => false,
        }
    }

    pub fn to_acp(&self) -> acp::McpServer {
        match self {
            McpServerEntry::Stdio {
                name,
                command,
                args,
                env,
                ..
            } => acp::McpServer::Stdio(
                acp::McpServerStdio::new(name.clone(), command.clone())
                    .args(args.clone())
                    .env(
                        env.iter()
                            .map(|(name, value)| acp::EnvVariable::new(name.clone(), value.clone()))
                            .collect(),
                    ),
            ),
            McpServerEntry::Http { name, url } => {
                acp::McpServer::Http(acp::McpServerHttp::new(name.clone(), url.clone()))
            }
        }
    }

    pub fn discovery_timeout(&self) -> Duration {
        match self {
            McpServerEntry::Stdio { timeout, .. } => timeout
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_DISCOVERY_TIMEOUT),
            McpServerEntry::Http { .. } => DEFAULT_DISCOVERY_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub command: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
}

impl AgentEntry {
    pub fn command_parts(&self) -> (&str, Vec<&str>) {
        let mut parts = self.command.split_whitespace();
        let binary = parts.next().unwrap_or("");
        let args = parts.collect();
        (binary, args)
    }
}
