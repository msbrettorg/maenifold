pub(crate) fn sanitized_resume_failure_kind(raw: &str) -> &'static str {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("does not advertise loadsession")
        || lower.contains("loadsession capability")
        || lower.contains("load_session unsupported")
        || lower.contains("session/load unsupported")
    {
        "unsupported"
    } else if lower.contains("not found")
        || lower.contains("not_found")
        || lower.contains("no such session")
        || lower.contains("unknown session")
    {
        "notFound"
    } else if lower.contains("timeout") {
        "timeout"
    } else if lower.contains("binary not found") || lower.contains("failed to start agent") {
        "startup"
    } else {
        "internal"
    }
}

#[cfg(test)]
mod tests {
    use super::sanitized_resume_failure_kind;

    #[test]
    fn load_session_unsupported_maps_to_sanitized_kind() {
        let raw = "agent 'alpha' does not advertise loadSession capability; cannot resume";
        assert_eq!(sanitized_resume_failure_kind(raw), "unsupported");
    }

    #[test]
    fn load_session_not_found_maps_to_sanitized_kind() {
        let raw = "session/load failed for session 'secret-id' on agent 'alpha': not found";
        assert_eq!(sanitized_resume_failure_kind(raw), "notFound");
    }

    #[test]
    fn load_session_timeout_maps_to_sanitized_kind() {
        let raw = "timeout: load_session exceeded 20s";
        assert_eq!(sanitized_resume_failure_kind(raw), "timeout");
    }
}
