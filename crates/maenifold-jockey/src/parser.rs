use crate::types::ParsedRouteInput;
use std::collections::HashSet;

pub(crate) fn parse_route_input(raw: &str) -> ParsedRouteInput {
    let mut out = ParsedRouteInput::default();
    let mut message_tokens = Vec::new();
    let mut seen_skills: HashSet<String> = HashSet::new();
    let mut seen_roles: HashSet<String> = HashSet::new();

    for token in raw.split_whitespace() {
        if let Some(skill_ref) = token.strip_prefix('#') {
            let name = strip_trailing_punct(skill_ref).trim();
            if !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                if seen_skills.insert(name.to_string()) {
                    out.skill_refs.push(name.to_string());
                }
                continue;
            }
        }
        let Some(rest) = token.strip_prefix('@') else {
            message_tokens.push(token.to_string());
            continue;
        };
        let mention = strip_trailing_punct(rest).trim();
        if mention.is_empty() {
            message_tokens.push(token.to_string());
            continue;
        }

        if let Some(role) = mention.strip_prefix("role:") {
            if let Some(clean_role) = sanitize_role_name(role) {
                let key = clean_role.to_ascii_lowercase();
                if seen_roles.insert(key) {
                    out.role_names.push(clean_role);
                }
                continue;
            }
        }
        if let Some(path) = mention.strip_prefix("file:") {
            let clean = path.trim();
            if !clean.is_empty() {
                out.file_refs.push(clean.to_string());
                continue;
            }
        }
        if let Some(path) = mention.strip_prefix("dir:") {
            let clean = path.trim().trim_end_matches('/');
            if !clean.is_empty() {
                out.dir_refs.push(clean.to_string());
                continue;
            }
        }
        if mention == "dir" {
            out.dir_refs.push(".".to_string());
            continue;
        }

        if mention.ends_with('/') {
            let clean = mention.trim_end_matches('/').trim();
            if !clean.is_empty() {
                out.dir_refs.push(clean.to_string());
                continue;
            }
        }
        if mention.ends_with("/**") {
            let clean = mention.trim();
            if !clean.is_empty() {
                out.dir_refs.push(clean.to_string());
                continue;
            }
        }

        if looks_like_path(mention) {
            out.file_refs.push(mention.to_string());
            continue;
        }

        if let Some(clean_role) = sanitize_role_name(mention) {
            let key = clean_role.to_ascii_lowercase();
            if seen_roles.insert(key) {
                out.role_names.push(clean_role);
            }
            continue;
        }

        message_tokens.push(token.to_string());
    }

    out.message = message_tokens.join(" ").trim().to_string();
    out
}

pub(crate) fn strip_trailing_punct(token: &str) -> &str {
    token.trim_end_matches(|c: char| matches!(c, ',' | ';' | '!' | '?'))
}

pub(crate) fn looks_like_path(raw: &str) -> bool {
    raw.contains('/')
        || raw.contains('\\')
        || raw.starts_with("./")
        || raw.starts_with("../")
        || raw.starts_with("~/")
}

pub(crate) fn sanitize_role_name(raw: &str) -> Option<String> {
    let clean = strip_trailing_punct(raw).trim();
    if clean.is_empty() {
        return None;
    }
    if clean
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Some(clean.to_string());
    }
    None
}
