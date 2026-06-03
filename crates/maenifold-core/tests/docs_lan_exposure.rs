use std::path::{Path, PathBuf};

fn docs_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("docs/lan-exposure.md")
}

fn read_doc_or_skip() -> Option<String> {
    if std::env::var("SKIP_DOCS_TEST").ok().as_deref() == Some("1") {
        eprintln!("Skipping docs_lan_exposure tests because SKIP_DOCS_TEST=1");
        return None;
    }

    let path = docs_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => Some(content),
        Err(error) => panic!("failed to read {}: {error}", path.display()),
    }
}

fn section_body(doc: &str, heading: &str) -> Result<String, String> {
    let mut in_section = false;
    let mut body_lines = Vec::new();

    for line in doc.lines() {
        if in_section && line.starts_with("## ") {
            break;
        }

        if line.trim_end() == heading {
            in_section = true;
            continue;
        }

        if in_section {
            body_lines.push(line);
        }
    }

    if !in_section {
        return Err(format!("missing required heading: {heading}"));
    }

    Ok(body_lines.join("\n"))
}

fn assert_section_has_prose(heading: &str) {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    let body = section_body(&doc, heading).unwrap_or_else(|message| panic!("{message}"));
    assert!(
        body.trim().chars().count() >= 100,
        "section {heading} must have at least 100 chars of body text; found {}",
        body.trim().chars().count()
    );
}

fn mcp_curl_lines(body: &str) -> Vec<(usize, &str)> {
    body.lines()
        .enumerate()
        .filter(|(_, line)| line.contains("curl") && line.contains("/mcp"))
        .collect()
}

fn line_context(lines: &[&str], index: usize) -> String {
    let end = usize::min(index + 3, lines.len());
    lines[index..end].join("\n")
}

#[test]
fn threat_model_section_present_and_non_empty() {
    assert_section_has_prose("## Threat model");
}

#[test]
fn architecture_summary_section_present_and_non_empty() {
    assert_section_has_prose("## Architecture summary");
}

#[test]
fn tailscale_setup_section_present_and_non_empty() {
    assert_section_has_prose("## Tailscale setup (operator playbook)");
}

#[test]
fn tailnet_acl_configuration_section_present_and_non_empty() {
    assert_section_has_prose("## Tailnet ACL configuration");
}

#[test]
fn tailnet_lock_fido2_section_present_and_non_empty() {
    assert_section_has_prose("## Tailnet Lock + FIDO2");
}

#[test]
fn maenifold_json_opt_in_section_present_and_non_empty() {
    assert_section_has_prose("## maenifold.json opt-in");
}

#[test]
fn verification_end_to_end_section_present_and_non_empty() {
    assert_section_has_prose("## Verification (end-to-end)");
}

#[test]
fn decommissioning_section_present_and_non_empty() {
    assert_section_has_prose("## Decommissioning");
}

#[test]
fn what_this_does_not_do_section_present_and_non_empty() {
    assert_section_has_prose("## What this does NOT do");
}

#[test]
fn tailscale_setup_mentions_path_scoped_serve() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    let body = section_body(&doc, "## Tailscale setup (operator playbook)")
        .unwrap_or_else(|message| panic!("{message}"));
    assert!(
        body.contains("tailscale serve")
            && body.contains("--set-path=/mcp")
            && body.contains("http://127.0.0.1:8080/mcp"),
        "Tailscale setup section must include the current v1.52+ path-scoped serve command (tailscale serve --bg --set-path=/mcp http://127.0.0.1:8080/mcp)"
    );
}

#[test]
fn tailscale_serve_command_uses_current_v1_syntax() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    let body = section_body(&doc, "## Tailscale setup (operator playbook)")
        .unwrap_or_else(|message| panic!("{message}"));
    let serve_line = body
        .lines()
        .find(|line| line.contains("tailscale serve"))
        .unwrap_or_else(|| {
            panic!("Tailscale setup section must include a tailscale serve command")
        });

    assert!(
        serve_line.contains("--set-path="),
        "tailscale serve command must use current v1.52+ --set-path syntax: {serve_line}"
    );
    assert!(
        serve_line.contains("http://127.0.0.1:8080"),
        "tailscale serve command must target the loopback backend on port 8080: {serve_line}"
    );
}

#[test]
fn tailscale_serve_command_does_not_use_legacy_positional_syntax() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    for heading in [
        "## Tailscale setup (operator playbook)",
        "## Decommissioning",
    ] {
        let body = section_body(&doc, heading).unwrap_or_else(|message| panic!("{message}"));
        assert!(
            !body.contains("https / http://"),
            "{heading} must not use legacy positional tailscale serve syntax"
        );
    }
}

#[test]
fn tailscale_serve_target_only_127_0_0_1() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    let body = section_body(&doc, "## Tailscale setup (operator playbook)")
        .unwrap_or_else(|message| panic!("{message}"));
    let serve_line = body
        .lines()
        .find(|line| line.contains("tailscale serve"))
        .unwrap_or_else(|| {
            panic!("Tailscale setup section must include a tailscale serve command")
        });

    assert!(
        serve_line.contains("http://127.0.0.1:8080/mcp"),
        "tailscale serve command must target http://127.0.0.1:8080/mcp: {serve_line}"
    );
    assert!(
        !serve_line.contains("http://localhost:8080/mcp"),
        "tailscale serve command must not target localhost: {serve_line}"
    );
}

#[test]
fn acl_section_forbids_funnel_grant() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    let body = section_body(&doc, "## Tailnet ACL configuration")
        .unwrap_or_else(|message| panic!("{message}"));
    assert!(body.contains("funnel"), "ACL section must mention funnel");
    assert!(body.contains("no"), "ACL section must explicitly say no");
    assert!(
        body.contains("```json"),
        "ACL section must include a JSON code fence"
    );
}

#[test]
fn opt_in_section_lists_both_camelcase_keys() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    let body = section_body(&doc, "## maenifold.json opt-in")
        .unwrap_or_else(|message| panic!("{message}"));
    assert!(
        body.contains("mcpAllowedHosts"),
        "opt-in section must mention mcpAllowedHosts"
    );
    assert!(
        body.contains("mcpAllowedOrigins"),
        "opt-in section must mention mcpAllowedOrigins"
    );
}

#[test]
fn verification_section_does_not_expect_200_on_bare_get() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    for heading in [
        "## Verification (end-to-end)",
        "## Tailscale setup (operator playbook)",
    ] {
        let body = section_body(&doc, heading).unwrap_or_else(|message| panic!("{message}"));
        let lines: Vec<&str> = body.lines().collect();

        for (index, line) in mcp_curl_lines(&body) {
            let context = line_context(&lines, index);
            let has_allowed_signal = context.contains("-X POST")
                || context.contains("-X OPTIONS")
                || context.contains("405")
                || context.contains("Method Not Allowed");

            assert!(
                has_allowed_signal,
                "{heading} contains an /mcp curl without POST/OPTIONS or 405 context: {line}"
            );
        }
    }
}

#[test]
fn verification_section_includes_post_example_with_jsonrpc_body() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    let body = section_body(&doc, "## Verification (end-to-end)")
        .unwrap_or_else(|message| panic!("{message}"));
    assert!(
        body.contains("-X POST"),
        "verification section must include a POST example for /mcp"
    );
    assert!(
        body.contains("Content-Type: application/json"),
        "verification section must include Content-Type: application/json"
    );
    assert!(
        body.contains("\"jsonrpc\":\"2.0\""),
        "verification section must include a JSON-RPC request body"
    );
}

#[test]
fn local_api_verification_uses_port_8081() {
    let Some(doc) = read_doc_or_skip() else {
        return;
    };

    let body = section_body(&doc, "## Verification (end-to-end)")
        .unwrap_or_else(|message| panic!("{message}"));
    let lines: Vec<&str> = body.lines().collect();
    let mut found_loopback_api_check = false;

    for (index, line) in lines.iter().enumerate() {
        if line.contains("curl") && line.contains("127.0.0.1:8081/api/health") {
            found_loopback_api_check = true;
            let context = line_context(&lines, index);
            assert!(
                context.contains("200"),
                "loopback API verification must expect HTTP 200 near: {line}"
            );
        }
    }

    assert!(
        found_loopback_api_check,
        "verification section must include a loopback curl for 127.0.0.1:8081/api/health"
    );
}
