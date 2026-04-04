use crate::endophasia::channel;
use axum::response::sse::{Event, Sse};
use futures_util::stream::{self, Stream};
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
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
            tracing::warn!("endophasia SSE serialization failed: {error}");
            Event::default()
                .event("serialization-error")
                .json_data(json!({
                    "message": "endophasia stream serialization failed"
                }))
                .unwrap_or_else(|fallback_error| {
                    tracing::warn!(
                        "endophasia SSE fallback serialization failed: {fallback_error}"
                    );
                    Event::default().event("serialization-error")
                })
        }
    }
}

pub async fn endophasia_stream() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = channel::subscribe();
    let stream = stream::unfold(receiver, |mut receiver| async move {
        let event = match receiver.recv().await {
            Ok(concept_event) => safe_json_event(None, &concept_event),
            Err(RecvError::Lagged(skipped)) => {
                safe_json_event(Some("lagged"), &json!({ "skipped": skipped }))
            }
            Err(RecvError::Closed) => return None,
        };

        Some((Ok(event), receiver))
    });

    Sse::new(stream)
}
