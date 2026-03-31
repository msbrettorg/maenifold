use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::{Config, GraphDatabase, ToolRegistry};
use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;
use rusqlite::params;
use serde_json::json;
use tempfile::TempDir;

fn isolated_root() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    (tmp, cfg)
}

#[test]
fn build_context_surfaces_related_concepts_after_sync() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _cfg) = isolated_root();

    let registry = ToolRegistry::global();

    let write_a = registry
        .try_invoke(
            "WriteMemory",
            &json!({
                "title": "Authentication Note",
                "content": "This document links [[authentication]] and [[oauth2]]."
            }),
            OutputFormat::Json,
        )
        .expect("write A");
    let write_a_json: serde_json::Value = serde_json::from_str(&write_a).expect("write A json");
    assert_eq!(
        write_a_json["success"], true,
        "seed write A should succeed: {write_a}"
    );

    let write_b = registry
        .try_invoke(
            "WriteMemory",
            &json!({
                "title": "Auth Retry Note",
                "content": "This note links [[authentication]] and [[retry-policy]]."
            }),
            OutputFormat::Json,
        )
        .expect("write B");
    let write_b_json: serde_json::Value = serde_json::from_str(&write_b).expect("write B json");
    assert_eq!(
        write_b_json["success"], true,
        "seed write B should succeed: {write_b}"
    );

    let sync = registry
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync");
    assert!(
        !sync.contains("ERROR"),
        "sync should complete without errors: {sync}"
    );

    let context = registry
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "authentication", "depth": 1 }),
            OutputFormat::Markdown,
        )
        .expect("build context");

    assert!(
        context.contains("oauth2"),
        "expected linked concept missing: {context}"
    );
    assert!(
        context.contains("retry-policy"),
        "expected linked concept missing: {context}"
    );

    Config::reset_overrides();
}

// T-15.6: RTM FR-15 — ExpandedRelations must be a flat string array (C# parity)
#[test]
fn build_context_expanded_relations_is_flat_string_array() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _cfg) = isolated_root();

    let registry = ToolRegistry::global();

    // Seed three notes creating a two-hop chain: auth -> oauth2 -> token-refresh
    for (title, content) in [
        ("Auth Note", "This links [[authentication]] and [[oauth2]]."),
        (
            "OAuth Detail",
            "Details on [[oauth2]] and [[token-refresh]].",
        ),
    ] {
        let write = registry
            .try_invoke(
                "WriteMemory",
                &json!({ "title": title, "content": content }),
                OutputFormat::Json,
            )
            .expect("write");
        let v: serde_json::Value = serde_json::from_str(&write).expect("json");
        assert_eq!(v["success"], true, "seed write should succeed: {write}");
    }

    registry
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("sync");

    // Request JSON output with depth 2 so ExpandedRelations is populated
    let context = registry
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": "authentication", "depth": 2 }),
            OutputFormat::Json,
        )
        .expect("build context");

    let payload: serde_json::Value = serde_json::from_str(&context).expect("json parse");
    let data = &payload["data"];

    // T-15.6: ExpandedRelations must exist and be an array
    let expanded = data["ExpandedRelations"]
        .as_array()
        .expect("ExpandedRelations should be a JSON array");

    // T-15.6: Every element must be a string (not a rich object)
    for (i, element) in expanded.iter().enumerate() {
        assert!(
            element.is_string(),
            "ExpandedRelations[{i}] should be a string, got: {element}"
        );
    }

    // DirectRelations should still be rich objects (not affected by this fix)
    if let Some(direct) = data["DirectRelations"].as_array() {
        for (i, element) in direct.iter().enumerate() {
            assert!(
                element.is_object(),
                "DirectRelations[{i}] should be an object, got: {element}"
            );
            assert!(
                element.get("Name").is_some(),
                "DirectRelations[{i}] should have a Name field"
            );
        }
    }

    Config::reset_overrides();
}

// NOTE: Direct SQL seeding is intentional — these boundary tests isolate the
// decay scorer from the WriteMemory/Sync indexing pipeline. If `file_content`
// schema changes, the `expect("seed")` panic will surface the break immediately.
#[test]
fn search_memories_applies_decay_weight_to_ranking_only() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_root();
    GraphDatabase::initialize_database(&cfg).expect("db init");
    let connection = db::open_connection(&cfg.database_path).expect("db open");
    let now = chrono::Utc::now();
    let stale = (now - chrono::Duration::days(40)).to_rfc3339();
    let fresh = now.to_rfc3339();

    for (path, title, last_accessed) in [
        ("memory://notes/stale-note", "Stale Result", stale.as_str()),
        ("memory://notes/fresh-note", "Fresh Result", fresh.as_str()),
    ] {
        connection
            .execute(
                "INSERT INTO file_content (file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed) \
                 VALUES (?1, ?2, ?3, '', ?4, 'saved', 'hash', 10, ?4, ?5)",
                params![
                    path,
                    title,
                    "alpha beta gamma",
                    now.to_rfc3339(),
                    last_accessed
                ],
            )
            .expect("seed file_content");
    }

    let output = ToolRegistry::global()
        .try_invoke(
            "SearchMemories",
            &json!({
                "query": "alpha beta",
                "mode": "Hybrid",
                "pageSize": 10,
                "page": 1,
            }),
            OutputFormat::Json,
        )
        .expect("search memories");
    let payload: serde_json::Value = serde_json::from_str(&output).expect("search output json");
    let results = payload["data"]["results"]
        .as_array()
        .expect("results array");

    assert_eq!(
        results[0]["path"].as_str(),
        Some("memory://notes/fresh-note"),
        "more recently accessed result should rank higher when base scores tie: {output}"
    );
    assert_eq!(
        results
            .iter()
            .map(|result| result["path"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["memory://notes/fresh-note", "memory://notes/stale-note"]
    );
    assert!(
        results.iter().all(|result| result["snippet"]
            .as_str()
            .unwrap_or_default()
            .contains("alpha beta gamma")),
        "decay should affect ranking only, not retrievability: {output}"
    );

    Config::reset_overrides();
}

// NOTE: Direct SQL seeding is intentional — see comment on
// `search_memories_applies_decay_weight_to_ranking_only` for rationale.
#[test]
fn search_memories_preserves_validated_assumptions_from_decay() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_root();
    GraphDatabase::initialize_database(&cfg).expect("db init");
    let connection = db::open_connection(&cfg.database_path).expect("db open");
    let now = chrono::Utc::now();
    let stale = (now - chrono::Duration::days(40)).to_rfc3339();

    for (path, title, status) in [
        (
            "memory://assumptions/validated-note",
            "Validated Result",
            "validated",
        ),
        (
            "memory://assumptions/invalidated-note",
            "Invalidated Result",
            "invalidated",
        ),
    ] {
        connection
            .execute(
                "INSERT INTO file_content (file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed) \
                 VALUES (?1, ?2, ?3, '', ?4, ?5, 'hash', 10, ?4, ?6)",
                params![path, title, "alpha beta gamma", now.to_rfc3339(), status, stale],
            )
            .expect("seed assumption file_content");
    }

    let output = ToolRegistry::global()
        .try_invoke(
            "SearchMemories",
            &json!({
                "query": "alpha beta",
                "mode": "Hybrid",
                "pageSize": 10,
                "page": 1,
            }),
            OutputFormat::Json,
        )
        .expect("search memories");
    let payload: serde_json::Value = serde_json::from_str(&output).expect("search output json");
    let results = payload["data"]["results"]
        .as_array()
        .expect("results array");

    // T-7.8: RTM FR-7.8
    assert_eq!(
        results[0]["path"].as_str(),
        Some("memory://assumptions/validated-note"),
        "validated assumptions should remain exempt from decay-driven reranking: {output}"
    );
    assert!(
        results[0]["decayWeight"].as_f64().unwrap_or_default()
            > results[1]["decayWeight"].as_f64().unwrap_or(1.0),
        "validated assumptions should retain a higher decay weight than invalidated assumptions: {output}"
    );

    Config::reset_overrides();
}
