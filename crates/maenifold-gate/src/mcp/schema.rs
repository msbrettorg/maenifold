use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
}

pub fn list() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "status",
            description: "List gate task status from the authoritative todo/ and done/ files.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": { "type": "string" },
                    "pipeline": { "type": "string" }
                },
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "append",
            description: "Append a non-advancing stage work record to a gate task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": { "type": "string" },
                    "root": { "type": "string" },
                    "pipeline": { "type": "string" },
                    "agent": { "type": "string" },
                    "notes": { "type": "string" },
                    "model": { "type": "string" }
                },
                "required": ["taskId", "agent", "notes"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "pass",
            description: "Apply a PASS outcome to the active gate stage for a task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": { "type": "string" },
                    "root": { "type": "string" },
                    "pipeline": { "type": "string" },
                    "agent": { "type": "string" },
                    "notes": { "type": "string" },
                    "model": { "type": "string" }
                },
                "required": ["taskId", "agent", "notes"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "fail",
            description: "Apply a FAIL outcome to the active gate stage for a task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "taskId": { "type": "string" },
                    "root": { "type": "string" },
                    "pipeline": { "type": "string" },
                    "agent": { "type": "string" },
                    "notes": { "type": "string" },
                    "model": { "type": "string" }
                },
                "required": ["taskId", "agent", "notes"],
                "additionalProperties": false
            }),
        },
        McpToolDefinition {
            name: "sync",
            description: "Rebuild or refresh .gate/gate.db from the authoritative markdown files.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": { "type": "string" },
                    "pipeline": { "type": "string" },
                    "rebuild": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::list;

    #[test]
    fn mcp_tool_list_exposes_exactly_five_gate_tools() {
        let tools = list();
        let names: Vec<_> = tools.iter().map(|tool| tool.name).collect();
        assert_eq!(names, vec!["status", "append", "pass", "fail", "sync"]);
    }
}
