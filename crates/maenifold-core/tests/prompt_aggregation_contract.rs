use maenifold_core::mcp::stdio_pool::{StdioPool, StdioServerConfig};
use std::collections::HashMap;
use std::time::Duration;

const PROMPT_FIXTURE_PATH: &str = "tests/fixtures/prompt_server.py";
const PYTHON: &str = "/usr/bin/python3";

fn fixture_config(name: &str, prompt_name: &str) -> StdioServerConfig {
    StdioServerConfig {
        name: name.to_string(),
        command: PYTHON.to_string(),
        args: vec![
            "-u".to_string(),
            PROMPT_FIXTURE_PATH.to_string(),
            name.to_string(),
            prompt_name.to_string(),
        ],
        env: HashMap::new(),
        timeout: Some(Duration::from_secs(5)),
    }
}

#[tokio::test]
async fn prompt_aggregation_two_servers_same_name() {
    let pool = StdioPool::start(vec![
        fixture_config("fixtureA", "summarize"),
        fixture_config("fixtureB", "summarize"),
    ], std::sync::Arc::new(maenifold_core::mcp::PendingRequestRegistry::new()))
    .await
    .expect("prompt fixture pool should start");

    let discovered = pool.all_gateway_prompts();
    assert_eq!(
        discovered
            .iter()
            .map(|(server, prompts)| (
                server.as_str(),
                prompts
                    .iter()
                    .map(|prompt| prompt.name.as_str())
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("fixtureA", vec!["summarize"]),
            ("fixtureB", vec!["summarize"]),
        ]
    );

    let fixture_a = pool
        .get_prompt("fixtureA", "summarize", None)
        .await
        .expect("fixtureA prompt should resolve");
    let fixture_b = pool
        .get_prompt("fixtureB", "summarize", None)
        .await
        .expect("fixtureB prompt should resolve");
    let fixture_a_json =
        serde_json::to_string(&fixture_a).expect("fixtureA prompt should serialize");
    let fixture_b_json =
        serde_json::to_string(&fixture_b).expect("fixtureB prompt should serialize");
    assert!(fixture_a_json.contains("server-id:fixtureA prompt:summarize"));
    assert!(fixture_b_json.contains("server-id:fixtureB prompt:summarize"));

    pool.shutdown().await;
}
