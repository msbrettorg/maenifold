use jockey::acp::AcpPromptResult;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{future::Future, pin::Pin, sync::Arc};
use tracing::{debug, info, warn};

use crate::error::{MaenifoldError, Result};

use super::agent_session_resume::{PromptGate, ResumeState, prompt_result_is_failed_resume};

pub(crate) use super::agent_session_open::open_agent_session;

pub(crate) type AcpSessionCapturedCallback =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentResponse {
    pub text: String,
    pub error: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub model: Option<String>,
    pub complete_node_args: Option<Value>,
}

impl From<AcpPromptResult> for AgentResponse {
    fn from(result: AcpPromptResult) -> Self {
        let error = if result.ok {
            None
        } else if !result.output.trim().is_empty() {
            Some(result.output.clone())
        } else {
            result.error_code.clone()
        };

        Self {
            text: result.output,
            error,
            input_tokens: result.meta.get("inputTokens").and_then(Value::as_u64),
            output_tokens: result.meta.get("outputTokens").and_then(Value::as_u64),
            model: result
                .meta
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string),
            complete_node_args: result.meta.get("completeNodeArgs").cloned(),
        }
    }
}

pub(crate) struct AgentSession {
    pub(super) agent_name: String,
    pub(super) runtime_kind: String,
    pub(super) session_id: String,
    pub(super) real_acp_session_id: std::sync::Mutex<Option<String>>,
    pub(super) on_acp_session_captured: Option<AcpSessionCapturedCallback>,
    pub(super) cwd: String,
    pub(super) model: Option<String>,
    pub(super) thought_level: Option<String>,
    pub(super) role_mode: Option<String>,
    pub(super) config_options: Vec<(String, String)>,
    pub(super) output_schema: Option<Value>,
    pub(super) allowed_tools: Option<Vec<String>>,
    pub(super) fenced_json_fallback: bool,
    pub(super) auto_approve: bool,
    /// Optional ACP session ID to resume on the next prompt via `session/load`.
    pub(super) resume_state: ResumeState,
    pub(super) prompt_gate: PromptGate,
    pub(super) event_tx: jockey::EventSender,
    pub(super) cleanup_on_drop: AtomicBool,
}

impl AgentSession {
    pub(crate) fn remote_id(&self) -> String {
        match self.real_acp_session_id.lock() {
            Ok(guard) => guard.clone().unwrap_or_else(|| self.session_id.clone()),
            Err(_) => {
                warn!(
                    agent = %self.agent_name,
                    session_id = %self.session_id,
                    "real ACP session id lock was poisoned; using internal session id"
                );
                self.session_id.clone()
            }
        }
    }

    pub(crate) fn set_acp_session_captured_callback(
        &mut self,
        callback: AcpSessionCapturedCallback,
    ) {
        self.on_acp_session_captured = Some(callback);
    }

    pub(crate) fn keep_alive(&self) {
        self.cleanup_on_drop.store(false, Ordering::Release);
    }

    /// T-328.16 — Fire-and-forget SIGKILL of the underlying ACP child process.
    /// Used by the wall-clock-budget kill switch when `tokio::time::timeout`
    /// drop-cancel fails to propagate into a hung `session.prompt()` future.
    ///
    /// v2: routes through the SYNCHRONOUS `force_kill_session_sync` path
    /// because the previous async/channel variant queued a WorkerMsg::ForceKill
    /// that the worker actor could not process while its LocalSet was blocked
    /// inside the very `session.prompt()` future we're trying to abort.
    pub(crate) async fn force_kill_child(&self) {
        let _ = jockey::acp::force_kill_session_sync(
            &self.runtime_kind,
            &self.agent_name,
            Some(self.session_id.as_str()),
        );
    }

    pub(crate) async fn prompt(&self, prompt: &str) -> Result<AgentResponse> {
        let _prompt_guard = self.prompt_gate.lock().await;
        let prompt = self.prompt_with_fallback_contract(prompt);
        info!(
            agent = %self.agent_name,
            session_id = %self.session_id,
            prompt_len = prompt.len(),
            config_options_count = self.config_options.len(),
            model = ?self.model,
            thought_level = ?self.thought_level,
            "sending prompt to ACP agent"
        );
        let resume_id = self.resume_state.take_next()?;
        let result = jockey::acp::execute_runtime(
            &self.runtime_kind,
            &self.agent_name,
            &prompt,
            &[],
            &self.cwd,
            &self.event_tx,
            self.auto_approve,
            vec![],
            self.allowed_tools.clone(),
            self.output_schema.clone(),
            self.role_mode.clone(),
            self.config_options.clone(),
            None,
            &self.session_id,
            resume_id.clone(),
            None,
        )
        .await;
        if !result.ok && prompt_result_is_failed_resume(&result.meta) {
            if let Some(id) = resume_id.clone() {
                let restored = self.resume_state.restore_after_failed_resume(id)?;
                if !restored {
                    warn!(
                        agent = %self.agent_name,
                        session_id = %self.session_id,
                        "resume session id was already restored before failed resume handling"
                    );
                }
            }
        }
        info!(
            agent = %self.agent_name,
            session_id = %self.session_id,
            ok = result.ok,
            output_len = result.output.len(),
            "ACP agent responded"
        );
        if !result.ok {
            warn!(
                agent = %self.agent_name,
                session_id = %self.session_id,
                output_preview = %result.output.chars().take(200).collect::<String>(),
                "ACP agent prompt failed"
            );
        }

        if let Some(sid) = result.meta.get("sessionId").and_then(|v| v.as_str()) {
            let sid = sid.to_string();
            let changed = {
                let mut guard = self.real_acp_session_id.lock().map_err(|_| {
                    MaenifoldError::Message(
                        "internal error: real ACP session id lock was poisoned".to_string(),
                    )
                })?;
                let changed = guard.as_deref() != Some(sid.as_str());
                *guard = Some(sid.clone());
                changed
            };
            if changed && let Some(callback) = &self.on_acp_session_captured {
                callback(sid).await?;
            }
        }

        Ok(result.into())
    }

    fn prompt_with_fallback_contract(&self, prompt: &str) -> String {
        if !self.fenced_json_fallback {
            return prompt.to_string();
        }
        let Some(schema) = self.output_schema.as_ref() else {
            return prompt.to_string();
        };
        format!(
            "{prompt}\n\nYou MUST end your response with a single fenced ```json block whose content validates against this schema:\n{schema}"
        )
    }

    pub(crate) async fn close(self) -> Result<()> {
        info!(
            agent = %self.agent_name,
            session_id = %self.session_id,
            "closing pipeline agent session"
        );
        let reset_result = jockey::acp::reset_session(
            &self.runtime_kind,
            &self.agent_name,
            Some(self.session_id.as_str()),
        )
        .await;
        match reset_result {
            Ok(()) => {
                self.cleanup_on_drop.store(false, Ordering::Release);
                Ok(())
            }
            Err(error) => {
                // Re-arm drop cleanup as fallback if explicit close fails.
                self.cleanup_on_drop.store(true, Ordering::Release);
                Err(MaenifoldError::Message(error))
            }
        }
    }
}

impl Drop for AgentSession {
    fn drop(&mut self) {
        if !self.cleanup_on_drop.load(Ordering::Acquire) {
            return;
        }

        let runtime_kind = self.runtime_kind.clone();
        let agent_name = self.agent_name.clone();
        let session_id = self.session_id.clone();
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            warn!(
                agent = %agent_name,
                session_id = %session_id,
                "dropping agent session without runtime; skipping async reset"
            );
            return;
        };

        handle.spawn(async move {
            if let Err(error) =
                jockey::acp::reset_session(&runtime_kind, &agent_name, Some(&session_id)).await
            {
                debug!(
                    agent = %agent_name,
                    session_id = %session_id,
                    error = %error,
                    "agent session reset during drop failed"
                );
            }
        });
    }
}
