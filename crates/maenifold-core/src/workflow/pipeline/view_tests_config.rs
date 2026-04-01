use super::*;

#[test]
#[serial(config)]
fn pipeline_view_includes_node_config_fields_for_all_supported_variants() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let payload = json!({
        "definition": {
            "nodes": [
                {
                    "id": "workflow",
                    "workflowId": "test-workflow-a",
                    "acpClient": "workflow-agent",
                    "prompt": "Review the upstream workflow output",
                    "timeout": 120
                },
                {
                    "id": "prompt",
                    "type": "prompt",
                    "prompt": "Draft the operator summary",
                    "agent": "prompt-agent",
                    "toolMode": "required",
                    "model": "gpt-5.4-mini",
                    "thoughtLevel": "high",
                    "allowedTools": ["ReadMemory", "SearchMemories"],
                    "maxTurns": 7,
                    "timeout": 90,
                    "agentConfig": {
                        "model": "gpt-5.4",
                        "mode": "interactive",
                        "thoughtLevel": "medium",
                        "configOptions": { "sandbox": "workspace-write" }
                    }
                },
                {
                    "id": "thinking",
                    "type": "sequentialThinking",
                    "topic": "Trace the pipeline execution path",
                    "totalThoughts": 4,
                    "analysisType": "diagnostic",
                    "agent": "thinking-agent",
                    "model": "claude-sonnet-4.6",
                    "thoughtLevel": "deep",
                    "allowedTools": ["BuildContext"],
                    "maxTurns": 5,
                    "timeout": 45,
                    "agentConfig": {
                        "model": "claude-sonnet-4.5",
                        "mode": "batch",
                        "thoughtLevel": "high",
                        "configOptions": { "temperature": 0 }
                    }
                },
                {
                    "id": "tool",
                    "type": "tool",
                    "toolName": "ReadMemory",
                    "toolArgs": { "identifier": "memory://build/test" },
                    "agent": "tool-agent",
                    "model": "gpt-5.2",
                    "thoughtLevel": "low",
                    "allowedTools": ["ReadMemory", "ListMemories"],
                    "maxTurns": 3,
                    "agentConfig": {
                        "model": "gpt-5.1",
                        "mode": "tool",
                        "thoughtLevel": "medium",
                        "configOptions": { "verbosity": "low" }
                    }
                }
            ],
            "edges": [
                { "source": "workflow", "target": "prompt" },
                { "source": "prompt", "target": "thinking" },
                { "source": "thinking", "target": "tool" }
            ]
        }
    });

    let create_output = pipeline(&context, &payload).expect("create pipeline");
    let create_response: Value =
        serde_json::from_str(&create_output).expect("parse create response");
    let pipeline_id = create_response["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id");

    let view_output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
        .expect("view pipeline");
    let view_response: Value = serde_json::from_str(&view_output).expect("parse view response");
    let nodes = view_response["data"]["nodes"]
        .as_array()
        .expect("nodes array");

    let workflow_node = nodes
        .iter()
        .find(|node| node["nodeId"] == "workflow")
        .expect("workflow node");
    assert_eq!(workflow_node["timeout"], 120);

    let prompt_node = nodes
        .iter()
        .find(|node| node["nodeId"] == "prompt")
        .expect("prompt node");
    assert_eq!(prompt_node["toolMode"], "required");
    assert_eq!(prompt_node["model"], "gpt-5.4-mini");
    assert_eq!(prompt_node["thoughtLevel"], "high");
    assert_eq!(
        prompt_node["allowedTools"],
        json!(["ReadMemory", "SearchMemories"])
    );
    assert_eq!(prompt_node["maxTurns"], 7);
    assert_eq!(prompt_node["timeout"], 90);
    assert_eq!(
        prompt_node["agentConfig"],
        json!({
            "model": "gpt-5.4",
            "mode": "interactive",
            "thoughtLevel": "medium",
            "configOptions": { "sandbox": "workspace-write" }
        })
    );

    let thinking_node = nodes
        .iter()
        .find(|node| node["nodeId"] == "thinking")
        .expect("thinking node");
    assert_eq!(thinking_node["model"], "claude-sonnet-4.6");
    assert_eq!(thinking_node["thoughtLevel"], "deep");
    assert_eq!(thinking_node["allowedTools"], json!(["BuildContext"]));
    assert_eq!(thinking_node["maxTurns"], 5);
    assert_eq!(thinking_node["timeout"], 45);
    assert_eq!(
        thinking_node["agentConfig"],
        json!({
            "model": "claude-sonnet-4.5",
            "mode": "batch",
            "thoughtLevel": "high",
            "configOptions": { "temperature": 0 }
        })
    );

    let tool_node = nodes
        .iter()
        .find(|node| node["nodeId"] == "tool")
        .expect("tool node");
    assert_eq!(tool_node["model"], "gpt-5.2");
    assert_eq!(tool_node["thoughtLevel"], "low");
    assert_eq!(
        tool_node["allowedTools"],
        json!(["ReadMemory", "ListMemories"])
    );
    assert_eq!(tool_node["maxTurns"], 3);
    assert_eq!(
        tool_node["agentConfig"],
        json!({
            "model": "gpt-5.1",
            "mode": "tool",
            "thoughtLevel": "medium",
            "configOptions": { "verbosity": "low" }
        })
    );
}
