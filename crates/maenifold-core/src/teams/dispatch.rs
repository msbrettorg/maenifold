use super::MatchResult;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Parameters for dispatching an ACP session from a Teams match.
#[derive(Debug, Clone, Serialize)]
pub struct TeamsDispatchRequest {
    pub agent: String,
    pub prompt: String,
    pub metadata: TeamsSessionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeamsSessionMetadata {
    pub source: String,
    pub conversation_id: String,
    pub message_id: String,
    pub sender_name: String,
    pub matched_keyword: String,
}

impl TeamsDispatchRequest {
    /// Build a dispatch request from a match result and agent name.
    pub fn from_match(result: &MatchResult, agent: &str) -> Self {
        let prompt = format!(
            "Teams message from {} matched keyword '{}'.\n\nMessage content:\n{}\n\nConversation: {}\nTime: {}",
            result.message.sender_name,
            result.matched_keyword,
            result.message.content,
            result.message.conversation_id,
            result.message.compose_time,
        );

        Self {
            agent: agent.to_string(),
            prompt,
            metadata: TeamsSessionMetadata {
                source: "teams".to_string(),
                conversation_id: result.message.conversation_id.clone(),
                message_id: result.message.message_id.clone(),
                sender_name: result.message.sender_name.clone(),
                matched_keyword: result.matched_keyword.clone(),
            },
        }
    }

    /// Convert to an AcpExec-compatible payload.
    pub fn to_acp_payload(&self) -> Value {
        json!({
            "agent": self.agent,
            "prompt": self.prompt,
            "autoGrantPermissions": true,
            "mode": "prompt",
            "metadata": self.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{TeamsDispatchRequest, TeamsSessionMetadata};
    use crate::teams::{EventMessage, MatchResult};
    use std::time::Instant;

    #[test]
    fn from_match_builds_prompt_and_metadata() {
        let result = MatchResult {
            message: EventMessage {
                message_id: "message-123".to_string(),
                conversation_id: "conversation-456".to_string(),
                content: "hello from Teams".to_string(),
                sender_mri: "8:orgid:user".to_string(),
                sender_name: "Brett".to_string(),
                compose_time: "2026-04-25T12:00:00Z".to_string(),
                resource_type: "NewMessage".to_string(),
            },
            matched_keyword: "keyword".to_string(),
            timestamp: Instant::now(),
        };

        let request = TeamsDispatchRequest::from_match(&result, "copilot");

        assert_eq!(request.agent, "copilot");
        assert!(request.prompt.contains("Teams message from Brett matched keyword 'keyword'."));
        assert!(request.prompt.contains("Message content:\nhello from Teams"));
        assert_eq!(
            request.metadata,
            TeamsSessionMetadata {
                source: "teams".to_string(),
                conversation_id: "conversation-456".to_string(),
                message_id: "message-123".to_string(),
                sender_name: "Brett".to_string(),
                matched_keyword: "keyword".to_string(),
            }
        );
    }

    #[test]
    fn to_acp_payload_keeps_teams_metadata() {
        let request = TeamsDispatchRequest {
            agent: "copilot".to_string(),
            prompt: "prompt".to_string(),
            metadata: TeamsSessionMetadata {
                source: "teams".to_string(),
                conversation_id: "conversation-456".to_string(),
                message_id: "message-123".to_string(),
                sender_name: "Brett".to_string(),
                matched_keyword: "keyword".to_string(),
            },
        };

        let payload = request.to_acp_payload();

        assert_eq!(payload["agent"], "copilot");
        assert_eq!(payload["prompt"], "prompt");
        assert_eq!(payload["autoGrantPermissions"], true);
        assert_eq!(payload["mode"], "prompt");
        assert_eq!(payload["metadata"]["source"], "teams");
        assert_eq!(payload["metadata"]["message_id"], "message-123");
    }
}
