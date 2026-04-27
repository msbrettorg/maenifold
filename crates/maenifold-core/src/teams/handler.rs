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
    #[serde(default = "default_listen_scope")]
    pub listen_scope: String,
    #[serde(default)]
    pub monitored_conversations: Vec<String>,
    #[serde(default)]
    pub self_mri: Option<String>,
    #[serde(default = "default_reply_behavior")]
    pub reply_behavior: String,
    #[serde(default = "default_reply_prefix")]
    pub reply_prefix: String,
    #[serde(default = "default_chatsvc_region")]
    pub chatsvc_region: String,
    #[serde(default = "default_dispatch_timeout_minutes")]
    pub dispatch_timeout_minutes: u64,
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
            listen_scope: default_listen_scope(),
            monitored_conversations: Vec::new(),
            self_mri: None,
            reply_behavior: default_reply_behavior(),
            reply_prefix: default_reply_prefix(),
            chatsvc_region: default_chatsvc_region(),
            dispatch_timeout_minutes: default_dispatch_timeout_minutes(),
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
    pub prompt: String,
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

                Some((trimmed.to_string(), keyword_regex(trimmed)))
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

        if !sender_allowed(
            message,
            &self.config.sender_allow_list,
            &self.config.sender_deny_list,
        ) {
            return None;
        }

        if !is_authorized_sender(message, self.config.self_mri.as_deref()) {
            return None;
        }

        if !conversation_allowed(message, &self.config) {
            return None;
        }

        if self.in_cooldown(&message.conversation_id) {
            return None;
        }

        let text = strip_html(&message.content);
        if text.is_empty() || is_self_loop_reply(&text, &self.config.reply_prefix) {
            return None;
        }
        let matched_keyword = self
            .keyword_patterns
            .iter()
            .find_map(|(keyword, pattern)| {
                (pattern.is_match(&text) || mention_match(keyword, &text, &message.content))
                    .then(|| keyword.clone())
            })?;
        let prompt = extract_prompt(&text, &matched_keyword, &message.content);

        let timestamp = Instant::now();
        self.last_match_by_conversation
            .insert(message.conversation_id.clone(), timestamp);

        Some(MatchResult {
            message: message.clone(),
            matched_keyword,
            prompt,
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

fn default_listen_scope() -> String {
    "wildcard".to_string()
}

fn default_reply_behavior() -> String {
    "sourceConversation".to_string()
}

fn default_reply_prefix() -> String {
    "maenifold: ".to_string()
}

fn default_chatsvc_region() -> String {
    "amer".to_string()
}

fn default_dispatch_timeout_minutes() -> u64 {
    10
}

fn keyword_regex(keyword: &str) -> Regex {
    Regex::new(&format!(
        r"(?i)(^|[^[:alnum:]_]){}([^[:alnum:]_]|$)",
        regex::escape(keyword)
    ))
    .expect("MessageHandler keyword regex must compile after escaping")
}

fn mention_match(keyword: &str, text: &str, raw_html: &str) -> bool {
    let Some(bare) = keyword.strip_prefix('@') else {
        return false;
    };
    if raw_html.is_empty() || !keyword_regex(bare).is_match(text) {
        return false;
    }
    let at_tag = Regex::new(&format!(
        r"(?i)<at\b[^>]*>\s*{}\s*</at>",
        regex::escape(bare)
    ))
    .expect("MessageHandler mention regex must compile after escaping");
    at_tag.is_match(raw_html)
}

fn extract_prompt(text: &str, keyword: &str, raw_html: &str) -> String {
    if let Some(matched) = keyword_regex(keyword).find(text) {
        return text[matched.end()..].trim().to_string();
    }
    if keyword.starts_with('@') && mention_match(keyword, text, raw_html) {
        let bare = &keyword[1..];
        if let Some(matched) = keyword_regex(bare).find(text) {
            return text[matched.end()..].trim().to_string();
        }
    }
    text.trim().to_string()
}

fn is_self_loop_reply(text: &str, reply_prefix: &str) -> bool {
    let prefix = reply_prefix.trim_end();
    !prefix.is_empty() && text.trim_start().starts_with(prefix)
}

fn is_authorized_sender(message: &EventMessage, self_mri: Option<&str>) -> bool {
    let Some(self_mri) = self_mri else {
        return true;
    };
    if self_mri.trim().is_empty() {
        return true;
    }
    normalize_sender(&message.sender_mri) == normalize_sender(self_mri)
}

fn conversation_allowed(message: &EventMessage, config: &MessageHandlerConfig) -> bool {
    match config.listen_scope.as_str() {
        "wildcard" => true,
        "self" => message.conversation_id == "48:notes",
        "custom" => config
            .monitored_conversations
            .iter()
            .any(|conversation_id| {
                conversation_id == "*" || conversation_id == &message.conversation_id
            }),
        _ => false,
    }
}

fn sender_allowed(
    message: &EventMessage,
    allow_list: &Option<Vec<String>>,
    deny_list: &Option<Vec<String>>,
) -> bool {
    let sender_mri = normalize_sender(&message.sender_mri);

    if let Some(deny_list) = deny_list {
        if deny_list
            .iter()
            .map(|entry| normalize_sender(entry))
            .any(|entry| !entry.is_empty() && entry == sender_mri)
        {
            return false;
        }
    }

    match allow_list {
        Some(allow_list) if !allow_list.is_empty() => allow_list
            .iter()
            .map(|entry| normalize_sender(entry))
            .any(|entry| !entry.is_empty() && entry == sender_mri),
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
#[path = "handler_tests.rs"]
mod tests;
