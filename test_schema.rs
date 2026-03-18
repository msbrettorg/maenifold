use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")] // Discriminated union
enum PipelineNode {
    Workflow {
        id: String,
        workflow_id: String,
        acp_client: Option<String>,
    },
    Prompt {
        id: String,
        prompt: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
}

fn main() {
    let workflow_node = PipelineNode::Workflow {
        id: "node-1".to_string(),
        workflow_id: "wf-123".to_string(),
        acp_client: Some("agent-1".to_string()),
    };

    let prompt_node = PipelineNode::Prompt {
        id: "node-2".to_string(),
        prompt: "Summarize {{node-1.output}}".to_string(),
        model: None,
    };

    let json_workflow = serde_json::to_string_pretty(&workflow_node).unwrap();
    let json_prompt = serde_json::to_string_pretty(&prompt_node).unwrap();

    println!("Workflow Node JSON:\n{}", json_workflow);
    println!("Prompt Node JSON:\n{}", json_prompt);

    // Test deserialization of legacy format (if we need to support it, we might need untagged or other tricks, 
    // but typically we just migrate data).
    // Let's assume we want to support the OLD format which didn't have "type".
    // Actually, the prompt implies we are *designing* it, so we can dictate the schema.
}
