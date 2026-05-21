use crate::workflow::pipeline::stream;
use axum::response::sse::{Event, Sse};
use futures_util::stream::{self as fut_stream, Stream};
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

fn safe_json_event<T>(event_name: Option<&str>, payload: &T) -> Event
where
    T: Serialize,
{
    let mut event = Event::default();
    if let Some(event_name) = event_name {
        event = event.event(event_name);
    }

    match event.json_data(payload) {
        Ok(event) => event,
        Err(error) => {
            tracing::warn!("pipeline SSE serialization failed: {error}");
            Event::default()
                .event("serialization-error")
                .json_data(json!({
                    "message": "pipeline stream serialization failed"
                }))
                .unwrap_or_else(|fallback_error| {
                    tracing::warn!("pipeline SSE fallback serialization failed: {fallback_error}");
                    Event::default().event("serialization-error")
                })
        }
    }
}

pub async fn pipeline_stream() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = stream::subscribe();
    let stream = fut_stream::unfold(receiver, |mut receiver| async move {
        let event = match receiver.recv().await {
            Ok(pipeline_event) => safe_json_event(None, &pipeline_event),
            Err(RecvError::Lagged(skipped)) => {
                safe_json_event(Some("lagged"), &json!({ "skipped": skipped }))
            }
            Err(RecvError::Closed) => return None,
        };

        Some((Ok(event), receiver))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}
