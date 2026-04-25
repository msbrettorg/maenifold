use super::{build_consolidated_tools, gateway_tool_counts};
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

#[test]
fn gateway_tool_counts_include_native_facades_and_passthrough() {
    let (consolidated, passthrough) = build_consolidated_tools(
        vec![
            (
                "Facade".to_string(),
                vec![tool("a"), tool("b"), tool("c"), tool("d")],
            ),
            ("Flat".to_string(), vec![tool("x"), tool("y")]),
        ],
        4,
    )
    .expect("builder should succeed");
    let native_tools = vec![tool("native_one"), tool("native_two"), tool("native_three")];

    assert_eq!(
        gateway_tool_counts(&consolidated, &passthrough, &native_tools),
        super::GatewayToolCounts {
            total: 6,
            facades: 1,
            passthrough: 2,
        }
    );
}
