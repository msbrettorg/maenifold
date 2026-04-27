//! Async dispatch orchestration for Teams matches.
//!
//! Wraps a user-supplied async dispatch function with timeout enforcement and
//! parses the agent's textual output into reply / summary parts using the
//! `---SUMMARY---` protocol (see `agency-cowork/skills/teams/scripts/monitor/
//! message_handler.py` L1561-1581 / `_split_reply_summary` L756-794).
//!
//! This module is purely orchestration — it does not perform HTTP, does not
//! touch the supervisor loop, and does not send replies. Wiring lives in
//! `supervisor.rs` (sub-dispatch C).
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use super::dispatch::TeamsDispatchRequest;

const SUMMARY_DELIMITER: &str = "---SUMMARY---";
const MAX_REPLY: usize = 30_000;
const MAX_SUMMARY: usize = 1_000;

/// Async dispatch function signature: takes a request, returns the agent's
/// raw stdout/output text on success, or a human-readable error on failure.
pub type DispatchFn = Arc<
    dyn Fn(TeamsDispatchRequest) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;

/// Per-supervisor dispatch context: shared across all in-flight matches.
#[derive(Clone)]
pub struct DispatchCtx {
    /// Current chatsvc bearer token (refreshed by background loop). The
    /// dispatcher reads `token.borrow().clone()` immediately before invoking
    /// the dispatch function so each call sees the freshest value.
    pub token: watch::Receiver<String>,
    /// MRI of the local Teams user, required to build chatsvc reply payloads.
    pub from_mri: String,
    /// Chatsvc region segment for reply URLs (e.g. `amer`, `emea`).
    pub chatsvc_region: String,
    /// Optional prefix prepended to the first reply chunk (e.g. `🤖 `).
    pub reply_prefix: String,
    /// Hard timeout applied around the dispatch future.
    pub dispatch_timeout: Duration,
}

/// Outcome of a single dispatch attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    /// Dispatch completed within timeout. `summary` is `Some` only when the
    /// agent emitted a `---SUMMARY---` delimiter.
    Success {
        reply: String,
        summary: Option<String>,
    },
    /// Dispatch did not complete within `ctx.dispatch_timeout`.
    Timeout,
    /// Dispatch function returned an error.
    Failed(String),
}

/// Orchestrate one dispatch: invoke `dispatch_fn`, enforce timeout, and parse
/// the result. Borrows `ctx` so callers retain ownership across many calls.
pub async fn run_dispatch(
    request: TeamsDispatchRequest,
    ctx: &DispatchCtx,
    dispatch_fn: &DispatchFn,
) -> DispatchOutcome {
    let fut = dispatch_fn(request);
    match tokio::time::timeout(ctx.dispatch_timeout, fut).await {
        Ok(Ok(output)) => {
            let (reply, summary) = split_reply_summary(&output);
            DispatchOutcome::Success { reply, summary }
        }
        Ok(Err(msg)) => DispatchOutcome::Failed(msg),
        Err(_elapsed) => DispatchOutcome::Timeout,
    }
}

/// Split agent output into `(reply, summary)` on the `---SUMMARY---` marker.
///
/// Ports `_split_reply_summary` from `agency-cowork/skills/teams/scripts/
/// monitor/message_handler.py` L756-794. Differences from the Python:
/// returns `Option<String>` for the summary when no delimiter is present
/// (callers decide whether to synthesize one) and never substitutes a
/// placeholder reply for empty output (callers handle that policy).
pub fn split_reply_summary(output: &str) -> (String, Option<String>) {
    let (mut reply, mut summary) = if let Some(idx) = output.find(SUMMARY_DELIMITER) {
        let head = output[..idx].trim().to_string();
        let tail = output[idx + SUMMARY_DELIMITER.len()..].trim().to_string();
        (head, Some(tail))
    } else {
        (output.trim().to_string(), None)
    };

    if reply.len() > MAX_REPLY {
        reply.truncate(MAX_REPLY.saturating_sub(3));
        reply.push_str("...");
    }
    if let Some(s) = summary.as_mut() {
        if s.len() > MAX_SUMMARY {
            s.truncate(MAX_SUMMARY.saturating_sub(3));
            s.push_str("...");
        }
    }

    (reply, summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::teams::dispatch::TeamsSessionMetadata;

    fn fake_request() -> TeamsDispatchRequest {
        TeamsDispatchRequest {
            agent: "copilot".to_string(),
            prompt: "hi".to_string(),
            timeout_seconds: 30,
            metadata: TeamsSessionMetadata {
                source: "teams".to_string(),
                conversation_id: "c".to_string(),
                message_id: "m".to_string(),
                sender_mri: "8:s".to_string(),
                sender_name: "S".to_string(),
                matched_keyword: "k".to_string(),
                reply_conversation_id: "c".to_string(),
            },
        }
    }

    fn ctx(timeout: Duration) -> DispatchCtx {
        let (_tx, rx) = watch::channel(String::new());
        DispatchCtx {
            token: rx,
            from_mri: "8:me".to_string(),
            chatsvc_region: "amer".to_string(),
            reply_prefix: String::new(),
            dispatch_timeout: timeout,
        }
    }

    #[test]
    fn split_no_delimiter_returns_full_reply_and_none_summary() {
        let (r, s) = split_reply_summary("hello world");
        assert_eq!(r, "hello world");
        assert!(s.is_none());
    }

    #[test]
    fn split_with_delimiter_separates_parts() {
        let (r, s) = split_reply_summary("body text\n---SUMMARY---\nthe summary");
        assert_eq!(r, "body text");
        assert_eq!(s.as_deref(), Some("the summary"));
    }

    #[test]
    fn split_caps_reply_at_30k() {
        let big = "x".repeat(40_000);
        let (r, _) = split_reply_summary(&big);
        assert_eq!(r.len(), MAX_REPLY);
        assert!(r.ends_with("..."));
    }

    #[test]
    fn split_caps_summary_at_1k() {
        let big = format!("body\n---SUMMARY---\n{}", "y".repeat(2_000));
        let (_, s) = split_reply_summary(&big);
        let s = s.expect("summary present");
        assert_eq!(s.len(), MAX_SUMMARY);
        assert!(s.ends_with("..."));
    }

    #[tokio::test]
    async fn run_dispatch_success_parses_summary() {
        let f: DispatchFn =
            Arc::new(|_req| Box::pin(async { Ok("reply body\n---SUMMARY---\ndone".to_string()) }));
        let outcome = run_dispatch(fake_request(), &ctx(Duration::from_secs(5)), &f).await;
        match outcome {
            DispatchOutcome::Success { reply, summary } => {
                assert_eq!(reply, "reply body");
                assert_eq!(summary.as_deref(), Some("done"));
            }
            other => panic!("expected Success, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn run_dispatch_propagates_failure() {
        let f: DispatchFn = Arc::new(|_req| Box::pin(async { Err("agent crashed".to_string()) }));
        let outcome = run_dispatch(fake_request(), &ctx(Duration::from_secs(5)), &f).await;
        assert_eq!(
            outcome,
            DispatchOutcome::Failed("agent crashed".to_string())
        );
    }

    #[tokio::test]
    async fn run_dispatch_returns_timeout_when_future_exceeds_budget() {
        let f: DispatchFn = Arc::new(|_req| {
            Box::pin(async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(String::new())
            })
        });
        let outcome = run_dispatch(fake_request(), &ctx(Duration::from_millis(20)), &f).await;
        assert_eq!(outcome, DispatchOutcome::Timeout);
    }
}
