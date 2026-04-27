//! Free helpers and config types for [`super::handler::MessageHandler`].
//! Extracted from `handler.rs` so the dispatch logic stays under the LOC cap
//! and so `prompt_guard` can reuse `decode_html_entities` without
//! duplication.

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
    #[serde(default = "default_send_receipt")]
    pub send_receipt: bool,
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
            send_receipt: default_send_receipt(),
        }
    }
}

pub fn default_send_receipt() -> bool { true }

pub fn default_cooldown_seconds() -> u64 { 60 }
pub fn default_listen_scope() -> String { "wildcard".to_string() }
pub fn default_reply_behavior() -> String { "sourceConversation".to_string() }
pub fn default_reply_prefix() -> String { "maenifold: ".to_string() }
pub fn default_chatsvc_region() -> String { "amer".to_string() }
pub fn default_dispatch_timeout_minutes() -> u64 { 10 }

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

pub(crate) fn decode_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub fn keyword_regex(keyword: &str) -> Regex {
    Regex::new(&format!(
        r"(?i)(^|[^[:alnum:]_]){}([^[:alnum:]_]|$)",
        regex::escape(keyword)
    ))
    .expect("MessageHandler keyword regex must compile after escaping")
}

pub fn mention_match(keyword: &str, text: &str, raw_html: &str) -> bool {
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

pub fn extract_prompt(text: &str, keyword: &str, raw_html: &str) -> String {
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

pub fn is_self_loop_reply(text: &str, reply_prefix: &str) -> bool {
    let prefix = reply_prefix.trim_end();
    !prefix.is_empty() && text.trim_start().starts_with(prefix)
}

pub fn is_authorized_sender(message: &EventMessage, self_mri: Option<&str>) -> bool {
    let Some(self_mri) = self_mri else {
        return true;
    };
    if self_mri.trim().is_empty() {
        return true;
    }
    normalize_sender(&message.sender_mri) == normalize_sender(self_mri)
}

pub fn conversation_allowed(message: &EventMessage, config: &MessageHandlerConfig) -> bool {
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

pub fn sender_allowed(
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

pub fn normalize_sender(sender: &str) -> String {
    sender.trim().to_ascii_lowercase()
}
