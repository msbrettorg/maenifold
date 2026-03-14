use chrono::{DateTime, Duration, FixedOffset, Utc};

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    DecayEnvGuard, TEST_MUTEX, isolated_env, list_memories_output, metadata_value,
    set_last_accessed, write_raw_memory,
};

// T-12.6: RTM NFR-1
#[test]
fn list_memories_output_includes_created_timestamp() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/created-test.md",
        "---\ntitle: Created Test\ntype: memory\nstatus: saved\ncreated: 2026-01-15T10:30:00Z\nmodified: 2026-01-15T10:30:00Z\n---\n\n# Created Test\n\nTest content for [[decay-metadata]] created timestamp verification.\n",
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");

    assert!(metadata_value(&output, "created-test", "created").contains("2026-01-15"));
}

// T-12.6: RTM NFR-1 — REWRITE: removed redundant output.contains("last_accessed") assertion
#[test]
fn list_memories_output_includes_last_accessed_timestamp() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let uri = write_raw_memory(
        &env,
        "decay-metadata-tests/standard/accessed-test.md",
        "---\ntitle: Accessed Test\ntype: memory\nstatus: saved\ncreated: 2026-01-10T08:00:00Z\nmodified: 2026-01-10T08:00:00Z\n---\n\n# Accessed Test\n\nTest content for [[decay-metadata]] last_accessed timestamp verification.\n",
    );
    set_last_accessed(&env, &uri, "2026-01-20T14:45:00Z");

    let output = list_memories_output(&env, "decay-metadata-tests/standard");

    assert!(metadata_value(&output, "accessed-test", "last_accessed").contains("2026-01-20"));
}

// T-12.6: RTM NFR-1 — REWRITE: removed redundant output.contains("decay_weight") assertion
#[test]
fn list_memories_output_includes_decay_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(7)).to_rfc3339();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/decay-weight-test.md",
        &format!(
            "---\ntitle: Decay Weight Test\ntype: memory\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Decay Weight Test\n\nTest content for [[decay-metadata]] decay_weight verification.\n"
        ),
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");
    let weight = metadata_value(&output, "decay-weight-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert!((0.0..=1.0).contains(&weight), "unexpected weight: {weight}");
}

// T-12.6: RTM NFR-1 — MERGE: collapsed tests 4/5/6 into parameterized grace-period test
#[test]
fn within_grace_period_weight_is_1_0() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();

    // (relative_path, type_value, grace_days, file_stem)
    let cases: &[(&str, &str, i64, &str)] = &[
        (
            "thinking/sequential/decay-test/seq-grace-test.md",
            "sequential",
            7,
            "seq-grace-test",
        ),
        (
            "thinking/workflows/decay-test/wf-grace-test.md",
            "workflow",
            14,
            "wf-grace-test",
        ),
        (
            "decay-metadata-tests/standard/std-grace-test.md",
            "memory",
            14,
            "std-grace-test",
        ),
    ];

    for &(path, type_value, grace_days, stem) in cases {
        let days_within = grace_days - 1; // just inside the grace period
        let created_date = (Utc::now() - Duration::days(days_within)).to_rfc3339();
        let parent = std::path::Path::new(path)
            .parent()
            .unwrap()
            .to_str()
            .unwrap();
        write_raw_memory(
            &env,
            path,
            &format!(
                "---\ntitle: {stem}\ntype: {type_value}\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# {stem}\n\nTest content for [[decay-metadata]] {type_value} tier grace verification.\n"
            ),
        );

        let output = list_memories_output(&env, parent);
        let weight = metadata_value(&output, stem, "decay_weight")
            .parse::<f64>()
            .expect("decay weight float");

        assert_eq!(
            weight, 1.0,
            "{type_value} file {days_within}d old (grace={grace_days}d) should have weight 1.0, got {weight}\noutput:\n{output}"
        );
    }
}

// T-12.6: RTM NFR-1 — REWRITE: tightened decay assertion to expected power-law range
// Power-law formula: weight = effective_age^(-0.5) where effective_age = (age - grace).max(1)
// For sequential (grace=7d), age=14d: effective_age=7, weight=7^(-0.5) ≈ 0.37796
#[test]
fn list_memories_sequential_file_past_grace_period_has_decayed_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(14)).to_rfc3339();
    write_raw_memory(
        &env,
        "thinking/sequential/decay-test/session-decayed-test.md",
        &format!(
            "---\ntitle: Sequential Decayed Test\ntype: sequential\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Sequential Decayed Test\n\nTest content for [[decay-metadata]] sequential tier decay after grace period.\n"
        ),
    );

    let output = list_memories_output(&env, "thinking/sequential/decay-test");
    let weight = metadata_value(&output, "session-decayed-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    // Expected: 7.0_f64.powf(-0.5) = 1/√7 ≈ 0.37796
    let expected = 7.0_f64.powf(-0.5);
    assert!(
        (weight - expected).abs() < 0.02,
        "expected weight ≈ {expected:.5} (power-law, age=14d, grace=7d), got {weight:.5}\noutput:\n{output}"
    );
}

// T-12.6: RTM NFR-1 — REWRITE: removed dead assertions, added ISO 8601 parse validation
#[test]
fn list_memories_file_without_last_accessed_uses_fallback() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(5)).to_rfc3339();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/no-accessed-test.md",
        &format!(
            "---\ntitle: No Accessed Test\ntype: memory\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# No Accessed Test\n\nTest content for [[decay-metadata]] fallback when last_accessed is missing.\n"
        ),
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");
    let fallback = metadata_value(&output, "no-accessed-test", "last_accessed");

    assert!(
        !fallback.is_empty(),
        "fallback last_accessed should not be empty"
    );
    // The fallback may be a full RFC 3339 timestamp or an ISO 8601 date-only string
    let is_valid_iso8601 = DateTime::parse_from_rfc3339(&fallback).is_ok()
        || DateTime::<FixedOffset>::parse_from_str(&fallback, "%Y-%m-%dT%H:%M:%S%.f%:z").is_ok()
        || chrono::NaiveDate::parse_from_str(&fallback, "%Y-%m-%d").is_ok();
    assert!(
        is_valid_iso8601,
        "fallback last_accessed should be a valid ISO 8601 timestamp or date, got: {fallback}"
    );
}

#[test]
fn list_memories_new_file_has_full_decay_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/new-file-test.md",
        "---\ntitle: New File Test\ntype: memory\nstatus: saved\n---\n\n# New File Test\n\nTest content for [[decay-metadata]] new file full weight verification.\n",
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");
    let weight = metadata_value(&output, "new-file-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert_eq!(weight, 1.0, "unexpected output:\n{output}");
}
