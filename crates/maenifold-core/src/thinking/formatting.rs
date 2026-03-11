use chrono::Utc;

// T-12.0: RTM NFR-1 - keep SequentialThinking output formatting isolated from session persistence.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_thought_section(
    thought_number: u64,
    total_thoughts: u64,
    needs_more_thoughts: bool,
    branch_id: Option<&str>,
    is_revision: bool,
    revises_thought: Option<u64>,
    response: &str,
    thoughts: Option<&str>,
) -> (String, String) {
    let display_total = if needs_more_thoughts && thought_number >= total_thoughts {
        thought_number + 1
    } else {
        total_thoughts
    };
    let agent_id = std::env::var("AGENT_ID").unwrap_or_else(|_| "agent".to_string());
    let suffix = if let Some(branch_id) = branch_id.filter(|value| !value.is_empty()) {
        format!(" (Branch: {branch_id})")
    } else if is_revision {
        revises_thought
            .map(|value| format!(" (Revises: {value})"))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let heading = format!("Thought {thought_number}/{display_total} [{agent_id}]{suffix}");
    let thoughts_section = thoughts
        .filter(|value| !value.is_empty())
        .map(|value| format!("\n\n*Thoughts: {value}*"))
        .unwrap_or_default();
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let content = format!("{response}{thoughts_section}\n\n*{timestamp}*\n");
    (heading, content)
}

pub(crate) fn build_completion_message(
    thought_number: u64,
    session_id: &str,
    cancel: bool,
    next_thought_needed: bool,
    needs_more_thoughts: bool,
    total_thoughts: u64,
) -> String {
    let base_message = if thought_number == 0 {
        format!("Created session: {session_id}")
    } else {
        format!("Added thought {thought_number} to session: {session_id}")
    };

    if cancel {
        return format!("{base_message}\n\n❌ Thinking cancelled");
    }
    if !next_thought_needed {
        return format!("{base_message}\n\n✅ Thinking complete");
    }

    let next_thought = thought_number + 1;
    let next_total = if needs_more_thoughts && thought_number >= total_thoughts.saturating_sub(1) {
        "?".to_string()
    } else {
        total_thoughts.to_string()
    };
    let extend_note = if needs_more_thoughts && thought_number >= total_thoughts.saturating_sub(1) {
        " (extending beyond initial estimate)"
    } else {
        ""
    };
    let checkpoint_note = if thought_number == 0 || thought_number.is_multiple_of(3) {
        "\n\n💡 **CHECK YOUR MEMORY:** `search_memories` for what exists and `build_context` on [[WikiLinks]] | `sync` new findings to add them to the graph"
    } else {
        ""
    };

    format!(
        "{base_message}\n\n💭 Continue with thought {next_thought}/{next_total}{extend_note}{checkpoint_note}"
    )
}
