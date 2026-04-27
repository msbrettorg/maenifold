//! Interactive Teams monitor commands.
//!
//! Parser only — no execution logic. Ported from the Python reference
//! `agency-cowork/skills/teams/scripts/monitor/message_handler.py` lines 890-950.
//!
//! Consumed by the handler in a follow-up sub-dispatch; until then, allow
//! the unused-symbol lint so the crate continues to build under `-D warnings`.
#![allow(dead_code)]

use super::types::EventMessage;

/// Interactive Teams monitor commands.
///
/// Ref: `message_handler.py` L890-950.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamsCommand {
    Status,
    Stop,
    Resume,
    Cancel { arg: Option<String> },
    Dispatches { filter: DispatchFilter },
    List,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchFilter {
    Recent,
    Pending,
    Incomplete,
}

/// Outcome produced by the message handler when it inspects an incoming event.
#[derive(Debug)]
pub enum HandlerOutcome {
    None,
    Match(super::handler::MatchResult),
    Command(TeamsCommand, EventMessage),
}

/// Parse a command from message text. Returns `None` if the text is not a command.
///
/// Commands start with a `!` prefix and the verb is matched case-insensitively.
///
/// Ref: `message_handler.py` L893-920.
pub fn parse_command(text: &str) -> Option<TeamsCommand> {
    let trimmed = text.trim();
    if !trimmed.starts_with('!') {
        return None;
    }
    let without_prefix = &trimmed[1..];
    let mut parts = without_prefix.split_whitespace();
    let verb = parts.next()?.to_lowercase();

    match verb.as_str() {
        "status" => Some(TeamsCommand::Status),
        "stop" | "pause" => Some(TeamsCommand::Stop),
        "resume" | "start" => Some(TeamsCommand::Resume),
        "cancel" => Some(TeamsCommand::Cancel {
            arg: parts.next().map(String::from),
        }),
        "dispatches" => {
            let filter = match parts.next().map(|s| s.to_lowercase()).as_deref() {
                Some("pending") => DispatchFilter::Pending,
                Some("incomplete") => DispatchFilter::Incomplete,
                _ => DispatchFilter::Recent,
            };
            Some(TeamsCommand::Dispatches { filter })
        }
        "list" | "conversations" => Some(TeamsCommand::List),
        "help" | "?" => Some(TeamsCommand::Help),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status() {
        assert_eq!(parse_command("!status"), Some(TeamsCommand::Status));
    }

    #[test]
    fn parses_stop() {
        assert_eq!(parse_command("!stop"), Some(TeamsCommand::Stop));
        assert_eq!(parse_command("!pause"), Some(TeamsCommand::Stop));
    }

    #[test]
    fn parses_cancel_with_arg() {
        assert_eq!(
            parse_command("!cancel abc123"),
            Some(TeamsCommand::Cancel {
                arg: Some("abc123".to_string())
            })
        );
    }

    #[test]
    fn parses_dispatches_pending() {
        assert_eq!(
            parse_command("!dispatches pending"),
            Some(TeamsCommand::Dispatches {
                filter: DispatchFilter::Pending
            })
        );
    }

    #[test]
    fn non_command_returns_none() {
        assert_eq!(parse_command("hello world"), None);
    }

    #[test]
    fn verb_is_case_insensitive() {
        assert_eq!(parse_command("!RESUME"), Some(TeamsCommand::Resume));
    }

    #[test]
    fn unknown_verb_returns_none() {
        assert_eq!(parse_command("!unknown"), None);
    }
}
