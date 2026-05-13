use crate::acp::config::AgentsConfig;

#[test]
fn agents_config_default_has_no_sampling_agent() {
    let cfg = AgentsConfig::default();
    assert!(cfg.sampling_agent.is_none());
}

#[test]
fn agents_config_loads_sampling_agent_field() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    std::fs::write(
        temp_dir.path().join("maenifold.json"),
        r#"{"samplingAgent":"copilot","mcpServers":[],"agents":{}}"#,
    )
    .expect("config file should be written");

    let cfg = AgentsConfig::load(temp_dir.path()).expect("config should load");

    assert_eq!(cfg.sampling_agent, Some("copilot".to_string()));
}

#[test]
fn agents_config_without_sampling_agent_field_still_parses() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    std::fs::write(
        temp_dir.path().join("maenifold.json"),
        r#"{"mcpServers":[],"agents":{}}"#,
    )
    .expect("config file should be written");

    let cfg = AgentsConfig::load(temp_dir.path()).expect("config should load");

    assert!(cfg.sampling_agent.is_none());
}
