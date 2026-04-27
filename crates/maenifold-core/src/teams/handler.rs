//! Teams message handler — gates inbound messages through sender/scope/
//! cooldown checks, keyword matching, and (for non-self-chat) prompt
//! injection scanning before producing a [`MatchResult`] for dispatch.
//!
//! Free helpers and the config struct live in
//! [`super::handler_helpers`]; the pattern table for prompt scanning lives
//! in [`super::prompt_guard_patterns`].

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use regex::Regex;

use super::EventMessage;
use super::handler_helpers::{
    MessageHandlerConfig, conversation_allowed, extract_prompt, is_authorized_sender,
    is_self_loop_reply, keyword_regex, mention_match, normalize_sender, sender_allowed,
    strip_html,
};
use super::prompt_guard;

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

        // Prompt-injection scan (T-238.51) — self-chat bypass mirrors
        // agency-cowork "user trusts themselves" model. See
        // `super::prompt_guard` module docs.
        if message.conversation_id != "48:notes" {
            let scan = prompt_guard::scan(&prompt);
            if scan.is_blocking() {
                tracing::warn!(
                    conversation_id = %message.conversation_id,
                    sender = %normalize_sender(&message.sender_mri),
                    severity = ?scan.max_severity,
                    patterns = %scan.pattern_names_csv(),
                    preview = %scan.preview(120),
                    "blocked external Teams message: prompt injection detected"
                );
                return None;
            }
        }

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

#[cfg(test)]
#[path = "handler_tests.rs"]
mod tests;
