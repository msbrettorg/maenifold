use crate::mcp::gateway_consolidation::{ConsolidatedServerTool, build_consolidated_tools};
use crate::mcp::gateway_dispatch::{DispatchResult, dispatch_facade_call};
use rmcp::model::Tool;
use serde_json::{Map, Value, json};
use std::sync::Arc;

const MIN_TOOLS_FOR_CONSOLIDATION: usize = 4;

fn tool(name: &str, description: &str, schema_fields: &[&str]) -> Tool {
    let properties: Map<String, Value> = schema_fields
        .iter()
        .map(|field| {
            (
                (*field).to_string(),
                json!({"type":"string","description":format!("{field} field")}),
            )
        })
        .collect();
    let schema = json!({
        "type": "object",
        "properties": Value::Object(properties),
        "required": schema_fields,
    });
    Tool::new(
        name.to_string(),
        description.to_string(),
        Arc::new(schema.as_object().cloned().unwrap_or_default()),
    )
}

fn billing_gateway_tools() -> Vec<Tool> {
    vec![
        tool("account_get", "Get account details", &["accountId"]),
        tool(
            "account_update",
            "Update account settings",
            &["accountId", "displayName"],
        ),
        tool("alerts_list", "List billing alerts", &["accountId"]),
        tool("budget_set", "Set account budget", &["accountId", "amount"]),
        tool("invoice_get", "Get invoice by id", &["invoiceId"]),
        tool("list_accounts", "List accounts", &["organizationId"]),
        tool("list_invoices", "List invoices", &["accountId"]),
        tool("list_regions", "List regions", &["cloud"]),
        tool(
            "subscription_list",
            "List subscriptions for an account",
            &["accountId"],
        ),
        tool(
            "usage_export",
            "Export usage report",
            &["accountId", "format"],
        ),
    ]
}

fn build_billing_gateway() -> (ConsolidatedServerTool, Vec<Tool>) {
    let (mut consolidated, passthrough) = build_consolidated_tools(
        vec![("Billing Gateway".to_string(), billing_gateway_tools())],
        MIN_TOOLS_FOR_CONSOLIDATION,
    )
    .expect("gateway should consolidate");
    let server = consolidated.pop().expect("expected one facade");
    (server, passthrough)
}

fn dispatch(
    intent: Option<&str>,
    command: Option<&str>,
    learn: bool,
    parameters: Value,
) -> DispatchResult {
    let (server, passthrough) = build_billing_gateway();
    assert!(
        passthrough.is_empty(),
        "large gateway should not passthrough tools"
    );
    dispatch_facade_call(&server, intent, command, learn, parameters)
        .expect("dispatch should succeed")
}

#[test]
fn tool_count_reduction_builds_single_facade() {
    let (server, passthrough) = build_billing_gateway();

    assert_eq!(server.server_name, "Billing Gateway");
    assert_eq!(server.tool_catalog.len(), 10);
    assert!(passthrough.is_empty());
    assert_eq!(
        server.facade_tool.name.to_string(),
        "gateway_billing_gateway"
    );
}

#[test]
fn learn_mode_tier_one_returns_catalog() {
    let DispatchResult::LearnCatalog(catalog) = dispatch(None, None, true, json!({})) else {
        panic!("expected learn catalog");
    };

    assert!(catalog.contains("Available commands for Billing Gateway"));
    assert!(catalog.contains("- subscription_list: List subscriptions for an account"));
    assert!(catalog.contains("- usage_export: Export usage report"));
    assert!(!catalog.contains("accountId"));
}

#[test]
fn learn_mode_tier_two_returns_tool_schema() {
    let result = dispatch(None, Some("budget_set"), true, json!({}));

    assert_eq!(
        result,
        DispatchResult::LearnSchema(json!({
            "type": "object",
            "properties": {
                "accountId": {"type":"string","description":"accountId field"},
                "amount": {"type":"string","description":"amount field"}
            },
            "required": ["accountId", "amount"]
        }))
    );
}

#[test]
fn direct_command_dispatch_targets_upstream_tool() {
    let parameters = json!({"accountId":"acct_123","amount":"25"});
    let result = dispatch(None, Some("budget_set"), false, parameters.clone());

    assert_eq!(
        result,
        DispatchResult::CallTool {
            server_name: "Billing Gateway".to_string(),
            tool_name: "budget_set".to_string(),
            parameters,
        }
    );
}

#[test]
fn intent_matching_dispatches_unambiguous_command() {
    let parameters = json!({"accountId":"acct_123"});
    let result = dispatch(Some("list subscriptions"), None, false, parameters.clone());

    assert_eq!(
        result,
        DispatchResult::CallTool {
            server_name: "Billing Gateway".to_string(),
            tool_name: "subscription_list".to_string(),
            parameters,
        }
    );
}

#[test]
fn ambiguous_intent_returns_candidates() {
    let result = dispatch(Some("list"), None, false, json!({}));

    assert_eq!(
        result,
        DispatchResult::Candidates(vec![
            ("alerts_list".to_string(), "List billing alerts".to_string()),
            ("list_accounts".to_string(), "List accounts".to_string()),
            ("list_invoices".to_string(), "List invoices".to_string()),
            ("list_regions".to_string(), "List regions".to_string()),
            (
                "subscription_list".to_string(),
                "List subscriptions for an account".to_string(),
            ),
        ])
    );
}

#[test]
fn unknown_command_returns_helpful_error() {
    let (server, passthrough) = build_billing_gateway();
    assert!(passthrough.is_empty());
    let error = dispatch_facade_call(&server, None, Some("nonexistent"), false, json!({}))
        .expect_err("unknown command should fail");

    let text = error.to_string();
    assert!(text.contains("Unknown gateway command 'nonexistent'"));
    assert!(text.contains("Billing Gateway"));
    assert!(text.contains("budget_set"));
    assert!(text.contains("learn=true"));
}
