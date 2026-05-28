// T-01.16: server/agent entry types extracted from acp/config.rs to keep
// each module ≤300 LOC. No behavioural changes.

use std::collections::HashMap;
use std::time::Duration;

use agent_client_protocol as acp;
use serde::{Deserialize, Deserializer, Serialize};

use crate::api::mcp_discovery::DEFAULT_DISCOVERY_TIMEOUT;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged, rename_all = "camelCase")]
pub enum McpServerEntry {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        env: HashMap<String, String>,
        #[serde(
            default,
            rename = "envFromFile",
            skip_serializing_if = "HashMap::is_empty"
        )]
        env_from_file: HashMap<String, String>,
        #[serde(
            default,
            rename = "envFromCommand",
            skip_serializing_if = "HashMap::is_empty"
        )]
        env_from_command: HashMap<String, EnvFromCommand>,
        #[serde(default)]
        gateway: bool,
    },
    Http {
        url: String,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        headers: HashMap<String, String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        auth: Option<HttpAuthConfig>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(default)]
        gateway: bool,
    },
}

impl McpServerEntry {
    #[allow(dead_code)]
    /// Returns true if this server should expose its tools through the HTTP gateway.
    pub fn is_gateway(&self) -> bool {
        match self {
            McpServerEntry::Stdio { gateway, .. } => *gateway,
            McpServerEntry::Http { gateway, .. } => *gateway,
        }
    }

    pub fn to_acp(&self, name: &str) -> acp::McpServer {
        match self {
            McpServerEntry::Stdio {
                command, args, env, ..
            } => acp::McpServer::Stdio(
                acp::McpServerStdio::new(name.to_string(), command.clone())
                    .args(args.clone())
                    .env(
                        env.iter()
                            .map(|(name, value)| acp::EnvVariable::new(name.clone(), value.clone()))
                            .collect(),
                    ),
            ),
            McpServerEntry::Http { url, .. } => acp::McpServer::Http(acp::McpServerHttp::new(
                name.to_string(),
                acp_http_url(name, url),
            )),
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

impl<'de> Deserialize<'de> for McpServerEntry {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawEntry {
            command: Option<String>,
            #[serde(default)]
            args: Vec<String>,
            #[serde(default)]
            timeout: Option<u64>,
            #[serde(default)]
            env: HashMap<String, String>,
            #[serde(default, rename = "envFromFile")]
            env_from_file: HashMap<String, String>,
            #[serde(default, rename = "envFromCommand")]
            env_from_command: HashMap<String, EnvFromCommand>,
            #[serde(default)]
            gateway: bool,
            url: Option<String>,
            #[serde(default)]
            headers: HashMap<String, String>,
            #[serde(default)]
            auth: Option<HttpAuthConfig>,
        }

        let raw = RawEntry::deserialize(deserializer)?;
        match (raw.command, raw.url) {
            (Some(command), None) => Ok(McpServerEntry::Stdio {
                command,
                args: raw.args,
                timeout: raw.timeout,
                env: raw.env,
                env_from_file: raw.env_from_file,
                env_from_command: raw.env_from_command,
                gateway: raw.gateway,
            }),
            (None, Some(url)) => Ok(McpServerEntry::Http {
                url,
                headers: raw.headers,
                auth: raw.auth,
                timeout: raw.timeout,
                gateway: raw.gateway,
            }),
            (Some(_), Some(_)) => Err(serde::de::Error::custom(
                "mcpServers entry must contain command or url, not both command and url",
            )),
            (None, None) => Err(serde::de::Error::custom(
                "mcpServers entry must contain command or url",
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HttpAuthConfig {
    AzureDefault,
}

fn acp_http_url(name: &str, url: &str) -> String {
    if name.eq_ignore_ascii_case("maenifold") {
        return ipv4_loopback_url(url);
    }
    url.to_string()
}

fn ipv4_loopback_url(url: &str) -> String {
    for prefix in ["http://localhost", "https://localhost"] {
        let Some(rest) = url.strip_prefix(prefix) else {
            continue;
        };
        if rest.is_empty() || rest.starts_with(':') || rest.starts_with('/') {
            return format!("{}127.0.0.1{}", prefix.trim_end_matches("localhost"), rest);
        }
    }
    url.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvFromCommand {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub command: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(
        default,
        rename = "envFromFile",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub env_from_file: HashMap<String, String>,
    #[serde(
        default,
        rename = "envFromCommand",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub env_from_command: HashMap<String, EnvFromCommand>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mcp_servers: HashMap<String, McpServerEntry>,
}

impl AgentEntry {
    pub fn command_parts(&self) -> (&str, Vec<&str>) {
        let mut parts = self.command.split_whitespace();
        let binary = parts.next().unwrap_or("");
        let args = parts.collect();
        (binary, args)
    }
}
