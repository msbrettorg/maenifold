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
            *existing = tool_call;
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

impl From<acp::ToolCall> for ToolCallRecord {
    fn from(value: acp::ToolCall) -> Self {
        Self {
            id: Some(value.tool_call_id.to_string()),
            name: tool_kind_name(value.kind).map(str::to_string),
            title: Some(value.title),
            status: tool_call_status_name(value.status).map(str::to_string),
            arguments: value.raw_input,
        }
    }
}

fn tool_kind_name(kind: acp::ToolKind) -> Option<&'static str> {
    Some(match kind {
        acp::ToolKind::Read => "read",
        acp::ToolKind::Edit => "edit",
        acp::ToolKind::Delete => "delete",
        acp::ToolKind::Move => "move",
        acp::ToolKind::Search => "search",
        acp::ToolKind::Execute => "execute",
        acp::ToolKind::Think => "think",
        acp::ToolKind::Fetch => "fetch",
        acp::ToolKind::SwitchMode => "switch_mode",
        acp::ToolKind::Other => "other",
        _ => return None,
    })
}

fn tool_call_status_name(status: acp::ToolCallStatus) -> Option<&'static str> {
    Some(match status {
        acp::ToolCallStatus::Pending => "pending",
        acp::ToolCallStatus::InProgress => "in_progress",
        acp::ToolCallStatus::Completed => "completed",
        acp::ToolCallStatus::Failed => "failed",
        _ => return None,
    })
}
