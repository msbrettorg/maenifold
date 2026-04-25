use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::EventMessage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageHandlerConfig {
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub sender_allow_list: Option<Vec<String>>,
    #[serde(default)]
    pub sender_deny_list: Option<Vec<String>>,
    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: u64,
}

impl Default for MessageHandlerConfig {
    fn default() -> Self {
        Self {
            keywords: Vec::new(),
            sender_allow_list: None,
            sender_deny_list: None,
            cooldown_seconds: default_cooldown_seconds(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub message: EventMessage,
    pub matched_keyword: String,
    pub timestamp: Instant,
}

#[derive(Debug)]
pub struct MessageHandler {
    config: MessageHandlerConfig,
    keyword_patterns: Vec<(String, Regex)>,
    last_match_by_conversation: HashMap<String, Instant>,
}

impl MessageHandler {
    pub fn new(config: MessageHandlerConfig) -> Self {
        let keyword_patterns = config
            .keywords
            .iter()
            .filter_map(|keyword| {
                let trimmed = keyword.trim();
                if trimmed.is_empty() {
                    return None;
                }

                let pattern = format!(r"(?i)\b{}\b", regex::escape(trimmed));
                let regex = Regex::new(&pattern)
                    .expect("MessageHandler keyword regex must compile after escaping");
                Some((trimmed.to_string(), regex))
            })
            .collect();

        Self {
            config,
            keyword_patterns,
            last_match_by_conversation: HashMap::new(),
        }
    }

    pub fn handle(&mut self, message: &EventMessage) -> Option<MatchResult> {
        self.prune_expired();

        if !sender_allowed(message, &self.config.sender_allow_list, &self.config.sender_deny_list) {
            return None;
        }

        if self.in_cooldown(&message.conversation_id) {
            return None;
        }

        let text = strip_html(&message.content);
        let matched_keyword = self
            .keyword_patterns
            .iter()
            .find_map(|(keyword, pattern)| pattern.is_match(&text).then(|| keyword.clone()))?;

        let timestamp = Instant::now();
        self.last_match_by_conversation
            .insert(message.conversation_id.clone(), timestamp);

        Some(MatchResult {
            message: message.clone(),
            matched_keyword,
            timestamp,
        })
    }

    fn in_cooldown(&self, conversation_id: &str) -> bool {
        self.last_match_by_conversation
            .get(conversation_id)
            .is_some_and(|last_match| {
                last_match.elapsed() < Duration::from_secs(self.config.cooldown_seconds)
            })
    }

    fn prune_expired(&mut self) {
        let cooldown = Duration::from_secs(self.config.cooldown_seconds);
        self.last_match_by_conversation
            .retain(|_, last_match| last_match.elapsed() < cooldown);
    }
}

pub fn strip_html(content: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let mut in_tag = false;
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '<' => {
                let is_tag_start = chars
                    .peek()
                    .is_some_and(|next| next.is_ascii_alphabetic() || matches!(next, '/' | '!'));
                if is_tag_start {
                    in_tag = true;
                    if !output.ends_with(' ') {
                        output.push(' ');
                    }
                } else {
                    output.push(ch);
                }
            }
            '>' => {
                in_tag = false;
            }
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }

    decode_html_entities(&output)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_cooldown_seconds() -> u64 {
    60
}

fn sender_allowed(
    message: &EventMessage,
    allow_list: &Option<Vec<String>>,
    deny_list: &Option<Vec<String>>,
) -> bool {
    let sender_mri = normalize_sender(&message.sender_mri);
    let sender_name = normalize_sender(&message.sender_name);

    if let Some(deny_list) = deny_list {
        if deny_list
            .iter()
            .map(|entry| normalize_sender(entry))
            .any(|entry| !entry.is_empty() && (entry == sender_mri || entry == sender_name))
        {
            return false;
        }
    }

    match allow_list {
        Some(allow_list) if !allow_list.is_empty() => allow_list
            .iter()
            .map(|entry| normalize_sender(entry))
            .any(|entry| !entry.is_empty() && (entry == sender_mri || entry == sender_name)),
        _ => true,
    }
}

fn normalize_sender(sender: &str) -> String {
    sender.trim().to_ascii_lowercase()
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MessageHandlerConfig {
        MessageHandlerConfig {
            keywords: vec!["help".into()],
            cooldown_seconds: 60,
            ..Default::default()
        }
    }

    fn message(conversation_id: &str, content: &str, sender_name: &str) -> EventMessage {
        EventMessage {
            message_id: format!("msg-{conversation_id}"),
            conversation_id: conversation_id.into(),
            content: content.into(),
            sender_mri: sender_name.into(),
            sender_name: sender_name.into(),
            compose_time: "2026-04-21T00:00:00Z".into(),
            resource_type: "NewMessage".into(),
        }
    }

    #[test]
    fn matches_keywords_case_insensitively() {
        let mut handler = MessageHandler::new(config());
        let result = handler.handle(&message("case-insensitive", "Help me", "alice"));

        assert_eq!(result.unwrap().matched_keyword, "help");
    }

    #[test]
    fn returns_none_when_no_keyword_matches() {
        let mut handler = MessageHandler::new(config());

        assert!(handler.handle(&message("no-match", "hello world", "alice")).is_none());
    }

    #[test]
    fn enforces_sender_allow_list() {
        let mut handler = MessageHandler::new(MessageHandlerConfig {
            sender_allow_list: Some(vec!["allowed-user".into()]),
            ..config()
        });

        assert!(handler
            .handle(&message("allowed", "help please", "allowed-user"))
            .is_some());
        assert!(handler
            .handle(&message("blocked", "help please", "unknown-user"))
            .is_none());
    }

    #[test]
    fn rejects_denied_sender_even_when_keyword_matches() {
        let mut handler = MessageHandler::new(MessageHandlerConfig {
            sender_deny_list: Some(vec!["blocked-user".into()]),
            ..config()
        });

        assert!(handler
            .handle(&message("deny-list", "help please", "blocked-user"))
            .is_none());
    }

    #[test]
    fn applies_cooldown_per_conversation() {
        let mut handler = MessageHandler::new(config());
        let conversation_id = "cooldown";
        let message = message(conversation_id, "help please", "alice");

        assert!(handler.handle(&message).is_some());
        assert!(handler.handle(&message).is_none());

        handler.last_match_by_conversation.insert(
            conversation_id.into(),
            Instant::now() - Duration::from_secs(61),
        );

        assert!(handler.handle(&message).is_some());
    }

    #[test]
    fn strips_html_before_matching() {
        let mut handler = MessageHandler::new(config());
        let message = message("html", "<b>help</b> me", "alice");

        assert_eq!(strip_html(&message.content), "help me");
        assert!(handler.handle(&message).is_some());
    }
}
