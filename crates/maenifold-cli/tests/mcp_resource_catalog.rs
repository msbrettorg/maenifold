// T-10.2: RTM FR-11
//! MCP resource catalog tests split from `mcp_resource_tests.rs`.

mod mcp_resource_support;

use mcp_resource_support::{isolated_root, read_catalog};
use serde_json::Value;

#[test]
fn get_catalog_returns_valid_json_with_all_categories() {
    let tmp = isolated_root();
    let catalog = read_catalog(&tmp);

    assert!(catalog.get("singleAgentWorkflows").is_some());
    assert!(catalog.get("multiAgentWorkflows").is_some());
    assert!(catalog.get("roles").is_some());
    assert!(catalog.get("colors").is_some());
    assert!(catalog.get("perspectives").is_some());
}

#[test]
fn get_catalog_contains_actual_assets() {
    let tmp = isolated_root();
    let catalog = read_catalog(&tmp);

    let single_agent = catalog["singleAgentWorkflows"]
        .as_array()
        .expect("singleAgentWorkflows array");
    assert!(!single_agent.is_empty());

    let multi_agent = catalog["multiAgentWorkflows"]
        .as_array()
        .expect("multiAgentWorkflows array");
    assert!(!multi_agent.is_empty());

    let roles = catalog["roles"].as_array().expect("roles array");
    assert!(!roles.is_empty());

    let colors = catalog["colors"].as_array().expect("colors array");
    assert!(!colors.is_empty());

    let perspectives = catalog["perspectives"]
        .as_array()
        .expect("perspectives array");
    assert!(!perspectives.is_empty());
}

#[test]
fn get_catalog_asset_metadata_is_complete() {
    let tmp = isolated_root();
    let catalog = read_catalog(&tmp);

    let workflows = catalog["singleAgentWorkflows"]
        .as_array()
        .expect("singleAgentWorkflows array");
    let first = &workflows[0];

    let id = first["id"].as_str().expect("workflow should have 'id'");
    assert!(!id.is_empty());

    let name = first["name"].as_str().expect("workflow should have 'name'");
    assert!(!name.is_empty());

    if let Some(emoji) = first.get("emoji").and_then(Value::as_str) {
        assert!(!emoji.is_empty());
    }
    if let Some(desc) = first.get("description").and_then(Value::as_str) {
        assert!(!desc.is_empty());
    }
}
