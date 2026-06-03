use std::fs;

use jockey::copilot_models::CopilotModelsEnvelope;

#[test]
fn parses_copilot_models_fixture_with_exact_reasoning_effort_shape() {
    let fixture = fs::read_to_string("tests/fixtures/copilot-models.json")
        .expect("read Copilot models fixture");
    let envelope: CopilotModelsEnvelope =
        serde_json::from_str(&fixture).expect("parse Copilot models fixture");
    let gpt_55 = envelope
        .data
        .iter()
        .find(|model| model.id == "gpt-5.5")
        .expect("find gpt-5.5");
    assert!(
        gpt_55
            .capabilities
            .supports
            .reasoning_effort
            .contains(&"xhigh".to_string())
    );
    let opus = envelope
        .data
        .iter()
        .find(|model| model.id == "claude-opus-4.7")
        .expect("find claude-opus-4.7");
    let opus_xhigh = envelope
        .data
        .iter()
        .find(|model| model.id == "claude-opus-4.7-xhigh")
        .expect("find claude-opus-4.7-xhigh");
    assert_ne!(opus.id, opus_xhigh.id);
    assert_eq!(gpt_55.capabilities.supports.structured_outputs, Some(true));
    assert_eq!(opus.capabilities.supports.structured_outputs, None);
    assert_eq!(
        gpt_55.capabilities.limits.max_non_streaming_output_tokens,
        Some(32_000)
    );
    assert_eq!(
        opus.capabilities.limits.max_non_streaming_output_tokens,
        None
    );
}
