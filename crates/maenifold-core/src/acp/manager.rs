use crate::error::{MaenifoldError, Result};
use agent_client_protocol::{self as acp, Agent as _};
use std::collections::HashMap;
use std::path::Path;
use std::process::ExitStatus;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::{Duration, timeout};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Debug, Clone)]
pub struct MaenifoldAcpClient {
    text_tx: UnboundedSender<String>,
}

impl MaenifoldAcpClient {
    #[must_use]
    pub fn new(text_tx: UnboundedSender<String>) -> Self {
        Self { text_tx }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for MaenifoldAcpClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        if let acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk { content, .. }) =
            args.update
        {
            if let acp::ContentBlock::Text(text_content) = content {
                let _ = self.text_tx.send(text_content.text);
            }
        }

        Ok(())
    }

    async fn write_text_file(
        &self,
        _args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn read_text_file(
        &self,
        _args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn create_terminal(
        &self,
        _args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn terminal_output(
        &self,
        _args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn release_terminal(
        &self,
        _args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn wait_for_terminal_exit(
        &self,
        _args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn kill_terminal(
        &self,
        _args: acp::KillTerminalRequest,
    ) -> acp::Result<acp::KillTerminalResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_method(&self, _args: acp::ExtRequest) -> acp::Result<acp::ExtResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(&self, _args: acp::ExtNotification) -> acp::Result<()> {
        Err(acp::Error::method_not_found())
    }
}

pub struct AcpManager {
    conn: acp::ClientSideConnection,
    session_id: String,
    text_rx: UnboundedReceiver<String>,
    // Keep the subprocess alive for the lifetime of the ACP connection.
    #[allow(dead_code)]
    child: Child,
}

impl AcpManager {
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: &Path,
    ) -> Result<Self> {
        let mut child = Command::new(command);
        child
            .args(args)
            .envs(env)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true);

        let mut child = child.spawn().map_err(|error| {
            MaenifoldError::Message(format!(
                "failed to spawn ACP subprocess '{command}': {error}"
            ))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            MaenifoldError::Message("ACP subprocess stdin pipe was not available".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            MaenifoldError::Message("ACP subprocess stdout pipe was not available".to_string())
        })?;

        let (text_tx, text_rx) = mpsc::unbounded_channel();
        let client = MaenifoldAcpClient::new(text_tx);
        let (conn, handle_io) =
            acp::ClientSideConnection::new(client, stdin.compat_write(), stdout.compat(), |fut| {
                tokio::task::spawn_local(fut);
            });
        tokio::task::spawn_local(handle_io);

        let initialize_request = acp::InitializeRequest::new(acp::ProtocolVersion::V1).client_info(
            acp::Implementation::new("maenifold", env!("CARGO_PKG_VERSION")).title("Maenifold"),
        );

        match timeout(Duration::from_secs(10), conn.initialize(initialize_request)).await {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                let _ = child.start_kill();
                return Err(MaenifoldError::Message(format!(
                    "ACP initialize failed: {error}"
                )));
            }
            Err(_) => {
                let _ = child.start_kill();
                return Err(MaenifoldError::Message(
                    "ACP initialize timed out after 10 seconds".to_string(),
                ));
            }
        }

        let new_session = match timeout(
            Duration::from_secs(10),
            conn.new_session(acp::NewSessionRequest::new(cwd.to_path_buf())),
        )
        .await
        {
            Ok(Ok(session)) => session,
            Ok(Err(error)) => {
                let _ = child.start_kill();
                return Err(MaenifoldError::Message(format!(
                    "ACP new session failed: {error}"
                )));
            }
            Err(_) => {
                let _ = child.start_kill();
                return Err(MaenifoldError::Message(
                    "ACP new session timed out after 10 seconds".to_string(),
                ));
            }
        };

        Ok(Self {
            conn,
            session_id: new_session.session_id.to_string(),
            text_rx,
            child,
        })
    }

    pub async fn prompt(&self, messages: Vec<String>) -> Result<()> {
        let prompt = messages.into_iter().map(Into::into).collect();
        self.conn
            .prompt(acp::PromptRequest::new(self.session_id.clone(), prompt))
            .await
            .map_err(|error| MaenifoldError::Message(format!("ACP prompt failed: {error}")))?;
        Ok(())
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn take_text_stream(&mut self) -> UnboundedReceiver<String> {
        std::mem::replace(&mut self.text_rx, mpsc::unbounded_channel().1)
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child.try_wait().map_err(|error| {
            MaenifoldError::Message(format!("failed to inspect ACP subprocess state: {error}"))
        })
    }
}
