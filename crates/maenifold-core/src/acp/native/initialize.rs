use super::types::{AgentError, InitInfo, Result};
use agent_client_protocol::{self as acp, Agent as _};
use tokio::time::Duration;

const INIT_TIMEOUT_SECONDS: u64 = 30;

/// Build prompt capabilities as `_meta` for `ClientCapabilities`.
///
/// The Rust ACP SDK (0.11.x) does not yet expose a `prompt` field on
/// [`acp::ClientCapabilities`] — the TypeScript SDK already does.  We bridge
/// the gap via the `_meta` extensibility mechanism using the crate's own
/// [`acp::PromptCapabilities`] type so the wire format stays close to acp-ui.
fn prompt_meta() -> acp::Meta {
    let caps = acp::PromptCapabilities::new()
        .image(true)
        .audio(true)
        .embedded_context(true);

    let mut meta = acp::Meta::new();
    meta.insert(
        "prompt".into(),
        serde_json::to_value(caps).expect("PromptCapabilities serializes"),
    );
    meta
}

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
                .terminal(true)
                .meta(prompt_meta()),
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
