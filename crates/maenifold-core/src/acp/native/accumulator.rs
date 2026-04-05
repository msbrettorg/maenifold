use agent_client_protocol as acp;
use sacp::schema as sacps;

use super::{ToolCallRecord, convert::bridge, types::AgentResponse};

#[derive(Debug, Default)]
pub(super) struct SessionAccumulator {
    text: String,
    thought: String,
    tool_calls: Vec<sacps::ToolCall>,
}

impl SessionAccumulator {
    pub(super) fn append_text(&mut self, text: &str) {
        self.text.push_str(text);
    }

    pub(super) fn append_thought(&mut self, text: &str) {
        self.thought.push_str(text);
    }

    pub(super) fn push_tool_call(&mut self, tool_call: sacps::ToolCall) {
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

    pub(super) fn apply_tool_call_update(&mut self, update: sacps::ToolCallUpdate) {
        let tool_call_id = update.tool_call_id.clone();
        if let Some(existing) = self
            .tool_calls
            .iter_mut()
            .find(|existing| existing.tool_call_id == tool_call_id)
        {
            existing.update(update.fields);
            return;
        }
        self.tool_calls.push(
            sacps::ToolCall::new(tool_call_id, "").status(
                update.fields.status.unwrap_or_default(),
            ),
        );
        if let Some(existing) = self.tool_calls.last_mut() {
            existing.update(update.fields);
        }
    }

    pub(super) fn mark_in_progress_cancelled(&mut self) {
        for tool_call in &mut self.tool_calls {
            if matches!(tool_call.status, sacps::ToolCallStatus::InProgress) {
                tool_call.status = sacps::ToolCallStatus::Failed;
            }
        }
    }

    pub(super) fn into_response(self) -> AgentResponse {
        AgentResponse {
            text: self.text,
            tool_calls: self.tool_calls.into_iter().map(tool_call_record).collect(),
            thought: (!self.thought.trim().is_empty()).then_some(self.thought),
            stop_reason: None,
            error: None,
            usage: None,
        }
    }
}

fn tool_call_record(tool_call: sacps::ToolCall) -> ToolCallRecord {
    match bridge::<_, acp::ToolCall>(tool_call.clone()) {
        Ok(tool_call) => ToolCallRecord::from(tool_call),
        Err(error) => {
            tracing::warn!("Failed to bridge tool call: {error}");
            ToolCallRecord {
                id: Some(tool_call.tool_call_id.to_string()),
                name: serialized_name(tool_call.kind),
                title: Some(tool_call.title),
                status: serialized_name(tool_call.status),
                arguments: tool_call.raw_input,
                output: tool_call.raw_output,
                content: vec![],
                locations: vec![],
            }
        }
    }
}

fn serialized_name<T: serde::Serialize>(value: T) -> Option<String> {
    match serde_json::to_value(value).ok()? {
        serde_json::Value::String(value) => Some(value),
        _ => None,
    }
}

fn merge_tool_call(existing: &mut sacps::ToolCall, update: sacps::ToolCall) {
    let sacps::ToolCall {
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
        sacps::ToolCallUpdateFields::new()
            .title((!title.is_empty()).then_some(title))
            .kind((!matches!(kind, sacps::ToolKind::Other)).then_some(kind))
            .status((!matches!(status, sacps::ToolCallStatus::Pending)).then_some(status))
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
