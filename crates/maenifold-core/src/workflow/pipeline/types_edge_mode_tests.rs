use std::collections::HashMap;

use serde_json::json;
use tracing_test::traced_test;

use super::types::{EdgeDataMode, EdgeType, PipelineEdge};
use super::{PipelineDefinition, PipelineNode, validate_pipeline};

fn tool_node(id: &str) -> PipelineNode {
    PipelineNode::Tool {
        id: id.to_string(),
        display_label: None,
        tool_name: format!("tool-{id}"),
        tool_args: None,
        agent: None,
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        retry_policy: None,
        join_policy: None,
    }
}

fn prompt_node(id: &str) -> PipelineNode {
    PipelineNode::Prompt {
        id: id.to_string(),
        display_label: None,
        prompt: format!("prompt {id}"),
        agent: "copilot".to_string(),
        tool_mode: "read".to_string(),
        model: None,
        thought_level: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        timeout: None,
        agent_config: None,
        adopt: None,
        requires_role: None,
        retry_policy: None,
        session_ref: None,
        join_policy: None,
        input_schema: None,
        output_schema: None,
    }
}

fn mapping() -> HashMap<String, String> {
    HashMap::from([("output.answer".to_string(), "input.answer".to_string())])
}

fn success_edge(source: &str, target: &str, mode: EdgeDataMode) -> PipelineEdge {
    PipelineEdge {
        source: source.to_string(),
        target: target.to_string(),
        edge_type: EdgeType::Success,
        mode,
        mapping: None,
    }
}

#[test]
fn edge_mode_defaults_to_context_when_field_missing() {
    let edge: PipelineEdge = serde_json::from_value(json!({
        "source": "source",
        "target": "target",
        "type": "success"
    }))
    .expect("edge without mode should deserialize");

    assert_eq!(edge.mode, EdgeDataMode::Context);
}

#[test]
fn edge_mode_round_trip() {
    for mode in [
        EdgeDataMode::Context,
        EdgeDataMode::Binding,
        EdgeDataMode::None,
    ] {
        let edge = success_edge("source", "target", mode.clone());
        let serialized = serde_json::to_string(&edge).expect("edge should serialize");
        let deserialized: PipelineEdge =
            serde_json::from_str(&serialized).expect("edge should deserialize");

        assert_eq!(deserialized, edge);
    }
}

#[test]
fn edge_mode_binding_serializes_lowercase_camel() {
    let cases = [
        (EdgeDataMode::Binding, "binding"),
        (EdgeDataMode::Context, "context"),
        (EdgeDataMode::None, "none"),
    ];

    for (mode, expected) in cases {
        let edge = success_edge("source", "target", mode);
        let value = serde_json::to_value(edge).expect("edge should serialize");

        assert_eq!(
            value.get("mode").and_then(|mode| mode.as_str()),
            Some(expected)
        );
    }
}

#[test]
fn legacy_mapping_on_tool_target_preserved() {
    let mut definition = PipelineDefinition {
        nodes: vec![prompt_node("source"), tool_node("target")],
        edges: vec![PipelineEdge {
            source: "source".to_string(),
            target: "target".to_string(),
            edge_type: EdgeType::Success,
            mode: EdgeDataMode::Context,
            mapping: Some(mapping()),
        }],
        inputs: None,
        input_schema: None,
        template_id: None,
    };

    definition.migrate_legacy_edges();

    assert_eq!(definition.edges[0].mapping.as_ref(), Some(&mapping()));
}

#[test]
#[traced_test]
fn legacy_mapping_on_acp_target_dropped_with_warning() {
    let mut definition = PipelineDefinition {
        nodes: vec![tool_node("source"), prompt_node("target")],
        edges: vec![PipelineEdge {
            source: "source".to_string(),
            target: "target".to_string(),
            edge_type: EdgeType::Success,
            mode: EdgeDataMode::Context,
            mapping: Some(mapping()),
        }],
        inputs: None,
        input_schema: None,
        template_id: None,
    };

    definition.migrate_legacy_edges();

    assert_eq!(definition.edges[0].mapping, None);
    assert!(logs_contain("legacy edge.mapping ignored for ACP target"));
}

#[test]
fn validation_binding_mode_without_schemas_errors() {
    let definition = PipelineDefinition {
        nodes: vec![tool_node("source"), tool_node("target")],
        edges: vec![success_edge("source", "target", EdgeDataMode::Binding)],
        inputs: None,
        input_schema: None,
        template_id: None,
    };

    let error = validate_pipeline(&definition).expect_err("binding without schemas must fail");

    assert!(
        error
            .to_string()
            .contains("Binding edge requires declared schemas on both endpoints")
    );
}

#[test]
fn validation_none_mode_with_non_empty_mapping_errors() {
    let mut edge = success_edge("source", "target", EdgeDataMode::None);
    edge.mapping = Some(mapping());
    let definition = PipelineDefinition {
        nodes: vec![tool_node("source"), tool_node("target")],
        edges: vec![edge],
        inputs: None,
        input_schema: None,
        template_id: None,
    };

    let error = validate_pipeline(&definition).expect_err("none mode with mapping must fail");

    assert!(error.to_string().contains("None mode forbids mapping"));
}

#[test]
fn validation_context_mode_default_for_existing_pipelines_passes() {
    let fixtures = [
        json!({
            "nodes": [
                {"type": "tool", "id": "source", "toolName": "load"},
                {"type": "tool", "id": "target", "toolName": "save"}
            ],
            "edges": [{"source": "source", "target": "target", "type": "success"}]
        }),
        json!({
            "nodes": [
                {"type": "prompt", "id": "prompt", "prompt": "Summarize", "agent": "copilot"},
                {"type": "tool", "id": "tool", "toolName": "memory.write"}
            ],
            "edges": [{"source": "prompt", "target": "tool", "type": "success"}]
        }),
    ];

    for fixture in fixtures {
        let definition: PipelineDefinition =
            serde_json::from_value(fixture).expect("fixture should deserialize");
        validate_pipeline(&definition).expect("must pass");
    }
}

#[test]
#[ignore = "requires user home pipeline session files; run manually with --ignored"]
fn live_session_files_load_cleanly() {
    let paths = [
        "/Users/brett/maenifold/memory/pipelines/2026/05/18/sprint-329-mini-1779120323606.md",
        "/Users/brett/maenifold/memory/pipelines/2026/05/18/sprint-329-2-1779127062976.md",
        "/Users/brett/maenifold/memory/pipelines/2026/05/18/sprint-329-3-1779127272722.md",
    ];
    for path in paths {
        let raw = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("read {path}"));
        // Extract YAML frontmatter between leading "---\n" delimiters.
        let body = raw.strip_prefix("---\n").expect("frontmatter starts");
        let end = body.find("\n---").expect("frontmatter ends");
        let fm: serde_yml::Value = serde_yml::from_str(&body[..end]).expect("yaml");
        let def_value = fm.get("definition").cloned().expect("definition block");
        let mut def: PipelineDefinition = serde_yml::from_value(def_value).expect("definition");
        def.migrate_legacy_edges();
        validate_pipeline(&def).unwrap_or_else(|e| panic!("{path} failed: {e}"));
        // Every edge should land at mode == Context after migration.
        for e in &def.edges {
            assert_eq!(
                e.mode,
                EdgeDataMode::Context,
                "edge {:?}→{:?}",
                e.source,
                e.target
            );
        }
    }
}
