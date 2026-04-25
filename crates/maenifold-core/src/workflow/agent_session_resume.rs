use std::sync::Mutex;

use crate::error::{MaenifoldError, Result};

#[derive(Debug)]
pub(crate) struct PromptGate {
    gate: tokio::sync::Mutex<()>,
}

impl PromptGate {
    pub(crate) fn new() -> Self {
        Self {
            gate: tokio::sync::Mutex::new(()),
        }
    }

    pub(crate) async fn lock(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.gate.lock().await
    }
}

#[derive(Debug)]
pub(crate) struct ResumeState {
    pending: Mutex<Option<String>>,
}

impl ResumeState {
    pub(crate) fn new(resume_session_id: Option<String>) -> Self {
        Self {
            pending: Mutex::new(resume_session_id),
        }
    }

    pub(crate) fn take_next(&self) -> Result<Option<String>> {
        let mut guard = self.pending.lock().map_err(|_| {
            MaenifoldError::Message(
                "internal error: resume session state lock was poisoned".to_string(),
            )
        })?;
        Ok(guard.take())
    }

    pub(crate) fn restore_after_failed_resume(&self, resume_session_id: String) -> Result<bool> {
        let mut guard = self.pending.lock().map_err(|_| {
            MaenifoldError::Message(
                "internal error: resume session state lock was poisoned".to_string(),
            )
        })?;
        if guard.is_none() {
            *guard = Some(resume_session_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub(crate) fn prompt_result_is_failed_resume(meta: &serde_json::Value) -> bool {
    meta.get("requestedResume")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && !meta
            .get("resumed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        && meta.get("resumeFailureKind").is_some()
}

#[cfg(test)]
mod tests {
    use super::{PromptGate, ResumeState, prompt_result_is_failed_resume};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[test]
    fn successful_resume_consumes_pending_resume_once() {
        let state = ResumeState::new(Some("session-123".to_string()));

        assert_eq!(
            state
                .take_next()
                .expect("resume state lock should be available"),
            Some("session-123".to_string())
        );
        assert_eq!(
            state
                .take_next()
                .expect("resume state lock should be available"),
            None
        );
    }

    #[test]
    fn failed_resume_restores_pending_resume_once() {
        let state = ResumeState::new(Some("session-123".to_string()));
        let taken = state
            .take_next()
            .expect("resume state lock should be available")
            .expect("resume id should be pending");

        assert!(
            state
                .restore_after_failed_resume(taken.clone())
                .expect("resume state lock should be available")
        );
        assert!(
            !state
                .restore_after_failed_resume(taken)
                .expect("resume state lock should be available")
        );
        assert_eq!(
            state
                .take_next()
                .expect("resume state lock should be available"),
            Some("session-123".to_string())
        );
        assert_eq!(
            state
                .take_next()
                .expect("resume state lock should be available"),
            None
        );
    }

    #[test]
    fn failed_resume_detection_requires_sanitized_failure_kind() {
        assert!(prompt_result_is_failed_resume(&json!({
            "requestedResume": true,
            "resumed": false,
            "resumeFailureKind": "notFound"
        })));
        assert!(!prompt_result_is_failed_resume(&json!({
            "requestedResume": true,
            "resumed": true
        })));
        assert!(!prompt_result_is_failed_resume(&json!({
            "requestedResume": true,
            "resumed": false
        })));
    }

    #[tokio::test]
    async fn prompt_gate_serializes_concurrent_attempts() {
        let gate = Arc::new(PromptGate::new());
        let first_guard = gate.lock().await;
        let (tx, mut rx) = mpsc::channel(1);
        let contender_gate = Arc::clone(&gate);
        let task = tokio::spawn(async move {
            let _second_guard = contender_gate.lock().await;
            tx.send(())
                .await
                .expect("test receiver should remain available");
        });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(rx.try_recv().is_err());
        drop(first_guard);
        tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("contender should acquire prompt gate after first guard is dropped")
            .expect("contender should send acquisition signal");
        task.await.expect("contender task should complete");
    }
}
