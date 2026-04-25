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

fn tool_names(tools: &[Tool]) -> Vec<String> {
    tools.iter().map(|tool| tool.name.to_string()).collect()
}

fn facade_names(tools: &[ConsolidatedServerTool]) -> Vec<String> {
    tools.iter()
        .map(|tool| tool.facade_tool.name.to_string())
        .collect()
}

fn learnless_description(tool: &Tool) -> String {
    tool.description
        .as_ref()
        .expect("facade should have description")
        .trim_end_matches(" Use learn=true to discover available commands.")
        .to_string()
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

    let facade = &consolidated[0].facade_tool;
    assert_eq!(facade.name.to_string(), "gateway_alpha_gateway");
    let schema = Value::Object((*facade.input_schema).clone());
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["intent"]["description"],
        "Natural language description of what you want to do"
    );
    assert_eq!(
        schema["properties"]["command"]["description"],
        "Exact tool name to invoke"
    );
    assert_eq!(
        schema["properties"]["learn"]["description"],
        "Discovery mode - returns available commands"
    );
    assert_eq!(
        schema["properties"]["parameters"]["description"],
        "Arguments forwarded to the selected tool"
    );
}

#[test]
fn small_servers_pass_through_flat_tools() {
    let (consolidated, passthrough) = build_consolidated_tools(
        vec![("Small".to_string(), vec![tool("beta"), tool("alpha")])],
        4,
    )
    .expect("builder should succeed");

    assert!(consolidated.is_empty());
    assert_eq!(tool_names(&passthrough), vec!["alpha", "beta"]);
}

#[test]
fn empty_servers_are_filtered_out() {
    let (consolidated, passthrough) = build_consolidated_tools(
        vec![
            ("Empty".to_string(), vec![]),
            ("Large".to_string(), vec![tool("one"), tool("two"), tool("three"), tool("four")]),
        ],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(consolidated.len(), 1);
    assert!(passthrough.is_empty());
    assert_eq!(consolidated[0].server_name, "Large");
}

#[test]
fn facade_name_is_sanitized() {
    let (consolidated, _) = build_consolidated_tools(
        vec![(
            "My COOL.Server/Name?!".to_string(),
            vec![tool("one"), tool("two"), tool("three"), tool("four")],
        )],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(
        consolidated[0].facade_tool.name.to_string(),
        "gateway_my_cool_server_name__"
    );
}

#[test]
fn duplicate_sanitized_facade_names_fail() {
    let error = build_consolidated_tools(
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
    .expect_err("sanitized facade collision should fail");

    assert_eq!(
        error.to_string(),
        "Gateway facade name collision: 'gateway_alpha_beta' is produced by both 'Alpha-Beta' and 'Alpha_Beta'"
    );
}

#[test]
fn description_is_deterministic_and_bounded() {
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

    let left = &forward.0[0].facade_tool;
    let right = &reverse.0[0].facade_tool;
    assert_eq!(left.description, right.description);
    assert!(left.description.as_ref().expect("description").chars().count() <= 200);
    assert!(learnless_description(left).chars().count() < 200);
}

#[test]
fn empty_server_name_returns_error() {
    let result = build_consolidated_tools(
        vec![(
            "".to_string(),
            vec![tool("t1"), tool("t2"), tool("t3")],
        )],
        2,
    );

    assert!(result.is_err());
}

#[test]
fn description_ends_with_learn_hint() {
    let (consolidated, _) = build_consolidated_tools(
        vec![(
            "Learnable".to_string(),
            vec![tool("one"), tool("two"), tool("three"), tool("four")],
        )],
        4,
    )
    .expect("builder should succeed");

    let description = consolidated[0]
        .facade_tool
        .description
        .as_ref()
        .expect("facade should have description");
    assert!(description.ends_with(" Use learn=true to discover available commands."));
}

#[test]
fn consolidated_catalogs_and_facades_have_stable_ordering() {
    let (consolidated, passthrough) = build_consolidated_tools(
        vec![
            ("Zulu".to_string(), vec![tool("zeta"), tool("alpha"), tool("beta"), tool("delta")]),
            ("Able".to_string(), vec![tool("tool_b"), tool("tool_a"), tool("tool_d"), tool("tool_c")]),
            ("Tiny".to_string(), vec![tool("tiny_b"), tool("tiny_a")]),
        ],
        4,
    )
    .expect("builder should succeed");

    assert_eq!(consolidated[0].server_name, "Able");
    assert_eq!(consolidated[1].server_name, "Zulu");
    assert_eq!(
        tool_names(&consolidated[0].tool_catalog),
        vec!["tool_a", "tool_b", "tool_c", "tool_d"]
    );
    assert_eq!(
        tool_names(&consolidated[1].tool_catalog),
        vec!["alpha", "beta", "delta", "zeta"]
    );
    assert_eq!(
        facade_names(&consolidated),
        vec!["gateway_able", "gateway_zulu"]
    );
    assert_eq!(tool_names(&passthrough), vec!["tiny_a", "tiny_b"]);
}
