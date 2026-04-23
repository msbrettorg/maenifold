use super::list;
use serde_json::Value;

#[test]
fn mcp_tool_list_exposes_exactly_eight_gate_tools() {
    let tools = list();
    let names: Vec<_> = tools.iter().map(|tool| tool.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "status",
            "append",
            "pass",
            "fail",
            "cancel",
            "history",
            "task_body",
            "sync"
        ]
    );
}

#[test]
fn every_mcp_tool_schema_property_has_a_description() {
    for tool in list() {
        let properties = tool
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .expect("tool schema should define properties");

        for (property_name, schema) in properties {
            let description = schema
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");
            assert!(
                !description.trim().is_empty(),
                "tool '{}' property '{}' is missing a description",
                tool.name,
                property_name
            );
        }
    }
}

#[test]
fn every_root_parameter_description_matches_runtime_behavior() {
    for tool in list() {
        let Some(root_schema) = tool
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|properties| properties.get("root"))
        else {
            continue;
        };

        let description = root_schema
            .get("description")
            .and_then(Value::as_str)
            .expect("root description should be present");

        assert!(
            description.contains("establishes and stores the MCP session root"),
            "tool '{}' root description should explain storing the session root",
            tool.name
        );
        assert!(
            description.contains("reuses that stored root"),
            "tool '{}' root description should explain stored-root reuse",
            tool.name
        );
        assert!(
            description.contains("call fails"),
            "tool '{}' root description should explain failure without a root",
            tool.name
        );
        assert!(
            !description.contains("Defaults to the current working directory"),
            "tool '{}' root description must not claim '.' defaulting",
            tool.name
        );
    }
}

#[test]
fn tool_descriptions_explain_gate_workflow() {
    let tools = list();
    let status = tools
        .iter()
        .find(|tool| tool.name == "status")
        .expect("status tool should exist");
    assert!(status.description.contains("project task tracker"));
    assert!(
        status
            .description
            .contains("Prefer gate instead of ad-hoc todo lists")
    );
    assert!(
        status
            .description
            .contains("replace SQL todo-table tracking with gate")
    );
    assert!(status.description.contains("default SDD workflow"));
    assert!(
        status
            .description
            .contains("pending -> swe -> red-team -> blue-team -> done")
    );
    assert!(status.description.contains("run sync, then status"));
    assert!(status.description.contains("append, pass, or fail"));

    let append = tools
        .iter()
        .find(|tool| tool.name == "append")
        .expect("append tool should exist");
    assert!(append.description.contains("non-advancing progress"));
    assert!(append.description.contains("current stage"));

    let pass = tools
        .iter()
        .find(|tool| tool.name == "pass")
        .expect("pass tool should exist");
    assert!(pass.description.contains("stage and role transitions"));
    assert!(
        pass.description
            .contains("pending -> swe -> red-team -> blue-team -> done")
    );

    let fail = tools
        .iter()
        .find(|tool| tool.name == "fail")
        .expect("fail tool should exist");
    assert!(fail.description.contains("sends the task back"));

    let history = tools
        .iter()
        .find(|tool| tool.name == "history")
        .expect("history tool should exist");
    assert!(history.description.contains("stage journey"));
    assert!(
        history
            .description
            .contains("JSON array of history records")
    );

    let task_body = tools
        .iter()
        .find(|tool| tool.name == "task_body")
        .expect("task_body tool should exist");
    assert!(
        task_body
            .description
            .contains("authoritative markdown content")
    );
    assert!(
        task_body
            .description
            .contains("without reading task files directly from disk")
    );

    let sync = tools
        .iter()
        .find(|tool| tool.name == "sync")
        .expect("sync tool should exist");
    assert!(
        sync.description
            .contains("authoritative markdown task files")
    );
    assert!(sync.description.contains("project task tracker"));
    assert!(
        sync.description
            .contains("Use gate instead of ad-hoc todo lists")
    );
    assert!(sync.description.contains("SQL todo-table tracking"));
}
