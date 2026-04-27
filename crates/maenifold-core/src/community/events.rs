use once_cell::sync::Lazy;
use tokio::sync::broadcast;

#[derive(Debug, Clone, PartialEq)]
pub enum CommunityEvent {
    Recomputed { community_count: usize, nmi: f64 },
}

static SENDER: Lazy<broadcast::Sender<CommunityEvent>> = Lazy::new(|| broadcast::channel(16).0);

pub fn sender() -> &'static broadcast::Sender<CommunityEvent> {
    &SENDER
}

pub fn subscribe() -> broadcast::Receiver<CommunityEvent> {
    SENDER.subscribe()
}
