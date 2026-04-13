pub(super) fn build_prepared_prompt(
    is_union_assistant: bool,
    tool_prompt: &str,
    role_system_prompt: Option<&str>,
    context_pairs: &[(String, String)],
    message: &str,
) -> String {
    let ctx_bytes: usize = context_pairs
        .iter()
        .map(|(k, v)| k.len() + v.len() + 4)
        .sum();
    let estimated = if is_union_assistant {
        tool_prompt.len() + 10
    } else {
        0
    } + ctx_bytes
        + message.len()
        + 64;
    let mut prepared = String::with_capacity(estimated);
    if is_union_assistant {
        prepared.push_str("Tools:\n");
        prepared.push_str(tool_prompt);
    }
    if let Some(sp) = role_system_prompt {
        if !prepared.is_empty() {
            prepared.push_str("\n\n");
        }
        prepared.push_str("System:\n");
        prepared.push_str(sp);
    }
    let is_slash_cmd = message.starts_with('/');
    if !is_slash_cmd {
        if !context_pairs.is_empty() {
            if !prepared.is_empty() {
                prepared.push_str("\n\n");
            }
            prepared.push_str("Context:\n");
            for (i, (k, v)) in context_pairs.iter().enumerate() {
                if i > 0 {
                    prepared.push('\n');
                }
                prepared.push_str(k);
                prepared.push_str(": ");
                prepared.push_str(v);
            }
        }
        if !prepared.is_empty() {
            prepared.push_str("\n\n");
        }
        prepared.push_str("User:\n");
    }
    prepared.push_str(message);
    prepared
}

pub(super) fn with_command_suggestion(
    output: String,
    explicit_role_targets: bool,
    is_union_assistant: bool,
) -> String {
    if explicit_role_targets || !is_union_assistant {
        return output;
    }
    if let Some(command_text) = crate::chat::extract_command_output(&output) {
        return format!(
            "{}\n\n[Command suggestion]\n{}\nRun this command manually if you want to apply it.",
            output, command_text
        );
    }
    output
}
