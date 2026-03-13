pub fn validate_assumption_append_input(
    assumption: Option<&str>,
    concepts: Option<&[String]>,
) -> Option<String> {
    // T-W1-5: RTM FR-17.9 - exported for parity-focused validation tests.
    if assumption.is_none_or(|value| value.trim().is_empty()) {
        // PARITY-LED-ERR: RTM NFR-1 — include action context matching C# parity
        return Some("ERROR: Assumption text is required for 'append' action.".to_string());
    }
    let Some(concepts) = concepts else {
        // PARITY-LED-ERR: RTM NFR-1 — include example hint matching C# parity
        return Some("ERROR: At least one [[WikiLink]] tag must be provided. Example: concepts: [\"dialogue\", \"workflow-dispatch\"]".to_string());
    };
    if concepts.is_empty() {
        // PARITY-LED-ERR: RTM NFR-1 — include example hint matching C# parity
        return Some("ERROR: At least one [[WikiLink]] tag must be provided. Example: concepts: [\"dialogue\", \"workflow-dispatch\"]".to_string());
    }
    if concepts
        .iter()
        .any(|concept| concept.contains("[[") || concept.contains("]]"))
    {
        // PARITY-LED-ERR: RTM NFR-1 — include usage hint matching C# parity
        return Some(
            "ERROR: Concept tags should not include brackets. Use 'dialogue' not '[[dialogue]]'."
                .to_string(),
        );
    }
    None
}

pub fn validate_assumption_update_input(uri: Option<&str>, status: Option<&str>) -> Option<String> {
    // T-W1-5: RTM FR-17.9 - mirror C# URI and status validation behavior.
    if uri.is_none_or(|value| value.trim().is_empty()) {
        // PARITY-LED-ERR: RTM NFR-1 — include action context matching C# parity
        return Some("ERROR: URI is required for 'update' action.".to_string());
    }
    if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
        return validate_assumption_status(status);
    }
    None
}

pub fn validate_assumption_read_input(uri: Option<&str>) -> Option<String> {
    // T-W1-5: RTM FR-17.9 - read requires only a non-empty URI.
    if uri.is_none_or(|value| value.trim().is_empty()) {
        // PARITY-LED-ERR: RTM NFR-1 — include action context matching C# parity
        return Some("ERROR: URI is required for 'read' action.".to_string());
    }
    None
}

pub fn validate_assumption_status(status: &str) -> Option<String> {
    // T-W1-5: RTM FR-17.9 - keep status enum handling case-insensitive.
    let status = status.trim();
    if ["active", "validated", "invalidated", "refined"]
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(status))
    {
        None
    } else {
        Some(
            "ERROR: Invalid status. Use one of: active, validated, invalidated, refined"
                .to_string(),
        )
    }
}

pub(crate) fn build_assumption_append_content(
    assumption: &str,
    validation_plan: Option<&str>,
    concepts: &[String],
) -> String {
    // T-0.3a: RTM FR-4, RTM-047
    let mut content = format!("# Assumption: {assumption}\n\n## Statement\n\n{assumption}\n\n");
    if let Some(validation_plan) = validation_plan.filter(|value| !value.trim().is_empty()) {
        content.push_str(&format!("## Validation Plan\n\n{validation_plan}\n\n"));
    }
    content.push_str("## Related Concepts\n\n");
    for concept in concepts {
        content.push_str(&format!("- [[{concept}]]\n"));
    }
    content
}
