use crate::api::routes::ApiState;
use crate::mcp::pending_requests::{RespondError, UserPromptEvent};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures_util::stream::{self, Stream, StreamExt};
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

pub(crate) use crate::mcp::pending_requests::UserPromptResponse;

pub(crate) async fn stream_handler(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.user_prompt_registry.subscribe();
    let initial = state.user_prompt_registry.pending_events().await;
    user_prompt_sse(initial, receiver)
}

pub(crate) async fn respond_handler(
    State(state): State<ApiState>,
    Json(request): Json<UserPromptResponse>,
) -> impl IntoResponse {
    match state.user_prompt_registry.respond_typed(request).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "ok": true }))).into_response(),
        // A UUID that is not currently pending is reported as 410 Gone whether it
        // was never registered, already answered, or expired.
        Err(RespondError::Gone) => (
            StatusCode::GONE,
            Json(json!({ "error": "user prompt request is no longer pending" })),
        )
            .into_response(),
        Err(RespondError::Invalid(errors)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "errors": errors })),
        )
            .into_response(),
        Err(RespondError::KindMismatch) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "kind mismatch" })),
        )
            .into_response(),
    }
}

fn user_prompt_sse(
    initial: Vec<UserPromptEvent>,
    receiver: broadcast::Receiver<UserPromptEvent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let initial_stream = stream::iter(initial.into_iter().map(|event| Ok(safe_json_event(&event))));
    let live_stream = stream::unfold(receiver, |mut receiver| async move {
        let event = match receiver.recv().await {
            Ok(event) => safe_json_event(&event),
            Err(RecvError::Lagged(skipped)) => safe_json_event(&json!({ "skipped": skipped })),
            Err(RecvError::Closed) => return None,
        };
        Some((Ok(event), receiver))
    });

    Sse::new(initial_stream.chain(live_stream)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}

fn safe_json_event<T: serde::Serialize>(payload: &T) -> Event {
    Event::default()
        .event("user-prompt")
        .json_data(payload)
        .unwrap_or_else(|error| {
            tracing::warn!("user prompt SSE serialization failed: {error}");
            Event::default()
                .event("serialization-error")
                .json_data(json!({ "message": "user prompt stream serialization failed" }))
                .unwrap_or_else(|fallback_error| {
                    tracing::warn!(
                        "user prompt SSE fallback serialization failed: {fallback_error}"
                    );
                    Event::default().event("serialization-error")
                })
        })
}
