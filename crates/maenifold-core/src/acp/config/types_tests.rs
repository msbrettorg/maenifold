#[cfg(test)]
mod tests {
    use crate::acp_config::McpServerEntry;
    use agent_client_protocol as acp;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn maenifold_http_mcp_uses_ipv4_loopback_for_acp_agents() {
        let entry = McpServerEntry::Http {
            url: "http://localhost:8080/mcp".to_string(),
            headers: HashMap::new(),
            auth: None,
            gateway: false,
        };

        let acp::McpServer::Http(server) = entry.to_acp("maenifold") else {
            panic!("expected HTTP MCP server");
        };

        assert_eq!(server.url.as_str(), "http://127.0.0.1:8080/mcp");
    }

    #[test]
    fn non_maenifold_http_mcp_keeps_configured_hostname() {
        let entry = McpServerEntry::Http {
            url: "http://localhost:9999/mcp".to_string(),
            headers: HashMap::new(),
            auth: None,
            gateway: false,
        };

        let acp::McpServer::Http(server) = entry.to_acp("other-local") else {
            panic!("expected HTTP MCP server");
        };

        assert_eq!(server.url.as_str(), "http://localhost:9999/mcp");
    }

    #[test]
    fn mcp_servers_schema_deserializes_object_keyed_entries_without_body_names() {
        let entries: HashMap<String, McpServerEntry> = serde_json::from_value(json!({
            "workiq": {
                "command": "/usr/bin/env",
                "args": ["agency", "mcp", "workiq"],
                "env": { "WORKIQ_MODE": "test" },
                "gateway": true
            },
            "learn": {
                "url": "https://learn.microsoft.com/api/mcp",
                "gateway": true
            }
        }))
        .expect("object-keyed mcpServers should deserialize");

        let workiq = entries.get("workiq").expect("workiq entry");
        match workiq {
            McpServerEntry::Stdio {
                command,
                args,
                env,
                gateway,
                ..
            } => {
                assert_eq!(command, "/usr/bin/env");
                assert_eq!(
                    args,
                    &vec![
                        "agency".to_string(),
                        "mcp".to_string(),
                        "workiq".to_string()
                    ]
                );
                assert_eq!(env.get("WORKIQ_MODE").map(String::as_str), Some("test"));
                assert!(*gateway);
            }
            other => panic!("workiq should discriminate as stdio, got {other:?}"),
        }

        let learn = entries.get("learn").expect("learn entry");
        match learn {
            McpServerEntry::Http { url, gateway, .. } => {
                assert_eq!(url, "https://learn.microsoft.com/api/mcp");
                assert!(*gateway);
            }
            other => panic!("learn should discriminate as http, got {other:?}"),
        }

        let serialized = serde_json::to_value(&entries).expect("serialize object-keyed entries");
        assert!(
            serialized
                .get("workiq")
                .and_then(|entry| entry.get("name"))
                .is_none(),
            "entry body must not serialize a name field"
        );
        assert!(
            serialized
                .get("learn")
                .and_then(|entry| entry.get("name"))
                .is_none(),
            "entry body must not serialize a name field"
        );
    }

    #[test]
    fn mcp_servers_schema_rejects_entry_body_name() {
        let error = serde_json::from_value::<HashMap<String, McpServerEntry>>(json!({
            "workiq": {
                "name": "not-allowed",
                "command": "/usr/bin/env",
                "args": ["agency", "mcp", "workiq"]
            }
        }))
        .expect_err("entry name must be the map key only");

        assert!(
            error.to_string().contains("name"),
            "migration error should mention forbidden name field: {error}"
        );
    }

    #[test]
    fn mcp_servers_schema_rejects_type_discriminator() {
        let error = serde_json::from_value::<HashMap<String, McpServerEntry>>(json!({
            "workiq": {
                "type": "stdio",
                "command": "/usr/bin/env",
                "args": ["agency", "mcp", "workiq"]
            }
        }))
        .expect_err("type discriminator must be removed");

        assert!(
            error.to_string().contains("type"),
            "migration error should mention forbidden type field: {error}"
        );
    }

    #[test]
    fn mcp_servers_schema_rejects_ambiguous_command_and_url_entry() {
        let error = serde_json::from_value::<HashMap<String, McpServerEntry>>(json!({
            "ambiguous": {
                "command": "/usr/bin/env",
                "url": "https://example.invalid/mcp"
            }
        }))
        .expect_err("entry with command and url must be rejected");

        assert!(
            error.to_string().contains("command") && error.to_string().contains("url"),
            "ambiguity error should mention command and url: {error}"
        );
    }

    #[test]
    fn mcp_servers_schema_rejects_entry_without_command_or_url() {
        let error = serde_json::from_value::<HashMap<String, McpServerEntry>>(json!({
            "missing-transport": {
                "gateway": true
            }
        }))
        .expect_err("entry without command or url must be rejected");

        assert!(
            error.to_string().contains("command") && error.to_string().contains("url"),
            "missing transport error should mention command or url: {error}"
        );
    }

    #[test]
    fn mcp_servers_schema_discriminates_by_command_or_url_field_presence() {
        let stdio: McpServerEntry = serde_json::from_value(json!({
            "command": "/usr/bin/env",
            "args": ["agency", "mcp", "workiq"]
        }))
        .expect("command field should select stdio");
        let http: McpServerEntry = serde_json::from_value(json!({
            "url": "https://learn.microsoft.com/api/mcp"
        }))
        .expect("url field should select http");

        assert!(matches!(stdio, McpServerEntry::Stdio { .. }));
        assert!(matches!(http, McpServerEntry::Http { .. }));
        assert!(
            serde_json::to_value(&stdio)
                .expect("serialize stdio")
                .get("type")
                .is_none()
        );
        assert!(
            serde_json::to_value(&http)
                .expect("serialize http")
                .get("type")
                .is_none()
        );
    }

    #[test]
    fn http_auth_config_roundtrips_when_present() {
        let entries: HashMap<String, McpServerEntry> = serde_json::from_value(json!({
            "arm": {
                "url": "https://mcp.management.azure.com",
                "gateway": true,
                "auth": "azure-default"
            }
        }))
        .expect("http auth config should deserialize");

        let serialized = serde_json::to_value(&entries).expect("http auth config should serialize");
        assert_eq!(serialized["arm"]["auth"], json!("azure-default"));
    }
}
