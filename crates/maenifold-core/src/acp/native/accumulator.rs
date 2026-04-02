use agent_client_protocol as acp;

use super::{AgentResponse, ToolCallRecord};

#[derive(Debug, Default)]
pub(super) struct SessionAccumulator {
    text: String,
    thought: String,
    tool_calls: Vec<acp::ToolCall>,
}

impl SessionAccumulator {
    pub(super) fn append_text(&mut self, text: &str) {
        self.text.push_str(text);
    }

    pub(super) fn append_thought(&mut self, text: &str) {
        self.thought.push_str(text);
    }

    pub(super) fn push_tool_call(&mut self, tool_call: acp::ToolCall) {
        if let Some(existing) = self
            .tool_calls
            .iter_mut()
            .find(|existing| existing.tool_call_id == tool_call.tool_call_id)
        {
            merge_tool_call(existing, tool_call);
        } else {
            self.tool_calls.push(tool_call);
        }
    }

    pub(super) fn apply_tool_call_update(
        &mut self,
        update: acp::ToolCallUpdate,
    ) -> std::result::Result<(), acp::Error> {
        let tool_call_id = update.tool_call_id.clone();
        if let Some(existing) = self
            .tool_calls
            .iter_mut()
            .find(|existing| existing.tool_call_id == tool_call_id)
        {
            existing.update(update.fields);
            return Ok(());
        }
        self.tool_calls.push(acp::ToolCall::try_from(update)?);
        Ok(())
    }

    pub(super) fn mark_in_progress_cancelled(&mut self) {
        for tool_call in &mut self.tool_calls {
            if matches!(tool_call.status, acp::ToolCallStatus::InProgress) {
                tool_call.status = acp::ToolCallStatus::Failed;
            }
        }
    }

    pub(super) fn into_response(self) -> AgentResponse {
        AgentResponse {
            text: self.text,
            tool_calls: self
                .tool_calls
                .into_iter()
                .map(ToolCallRecord::from)
                .collect(),
            thought: (!self.thought.trim().is_empty()).then_some(self.thought),
            stop_reason: None,
            usage: None,
        }
    }
}

fn merge_tool_call(existing: &mut acp::ToolCall, update: acp::ToolCall) {
    let acp::ToolCall {
        title,
        kind,
        status,
        content,
        locations,
        raw_input,
        raw_output,
        meta,
        ..
    } = update;

    existing.update(
        acp::ToolCallUpdateFields::new()
            .title((!title.is_empty()).then_some(title))
            .kind((!matches!(kind, acp::ToolKind::Other)).then_some(kind))
            .status((!matches!(status, acp::ToolCallStatus::Pending)).then_some(status))
            .content((!content.is_empty()).then_some(content))
            .locations((!locations.is_empty()).then_some(locations))
            .raw_input(raw_input.filter(|value| {
                !value.is_null()
                    && !matches!(value, serde_json::Value::String(text) if text.is_empty())
            }))
            .raw_output(raw_output),
    );

    if let Some(meta) = meta {
        existing.meta = Some(meta);
    }
}

