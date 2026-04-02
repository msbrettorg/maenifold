use super::types::{AgentError, InitInfo, Result};
use agent_client_protocol::{self as acp, Agent as _};
use tokio::time::Duration;

const INIT_TIMEOUT_SECONDS: u64 = 30;

pub(super) async fn initialize(conn: &acp::ClientSideConnection) -> Result<InitInfo> {
    let request = acp::InitializeRequest::new(acp::ProtocolVersion::V1)
        .client_info(acp::Implementation::new(
            "maenifold",
            env!("CARGO_PKG_VERSION"),
        ))
        .client_capabilities(
            acp::ClientCapabilities::new()
                .fs(acp::FileSystemCapabilities::new()
                    .read_text_file(true)
                    .write_text_file(true))
                .terminal(false),
        );

    let response = tokio::time::timeout(
        Duration::from_secs(INIT_TIMEOUT_SECONDS),
        conn.initialize(request),
    )
    .await
    .map_err(|_| AgentError::Timeout {
        message: format!("ACP initialize timed out after {INIT_TIMEOUT_SECONDS}s"),
    })?
    .map_err(AgentError::from)?;

    if response.protocol_version != acp::ProtocolVersion::V1 {
        return Err(AgentError::Acp(format!(
            "ACP protocol mismatch: requested {}, got {}",
            acp::ProtocolVersion::V1,
            response.protocol_version
        )));
    }

    Ok(InitInfo {
        agent_info: response.agent_info,
        capabilities: response.agent_capabilities,
        auth_methods: response.auth_methods,
    })
}
