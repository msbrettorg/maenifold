use std::sync::LazyLock;

use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PipelineEvent {
    AgentMessage {
        session_id: String,
        text: String,
    },
    ThoughtChunk {
        session_id: String,
        text: String,
    },
    ToolCall {
        session_id: String,
        tool_name: String,
    },
    ToolCallUpdate {
        session_id: String,
        tool_name: String,
    },
}

static CHANNEL: LazyLock<broadcast::Sender<PipelineEvent>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

pub fn sender() -> &'static broadcast::Sender<PipelineEvent> {
    &CHANNEL
}

pub fn subscribe() -> broadcast::Receiver<PipelineEvent> {
    CHANNEL.subscribe()
}
