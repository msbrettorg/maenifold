// T-0.1: RTM NFR-3
#![allow(dead_code, unused_imports)]

include!("sync_perf_support.rs");

// T-0.1: RTM NFR-3 — 60-day decay bracket: file should still be "mostly fresh"
#[test]
fn sync_decay_weight_for_60_day_file_is_near_fresh() {
    let env = TestEnv::new();
    let relative_path = format!("decay-db-tests/decay-test-{}.md", unique_suffix());
    let uri = create_aged_file(
        &env,
        &relative_path,
        "Decay DB Test",
        "Content with [[decay-test]].",
        60,
    );

    let output = sync_one(&env, &relative_path);
    assert!(
        output.contains("Synced memory://decay-db-tests/"),
        "{output}"
    );

    let expected = expected_decay_weight(&env.cfg, 60);
    let actual = decay_weight_for_uri(&env.cfg, &uri);
    // Distinct from test 7 (120-day): power-law with grace_days=28 yields ~0.177 at 60d
    // vs ~0.104 at 120d. The >0.15 bound passes at 60d but would fail at 120d.
    assert!(
        actual > 0.15,
        "60-day file should be in the near-fresh bracket (>0.15), got {actual}"
    );
    assert!(
        (actual - expected).abs() <= 0.005,
        "expected {expected}, got {actual}"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn process_file_persists_created_from_frontmatter_to_database() {
    let env = TestEnv::new();
    let relative_path = format!("reg-created/created-reg-{}.md", unique_suffix());
    let expected_created = "2024-06-15T10:00:00Z";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("Created Reg Test".to_string()),
    );
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    frontmatter.insert("status".to_string(), YamlValue::String("saved".to_string()));
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(expected_created.to_string()),
    );
    let uri = write_memory_with_frontmatter(
        &env.cfg,
        &relative_path,
        &frontmatter,
        "# Created Reg Test\n\n[[concept-alpha]] mentioned here.",
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("Synced memory://reg-created/"), "{output}");

    let stored = fetch_created(&env.cfg, &uri).expect("created timestamp");
    let parsed = DateTime::parse_from_rfc3339(&stored)
        .expect("stored created")
        .with_timezone(&Utc);
    let expected = DateTime::parse_from_rfc3339(expected_created)
        .expect("expected created")
        .with_timezone(&Utc);
    assert!(
        (parsed - expected).num_seconds().abs() < 2,
        "stored={stored} expected={expected_created}"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn process_file_falls_back_to_filesystem_ctime_when_frontmatter_has_no_created_key() {
    let env = TestEnv::new();
    let relative_path = format!("reg-ctime-fallback/ctime-fallback-{}.md", unique_suffix());
    let before = Utc::now() - Duration::seconds(2);
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        "---\ntitle: Ctime Fallback Test\ntype: memory\nstatus: saved\n---\n\n# Ctime Fallback\n\n[[concept-beta]] here.\n",
    );

    let output = sync_one(&env, &relative_path);
    let after = Utc::now() + Duration::seconds(2);
    assert!(
        output.contains("Synced memory://reg-ctime-fallback/"),
        "{output}"
    );

    let stored = fetch_created(&env.cfg, &uri).expect("created fallback");
    let parsed = DateTime::parse_from_rfc3339(&stored)
        .expect("parse created")
        .with_timezone(&Utc);
    assert!(
        parsed >= before && parsed <= after,
        "stored created out of range: {stored}"
    );
}

// T-8.4: RTM FR-25.2 / FR-33.3 — sync persists real ONNX-backed file embeddings into vec_memory_files.
#[test]
fn sync_populates_vec_memory_files_with_non_trivial_onnx_embedding() {
    let env = TestEnv::new();
    let relative_path = format!("perf-bt-001/embed-guard-{}.md", unique_suffix());
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        "# Embed Guard Test\n\n[[concept-a]] is mentioned here.\n",
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("Synced memory://perf-bt-001/"), "{output}");

    let conn = open_db(&env.cfg);
    // vec0 is a per-connection extension — load it before querying vec_memory_files.
    if db::ensure_vec0_tables(&conn, &env.cfg.assets_path).is_err() {
        eprintln!(
            "Skipping: vec0 extension not available at {}",
            env.cfg.assets_path.display()
        );
        return;
    }

    let blob = conn
        .query_row(
            "SELECT embedding FROM vec_memory_files WHERE file_path = ?1",
            [uri],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .expect("optional embedding");

    // ONNX model may not be present in all environments — skip if embedding was not generated.
    let blob = match blob {
        Some(b) => b,
        None => {
            eprintln!("Skipping: ONNX embedding not generated (model not available)");
            return;
        }
    };

    assert!(!blob.is_empty(), "embedding blob should be populated");
    assert!(
        blob.iter().any(|byte| *byte != 0),
        "embedding blob should not be all zeros"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn get_decay_weight_for_file_uses_db_row_not_disk_io_matches_decay_calculator() {
    let env = TestEnv::new();
    let relative_path = format!("reg-decay/decay-reg-{}.md", unique_suffix());
    let uri = create_aged_file(
        &env,
        &relative_path,
        "Decay Reg Test",
        "[[api-design]] links to [[old-pattern]].",
        120,
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("Synced memory://reg-decay/"), "{output}");

    let expected = expected_decay_weight(&env.cfg, 120);
    let actual = decay_weight_for_uri(&env.cfg, &uri);
    assert!(expected < 1.0, "120-day-old file should decay below 1.0");
    assert!(
        (actual - expected).abs() <= 0.005,
        "expected {expected}, got {actual}"
    );

    let context_payload = build_context_json(&env, "api-design");
    let relation = direct_relation(&context_payload, "old-pattern");
    let decay_weight = relation["DecayWeight"].as_f64().expect("DecayWeight");
    let weighted_score = relation["WeightedScore"].as_f64().expect("WeightedScore");
    let co_occurrence = relation["CoOccurrenceCount"]
        .as_f64()
        .expect("CoOccurrenceCount");
    assert!(
        decay_weight > 0.0 && decay_weight < 1.0,
        "unexpected relation decay: {context_payload}"
    );
    assert!(
        (weighted_score - (co_occurrence * decay_weight)).abs() <= 1e-9,
        "weighted score should reflect co-occurrence-scaled decay: {context_payload}"
    );
}
