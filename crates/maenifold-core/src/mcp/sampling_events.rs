use axum::{
    extract::Path,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::{self as fut_stream, Stream};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{LazyLock, Mutex},
    time::Duration,
};
use tokio::sync::broadcast::{self, error::RecvError};

type SenderMap = HashMap<String, jockey::EventSender>;

static SENDERS: LazyLock<Mutex<SenderMap>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn sender_for(app_session_id: uuid::Uuid) -> jockey::EventSender {
    let key = app_session_id.to_string();
    let mut senders = SENDERS.lock().expect("sampling event sender map mutex");
    senders
        .entry(key)
        .or_insert_with(|| broadcast::channel::<jockey::JockeyEvent>(64).0)
        .clone()
}

pub(crate) fn unregister(app_session_id: uuid::Uuid) {
    if let Ok(mut senders) = SENDERS.lock() {
        senders.remove(&app_session_id.to_string());
    }
}

fn subscribe(app_session_id: &str) -> broadcast::Receiver<jockey::JockeyEvent> {
    let mut senders = SENDERS.lock().expect("sampling event sender map mutex");
    senders
        .entry(app_session_id.to_string())
        .or_insert_with(|| broadcast::channel::<jockey::JockeyEvent>(64).0)
        .subscribe()
}

fn safe_json_event<T>(event_name: Option<&str>, payload: &T) -> Event
where
    T: Serialize,
{
    let mut event = Event::default();
    if let Some(event_name) = event_name {
        event = event.event(event_name);
    }
    event.json_data(payload).unwrap_or_else(|error| {
        tracing::warn!("sampling SSE serialization failed: {error}");
        Event::default()
            .event("serialization-error")
            .json_data(json!({ "message": "sampling stream serialization failed" }))
            .unwrap_or_else(|fallback_error| {
                tracing::warn!("sampling SSE fallback serialization failed: {fallback_error}");
                Event::default().event("serialization-error")
            })
    })
}

fn event_app_session_id(event: &jockey::JockeyEvent) -> Option<&str> {
    match event {
        jockey::JockeyEvent::AcpDelta(value) | jockey::JockeyEvent::AcpStream(value) => {
            value.get("appSessionId").and_then(serde_json::Value::as_str)
        }
        _ => None,
    }
}

pub(crate) async fn sse_handler(
    Path(app_session_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = subscribe(&app_session_id);
    let stream =
        fut_stream::unfold((receiver, app_session_id), |(mut receiver, app_session_id)| async move {
            loop {
                match receiver.recv().await {
                    Ok(jockey_event) => {
                        if event_app_session_id(&jockey_event) == Some(app_session_id.as_str()) {
                            let event = safe_json_event(None, &jockey_event);
                            return Some((Ok(event), (receiver, app_session_id)));
                        }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        let event = safe_json_event(Some("lagged"), &json!({ "skipped": skipped }));
                        return Some((Ok(event), (receiver, app_session_id)));
                    }
                    Err(RecvError::Closed) => return None,
                };
            }
        });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)).text("ping"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast::error::TryRecvError;

    #[tokio::test]
    async fn test_sender_for_returns_same_sender_for_same_app_session_id() {
        let app_session_id = uuid::Uuid::new_v4();
        unregister(app_session_id);

        let sender_a = sender_for(app_session_id);
        let mut receiver = sender_a.subscribe();
        let sender_b = sender_for(app_session_id);
        let event = jockey::JockeyEvent::AcpDelta(json!({
            "appSessionId": app_session_id.to_string(),
            "delta": "hello",
        }));

        sender_b
            .send(event)
            .expect("same-channel receiver should keep send open");

        match receiver.recv().await.expect("receiver should see sender_b event") {
            jockey::JockeyEvent::AcpDelta(value) => {
                assert_eq!(value.get("delta").and_then(serde_json::Value::as_str), Some("hello"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
        unregister(app_session_id);
    }

    #[test]
    fn test_unregister_removes_sender_for_app_session_id() {
        let app_session_id = uuid::Uuid::new_v4();
        unregister(app_session_id);

        let old_sender = sender_for(app_session_id);
        let mut old_receiver = old_sender.subscribe();
        unregister(app_session_id);
        let new_sender = sender_for(app_session_id);
        let _new_receiver = new_sender.subscribe();

        new_sender
            .send(jockey::JockeyEvent::AcpDelta(json!({
                "appSessionId": app_session_id.to_string(),
                "delta": "new-channel",
            })))
            .expect("new sender has active receiver");

        assert!(matches!(old_receiver.try_recv(), Err(TryRecvError::Empty)));
        unregister(app_session_id);
    }

    #[test]
    fn test_event_app_session_id_filters_by_field() {
        let wanted = uuid::Uuid::new_v4().to_string();
        let other = uuid::Uuid::new_v4().to_string();
        let matching = jockey::JockeyEvent::AcpDelta(json!({
            "appSessionId": wanted,
            "delta": "match",
        }));
        let non_matching = jockey::JockeyEvent::AcpDelta(json!({
            "appSessionId": other,
            "delta": "skip",
        }));
        let missing = jockey::JockeyEvent::AcpDelta(json!({ "delta": "skip" }));

        assert!(event_app_session_id(&matching).is_some());
        assert_ne!(event_app_session_id(&matching), event_app_session_id(&non_matching));
        assert_eq!(event_app_session_id(&missing), None);
    }
}
