use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum TrouterFrame {
    Connect,
    Event { name: String, args: Vec<Value> },
    Message(EventMessage),
    Ack(String),
    Unknown(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub content: String,
    pub sender_mri: String,
    pub sender_name: String,
    pub compose_time: String,
    pub resource_type: String,
}
