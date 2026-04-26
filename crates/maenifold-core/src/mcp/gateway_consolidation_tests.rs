use super::{ConsolidatedServerTool, build_consolidated_tools};
use rmcp::model::Tool;
use serde_json::{Map, Value};
use std::sync::Arc;

fn tool(name: &str) -> Tool {
    Tool::new(
        name.to_string(),
        format!("{name} description"),
        Arc::new(Map::<String, Value>::new()),
    )
}

fn tool_with_schema_properties(name: &str, properties: &[&str]) -> Tool {
    let schema_properties = properties
        .iter()
        .map(|property| {
            let mut schema = Map::new();
            schema.insert("type".to_string(), Value::String("string".to_string()));
            (property.to_string(), Value::Object(schema))
        })
        .collect();
    let mut schema = Map::new();
    schema.insert("properties".to_string(), Value::Object(schema_properties));
    Tool::new(
        name.to_string(),
        format!("{name} description"),
        Arc::new(schema),
    )
}

fn tool_names(tools: &[Tool]) -> Vec<String> {
    tools.iter().map(|tool| tool.name.to_string()).collect()
}

fn server_names(tools: &[ConsolidatedServerTool]) -> Vec<String> {
    tools.iter().map(|tool| tool.server_name.clone()).collect()
}

#[test]
#[should_panic]
fn build_panics_on_min_below_two() {
    build_consolidated_tools(vec![], 1).unwrap();
}

#[test]
fn server_with_many_tools_produces_facade() {
    let tools = (0..10)
        .map(|index| tool(&format!("alpha_tool_{index}")))
        .collect();
    let (consolidated, passthrough) =
        build_consolidated_tools(vec![("Alpha Gateway".to_string(), tools)], 4)
            .expect("builder should succeed");

    assert_eq!(consolidated.len(), 1);
    assert!(passthrough.is_empty());
    assert_eq!(consolidated[0].server_name, "Alpha Gateway");
    assert_eq!(consolidated[0].tool_catalog.len(), 10);
}

#[test]
fn small_servers_are_indexed_not_passthrough() {
    let (consolidated, passthrough) = build_consolidated_tools(
        vec![("Small".to_string(), vec![tool("beta"), tool("alpha")])],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(consolidated.len(), 1);
    assert!(passthrough.is_empty());
    assert_eq!(
        tool_names(&consolidated[0].tool_catalog),
        vec!["alpha", "beta"]
    );
}

#[test]
fn already_hierarchical_servers_are_indexed() {
    let tools = vec![
        tool_with_schema_properties("hierarchical", &["command", "parameters"]),
        tool("nested"),
        tool("wrapped"),
        tool("other"),
    ];
    let (consolidated, passthrough) =
        build_consolidated_tools(vec![("Azure".to_string(), tools)], 4)
            .expect("builder should succeed");

    assert_eq!(consolidated.len(), 1);
    assert!(passthrough.is_empty());
    assert_eq!(
        tool_names(&consolidated[0].tool_catalog),
        vec!["hierarchical", "nested", "other", "wrapped"]
    );
}

#[test]
fn empty_servers_are_filtered_out() {
    let (consolidated, passthrough) = build_consolidated_tools(
        vec![
            ("Empty".to_string(), vec![]),
            (
                "Large".to_string(),
                vec![tool("one"), tool("two"), tool("three"), tool("four")],
            ),
        ],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(consolidated.len(), 1);
    assert!(passthrough.is_empty());
    assert_eq!(consolidated[0].server_name, "Large");
}

#[test]
fn server_name_is_preserved() {
    let (consolidated, _) = build_consolidated_tools(
        vec![(
            "My COOL.Server/Name?!".to_string(),
            vec![tool("one"), tool("two"), tool("three"), tool("four")],
        )],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(consolidated[0].server_name, "My COOL.Server/Name?!");
}

#[test]
fn duplicate_legacy_facade_names_do_not_collide() {
    let (consolidated, passthrough) = build_consolidated_tools(
        vec![
            (
                "Alpha-Beta".to_string(),
                vec![tool("one"), tool("two"), tool("three"), tool("four")],
            ),
            (
                "Alpha_Beta".to_string(),
                vec![tool("five"), tool("six"), tool("seven"), tool("eight")],
            ),
        ],
        4,
    )
    .expect("catalog-only consolidation should not create facade collisions");

    assert_eq!(
        server_names(&consolidated),
        vec!["Alpha-Beta", "Alpha_Beta"]
    );
    assert!(passthrough.is_empty());
}

#[test]
fn tool_catalog_ordering_is_deterministic() {
    let forward = build_consolidated_tools(
        vec![(
            "Very Long Gateway Name That Should Still Produce A Stable Description".to_string(),
            vec![
                tool("listUsers"),
                tool("get_user_profile"),
                tool("user-permissions-read"),
                tool("list-user-groups"),
            ],
        )],
        4,
    )
    .expect("builder should succeed");
    let reverse = build_consolidated_tools(
        vec![(
            "Very Long Gateway Name That Should Still Produce A Stable Description".to_string(),
            vec![
                tool("list-user-groups"),
                tool("user-permissions-read"),
                tool("get_user_profile"),
                tool("listUsers"),
            ],
        )],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(
        tool_names(&forward.0[0].tool_catalog),
        tool_names(&reverse.0[0].tool_catalog)
    );
}

#[test]
fn empty_server_name_returns_error() {
    let result = build_consolidated_tools(
        vec![("".to_string(), vec![tool("t1"), tool("t2"), tool("t3")])],
        2,
    );

    assert!(result.is_err());
}

#[test]
fn tool_catalog_preserves_all_commands() {
    let (consolidated, _) = build_consolidated_tools(
        vec![(
            "Learnable".to_string(),
            vec![tool("one"), tool("two"), tool("three"), tool("four")],
        )],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(
        tool_names(&consolidated[0].tool_catalog),
        vec!["four", "one", "three", "two"]
    );
}

#[test]
fn consolidated_catalogs_and_facades_have_stable_ordering() {
    let (consolidated, passthrough) = build_consolidated_tools(
        vec![
            (
                "Zulu".to_string(),
                vec![tool("zeta"), tool("alpha"), tool("beta"), tool("delta")],
            ),
            (
                "Able".to_string(),
                vec![
                    tool("tool_b"),
                    tool("tool_a"),
                    tool("tool_d"),
                    tool("tool_c"),
                ],
            ),
            ("Tiny".to_string(), vec![tool("tiny_b"), tool("tiny_a")]),
        ],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(server_names(&consolidated), vec!["Able", "Tiny", "Zulu"]);
    assert_eq!(
        tool_names(&consolidated[0].tool_catalog),
        vec!["tool_a", "tool_b", "tool_c", "tool_d"]
    );
    assert_eq!(
        tool_names(&consolidated[2].tool_catalog),
        vec!["alpha", "beta", "delta", "zeta"]
    );
    assert_eq!(
        tool_names(&consolidated[1].tool_catalog),
        vec!["tiny_a", "tiny_b"]
    );
    assert!(passthrough.is_empty());
}
