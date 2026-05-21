use std::sync::LazyLock;

use tokio::sync::broadcast;

use super::ConceptEvent;

static CHANNEL: LazyLock<broadcast::Sender<ConceptEvent>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

/// Returns the broadcast sender for publishing ConceptEvents.
pub fn sender() -> &'static broadcast::Sender<ConceptEvent> {
    &CHANNEL
}

/// Returns a new broadcast receiver for subscribing to ConceptEvents.
#[allow(dead_code)]
pub fn subscribe() -> broadcast::Receiver<ConceptEvent> {
    CHANNEL.subscribe()
}
