//! Shared helpers for Pipeline MCP tool E2E tests.
//!
//! All tests spawn `maenifold --mcp` via [`McpTestHarness`] and call the
//! `Pipeline` tool through JSON-RPC `tools/call`.

#![allow(dead_code)]

use serde_json::{Value, json};

#[path = "../mcp_test_support/mod.rs"]
mod mcp_test_support;
pub use mcp_test_support::{McpTestHarness, isolated_root};

/// Wrapper around [`McpTestHarness`] with Pipeline-specific convenience methods.
pub struct PipelineHarness {
    pub h: McpTestHarness,
}

impl PipelineHarness {
    /// Spawn `maenifold --mcp`, initialize, return a ready harness.
    pub fn new() -> (tempfile::TempDir, Self) {
        let tmp = isolated_root();
        let mut h = McpTestHarness::spawn(&tmp);
        h.initialize();
        (tmp, Self { h })
    }

    /// Raw call to the Pipeline tool. Returns the parsed JSON response.
    pub fn pipeline(&mut self, args: Value) -> Value {
        self.h.request(
            "tools/call",
            json!({
                "name": "Pipeline",
                "arguments": args
            }),
        )
    }

    /// Call Pipeline and extract the `data` object from content text.
    /// Panics with a clear message if the call returned an error.
    pub fn pipeline_ok(&mut self, args: Value) -> Value {
        let resp = self.pipeline(args.clone());
        let result = &resp["result"];
        if result["isError"] == true {
            let text = Self::extract_text(result);
            panic!("Pipeline call failed (args={args}): {text}");
        }
        Self::parse_data(result)
    }

    /// Call Pipeline and assert it returns isError=true. Returns the error text.
    pub fn pipeline_err(&mut self, args: Value) -> String {
        let resp = self.pipeline(args.clone());
        let result = &resp["result"];
        assert_eq!(
            result["isError"],
            true,
            "Expected error for args={args}, got success: {}",
            Self::extract_text(result)
        );
        Self::extract_text(result)
    }

    fn session_args(session_id: &str, field: &str, value: Value) -> Value {
        let mut args = serde_json::Map::new();
        args.insert("sessionId".to_string(), json!(session_id));
        args.insert(field.to_string(), value);
        Value::Object(args)
    }

    fn session_value(&mut self, session_id: &str, field: &str, value: Value) -> Value {
        self.pipeline_ok(Self::session_args(session_id, field, value))
    }

    fn session_err(&mut self, session_id: &str, field: &str, value: Value) -> String {
        self.pipeline_err(Self::session_args(session_id, field, value))
    }

    /// Create a pipeline from a definition. Returns the parsed data.
    pub fn create(&mut self, definition: Value) -> Value {
        self.pipeline_ok(json!({ "definition": definition }))
    }

    /// Create with maxConcurrency.
    pub fn create_with_concurrency(&mut self, definition: Value, max: u32) -> Value {
        self.pipeline_ok(json!({
            "definition": definition,
            "maxConcurrency": max
        }))
    }

    /// View a pipeline by session ID. Returns parsed data.
    pub fn view(&mut self, session_id: &str) -> Value {
        self.pipeline_ok(json!({ "sessionId": session_id, "view": true }))
    }

    /// List all pipelines. Returns parsed data.
    pub fn list(&mut self) -> Value {
        self.pipeline_ok(json!({ "list": true }))
    }

    /// Delete a pipeline.
    pub fn delete(&mut self, session_id: &str) -> Value {
        self.pipeline_ok(json!({ "sessionId": session_id, "delete": true }))
    }

    /// Dispatch ready nodes. Returns parsed data.
    pub fn dispatch(&mut self, session_id: &str) -> Value {
        self.pipeline_ok(json!({
            "sessionId": session_id,
            "dispatch": true
        }))
    }

    /// Complete a node with a conclusion.
    pub fn complete(&mut self, session_id: &str, node_id: &str, conclusion: &str) -> Value {
        self.pipeline_ok(json!({
            "sessionId": session_id,
            "nodeId": node_id,
            "status": "completed",
            "conclusion": conclusion
        }))
    }

    /// Abandon a node.
    pub fn abandon(&mut self, session_id: &str, node_id: &str) -> Value {
        self.pipeline_ok(json!({
            "sessionId": session_id,
            "nodeId": node_id,
            "status": "abandoned"
        }))
    }

    /// Run a pipeline (supervisor execution).
    pub fn run(&mut self, session_id: &str) -> Value {
        self.pipeline_ok(json!({
            "sessionId": session_id,
            "run": true
        }))
    }

    /// Graft nodes/edges onto an existing pipeline.
    pub fn graft(&mut self, session_id: &str, graft: Value) -> Value {
        self.session_value(session_id, "graft", graft)
    }

    /// Graft expecting an error.
    pub fn graft_err(&mut self, session_id: &str, graft: Value) -> String {
        self.session_err(session_id, "graft", graft)
    }

    /// Join another pipeline into this one.
    pub fn join(&mut self, session_id: &str, join: Value) -> Value {
        self.session_value(session_id, "join", join)
    }

    /// Join expecting an error.
    pub fn join_err(&mut self, session_id: &str, join: Value) -> String {
        self.session_err(session_id, "join", join)
    }

    /// Fork from a node.
    pub fn fork(&mut self, session_id: &str, fork: Value) -> Value {
        self.pipeline_ok(json!({
            "sessionId": session_id,
            "fork": fork
        }))
    }

    /// Rewind nodes.
    pub fn rewind(&mut self, session_id: &str, rewind: Value) -> Value {
        self.session_value(session_id, "rewind", rewind)
    }

    /// Rewind expecting an error.
    pub fn rewind_err(&mut self, session_id: &str, rewind: Value) -> String {
        self.session_err(session_id, "rewind", rewind)
    }

    /// Restart a node.
    pub fn restart(&mut self, session_id: &str, node_id: &str) -> Value {
        self.pipeline_ok(json!({
            "sessionId": session_id,
            "nodeId": node_id,
            "restart": true
        }))
    }

    /// Restart expecting an error.
    pub fn restart_err(&mut self, session_id: &str, node_id: &str) -> String {
        self.pipeline_err(json!({
            "sessionId": session_id,
            "nodeId": node_id,
            "restart": true
        }))
    }

    /// Extract the text content from an MCP result envelope.
    pub fn extract_text(result: &Value) -> String {
        result["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .unwrap_or("<no text>")
            .to_string()
    }

    /// Parse the text content as JSON and return the `data` field.
    /// Falls back to the full parsed JSON if no `data` key.
    pub fn parse_data(result: &Value) -> Value {
        let text = Self::extract_text(result);
        let parsed: Value = serde_json::from_str(&text).unwrap_or_else(|e| {
            panic!("Failed to parse Pipeline response as JSON: {e}\nRaw: {text}")
        });
        if parsed.get("data").is_some() {
            parsed["data"].clone()
        } else {
            parsed
        }
    }

    /// Get the pipeline ID from a create response.
    pub fn id(data: &Value) -> String {
        data["pipelineId"]
            .as_str()
            .expect("response must contain pipelineId")
            .to_string()
    }

    /// Get node status from a view response by node ID.
    pub fn node_status(view_data: &Value, node_id: &str) -> String {
        let nodes = view_data["nodes"]
            .as_array()
            .expect("view must have nodes array");
        for node in nodes {
            if node["nodeId"].as_str() == Some(node_id) {
                return node["status"].as_str().unwrap_or("unknown").to_string();
            }
        }
        panic!("Node '{node_id}' not found in view response");
    }

    /// Check if a node ID is in the `ready` array.
    pub fn is_ready(data: &Value, node_id: &str) -> bool {
        data["ready"]
            .as_array()
            .map(|arr| arr.iter().any(|v| v.as_str() == Some(node_id)))
            .unwrap_or(false)
    }

    /// Count nodes with a given status in a view response.
    pub fn count_status(view_data: &Value, status: &str) -> usize {
        view_data["nodes"]
            .as_array()
            .map(|nodes| {
                nodes
                    .iter()
                    .filter(|n| n["status"].as_str() == Some(status))
                    .count()
            })
            .unwrap_or(0)
    }

    /// Simple tool node definition.
    pub fn tool_node(id: &str) -> Value {
        json!({
            "id": id,
            "type": "tool",
            "toolName": "MemoryStatus"
        })
    }

    /// Success edge.
    pub fn edge(source: &str, target: &str) -> Value {
        json!({ "source": source, "target": target, "type": "success" })
    }

    /// Always edge.
    pub fn always_edge(source: &str, target: &str) -> Value {
        json!({ "source": source, "target": target, "type": "always" })
    }

    /// Edge without explicit type (tests default).
    pub fn bare_edge(source: &str, target: &str) -> Value {
        json!({ "source": source, "target": target })
    }
}
