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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn multi_chunk_text_concatenation() {
        let mut accumulator = SessionAccumulator::default();

        accumulator.append_text("hello");
        accumulator.append_text(", ");
        accumulator.append_text("world");

        let response = accumulator.into_response();
        assert_eq!(response.text, "hello, world");
        assert!(response.tool_calls.is_empty());
        assert_eq!(response.thought, None);
    }

    #[test]
    fn push_tool_call_then_apply_update_sequence() {
        let mut accumulator = SessionAccumulator::default();

        accumulator.push_tool_call(
            acp::ToolCall::new("call-1", "Read file")
                .kind(acp::ToolKind::Read)
                .raw_input(json!({ "path": "src/lib.rs" })),
        );

        accumulator
            .apply_tool_call_update(acp::ToolCallUpdate::new(
                "call-1",
                acp::ToolCallUpdateFields::new()
                    .status(acp::ToolCallStatus::Completed)
                    .raw_output(json!({ "ok": true })),
            ))
            .expect("tool call update should apply");

        let response = accumulator.into_response();
        let record = &response.tool_calls[0];

        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(record.id.as_deref(), Some("call-1"));
        assert_eq!(record.name.as_deref(), Some("read"));
        assert_eq!(record.title.as_deref(), Some("Read file"));
        assert_eq!(record.status.as_deref(), Some("completed"));
        assert_eq!(
            record.arguments.as_ref(),
            Some(&json!({ "path": "src/lib.rs" }))
        );
        assert_eq!(record.output.as_ref(), Some(&json!({ "ok": true })));
    }

    #[test]
    fn duplicate_push_tool_call_merges_instead_of_overwriting() {
        let mut accumulator = SessionAccumulator::default();

        accumulator.push_tool_call(
            acp::ToolCall::new("call-1", "Read file")
                .kind(acp::ToolKind::Read)
                .status(acp::ToolCallStatus::InProgress)
                .raw_input(json!({ "path": "src/lib.rs" })),
        );

        accumulator.push_tool_call(
            acp::ToolCall::new("call-1", "")
                .status(acp::ToolCallStatus::Completed)
                .raw_output(json!({ "ok": true })),
        );

        let response = accumulator.into_response();
        let record = &response.tool_calls[0];

        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(record.id.as_deref(), Some("call-1"));
        assert_eq!(record.name.as_deref(), Some("read"));
        assert_eq!(record.title.as_deref(), Some("Read file"));
        assert_eq!(record.status.as_deref(), Some("completed"));
        assert_eq!(
            record.arguments.as_ref(),
            Some(&json!({ "path": "src/lib.rs" }))
        );
        assert_eq!(record.output.as_ref(), Some(&json!({ "ok": true })));
    }

    #[test]
    fn out_of_order_tool_call_update_creates_tool_call() {
        let mut accumulator = SessionAccumulator::default();

        accumulator
            .apply_tool_call_update(acp::ToolCallUpdate::new(
                "call-2",
                acp::ToolCallUpdateFields::new()
                    .title("Search docs")
                    .kind(acp::ToolKind::Search)
                    .status(acp::ToolCallStatus::InProgress)
                    .raw_input(json!({ "query": "acp" })),
            ))
            .expect("out-of-order update should create tool call");

        let response = accumulator.into_response();
        let record = &response.tool_calls[0];

        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(record.id.as_deref(), Some("call-2"));
        assert_eq!(record.name.as_deref(), Some("search"));
        assert_eq!(record.title.as_deref(), Some("Search docs"));
        assert_eq!(record.status.as_deref(), Some("in_progress"));
        assert_eq!(record.arguments.as_ref(), Some(&json!({ "query": "acp" })));
    }

    #[test]
    fn into_response_omits_whitespace_only_thought() {
        let mut accumulator = SessionAccumulator::default();

        accumulator.append_thought(" \n\t ");

        let response = accumulator.into_response();
        assert_eq!(response.thought, None);
    }

    #[test]
    fn into_response_with_empty_accumulator() {
        let response = SessionAccumulator::default().into_response();

        assert_eq!(response.text, "");
        assert!(response.tool_calls.is_empty());
        assert_eq!(response.thought, None);
        assert_eq!(response.stop_reason, None);
        assert_eq!(response.usage, None);
    }
}
