use sacp::{ConnectionTo, schema as sacps};
use tokio::time::Duration;

use super::{
    convert::bridge,
    types::{AgentError, InitInfo, Result},
};

const INIT_TIMEOUT_SECONDS: u64 = 30;

fn prompt_meta() -> sacps::Meta {
    let caps = sacps::PromptCapabilities::new()
        .image(true)
        .audio(true)
        .embedded_context(true);

    let mut meta = sacps::Meta::new();
    meta.insert(
        "prompt".into(),
        serde_json::to_value(caps).expect("PromptCapabilities serializes"),
    );
    meta
}

pub(super) async fn initialize(conn: &ConnectionTo<sacp::Agent>) -> Result<InitInfo> {
    let request = sacps::InitializeRequest::new(sacps::ProtocolVersion::V1)
        .client_info(sacps::Implementation::new(
            "maenifold",
            env!("CARGO_PKG_VERSION"),
        ))
        .client_capabilities(
            sacps::ClientCapabilities::new()
                .fs(
                    sacps::FileSystemCapabilities::new()
                        .read_text_file(true)
                        .write_text_file(true),
                )
                .terminal(false)
                .meta(prompt_meta()),
        );

    let response = tokio::time::timeout(
        Duration::from_secs(INIT_TIMEOUT_SECONDS),
        conn.send_request(request).block_task(),
    )
    .await
    .map_err(|_| AgentError::Timeout {
        message: format!("ACP initialize timed out after {INIT_TIMEOUT_SECONDS}s"),
    })?
    .map_err(|error| AgentError::Acp(error.to_string()))?;

    if response.protocol_version != sacps::ProtocolVersion::V1 {
        return Err(AgentError::Acp(format!(
            "ACP protocol mismatch: requested {}, got {}",
            sacps::ProtocolVersion::V1,
            response.protocol_version
        )));
    }

    Ok(InitInfo {
        agent_info: response.agent_info.map(bridge).transpose()?,
        capabilities: bridge(response.agent_capabilities)?,
        auth_methods: response
            .auth_methods
            .into_iter()
            .map(bridge)
            .collect::<Result<Vec<_>>>()?,
    })
}
