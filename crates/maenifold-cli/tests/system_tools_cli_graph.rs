mod system_tools_cli_support;

// T-12.7: RTM NFR-1 - preserve graph/build-context CLI regression coverage after splitting the monolithic test file.
use rusqlite::Connection;
use serde_json::Value;
use system_tools_cli_support::*;

#[test]
fn sync_and_build_context_cli_surface_real_community_ids_and_siblings() {
    // T-12.7: RTM NFR-1 - preserve the BuildContext CLI contract for real community ids and siblings after the refactor.
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();
    for (relative, content) in [
        (
            "clusters/alpha-1.md",
            r#"---
title: Alpha Cluster 1
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Alpha
Links [[alpha]], [[beta]], and [[gamma]].
"#,
        ),
        (
            "clusters/alpha-2.md",
            r#"---
title: Alpha Cluster 2
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Alpha
Links [[beta]] and [[gamma]].
"#,
        ),
        (
            "clusters/omega-rho.md",
            r#"---
title: Omega Rho
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Omega
Links [[omega]] and [[rho]].
"#,
        ),
        (
            "clusters/omega-neighborhood.md",
            r#"---
title: Omega Neighborhood
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Omega
Links [[omega]], [[rho]], [[sigma]], and [[tau]].
"#,
        ),
        (
            "clusters/lambda-neighborhood.md",
            r#"---
title: Lambda Neighborhood
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Lambda
Links [[lambda]], [[rho]], [[sigma]], and [[tau]].
"#,
        ),
    ] {
        write_markdown_file(&tmp, relative, content);
    }

    let sync_output = maenifold_cmd(&tmp)
        .args(["tool", "Sync", "{}", "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let sync_rendered = render_json_data_or_text(sync_output);
    assert!(
        sync_rendered.contains("COMMUNITIES detected"),
        "expected community summary after sync: {sync_rendered}"
    );

    let connection = Connection::open(tmp.path().join("memory.db")).expect("open db");
    let omega_community_id: i64 = connection
        .query_row(
            "SELECT community_id FROM concept_communities WHERE concept_name = 'omega'",
            [],
            |row| row.get(0),
        )
        .expect("omega community id");
    assert!(
        omega_community_id > 0,
        "expected omega to land outside community 0"
    );

    let build_output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "BuildContext",
            r#"{"conceptName":"omega","depth":1}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let build_value: Value = serde_json::from_slice(&build_output).expect("json");
    let direct_relations = build_value["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations");
    assert!(
        direct_relations
            .iter()
            .any(|relation| relation["CommunityId"] == omega_community_id),
        "expected real community id in BuildContext output: {build_value}"
    );
    assert!(
        build_value["data"]["CommunitySiblings"]
            .as_array()
            .expect("siblings")
            .iter()
            .any(|sibling| sibling["Name"] == "lambda"),
        "expected sibling section in BuildContext output: {build_value}"
    );
}

#[test]
fn build_context_cli_gracefully_degrades_when_community_table_is_empty() {
    // T-12.7: RTM NFR-1 - preserve BuildContext CLI degradation behavior when community tables are absent.
    let _guard = test_mutex().lock().expect("mutex");
    let tmp = isolated_root();
    for (relative, content) in [
        (
            "clusters/omega-rho.md",
            r#"---
title: Omega Rho
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Omega
Links [[omega]] and [[rho]].
"#,
        ),
        (
            "clusters/omega-sigma.md",
            r#"---
title: Omega Sigma
created: 2026-01-01T00:00:00.0000000+00:00
modified: 2026-01-01T00:00:00.0000000+00:00
---
# Omega
Links [[omega]] and [[sigma]].
"#,
        ),
    ] {
        write_markdown_file(&tmp, relative, content);
    }

    maenifold_cmd(&tmp)
        .args(["tool", "Sync", "{}"])
        .assert()
        .code(0);

    let connection = Connection::open(tmp.path().join("memory.db")).expect("open db");
    connection
        .execute("DELETE FROM concept_communities", [])
        .expect("clear concept_communities");

    let output = maenifold_cmd(&tmp)
        .args([
            "tool",
            "BuildContext",
            r#"{"conceptName":"omega","depth":1}"#,
            "--json",
        ])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    let direct_relations = value["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations");
    assert!(
        direct_relations
            .iter()
            .any(|relation| relation["Name"] == "rho"),
        "expected relation output to remain: {value}"
    );
    assert!(
        direct_relations
            .iter()
            .all(|relation| relation["CommunityId"].is_null()),
        "graceful degradation should omit community ids: {value}"
    );
    assert!(
        value["data"]["CommunitySiblings"]
            .as_array()
            .map(|siblings| siblings.is_empty())
            .unwrap_or(false),
        "sibling section should be omitted without persisted communities: {value}"
    );
}
