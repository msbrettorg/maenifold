use chrono::{Duration, Utc};

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

    assert!(output.contains("created"));
    assert!(metadata_value(&output, "created-test", "created").contains("2026-01-15"));
}

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

    assert!(output.contains("last_accessed"));
    assert!(metadata_value(&output, "accessed-test", "last_accessed").contains("2026-01-20"));
}

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

    assert!(output.contains("decay_weight"));
    assert!((0.0..=1.0).contains(&weight), "unexpected weight: {weight}");
}

#[test]
fn list_memories_sequential_file_uses_7_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(6)).to_rfc3339();
    write_raw_memory(
        &env,
        "thinking/sequential/decay-test/session-grace-test.md",
        &format!(
            "---\ntitle: Sequential Grace Test\ntype: sequential\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Sequential Grace Test\n\nTest content for [[decay-metadata]] sequential tier 7d grace verification.\n"
        ),
    );

    let output = list_memories_output(&env, "thinking/sequential/decay-test");
    let weight = metadata_value(&output, "session-grace-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert_eq!(weight, 1.0, "unexpected output:\n{output}");
}

#[test]
fn list_memories_workflow_file_uses_14_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(10)).to_rfc3339();
    write_raw_memory(
        &env,
        "thinking/workflows/decay-test/workflow-grace-test.md",
        &format!(
            "---\ntitle: Workflow Grace Test\ntype: workflow\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Workflow Grace Test\n\nTest content for [[decay-metadata]] workflow tier 14d grace verification.\n"
        ),
    );

    let output = list_memories_output(&env, "thinking/workflows/decay-test");
    let weight = metadata_value(&output, "workflow-grace-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert_eq!(weight, 1.0, "unexpected output:\n{output}");
}

#[test]
fn list_memories_standard_file_uses_14_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(10)).to_rfc3339();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/standard-grace-test.md",
        &format!(
            "---\ntitle: Standard Grace Test\ntype: memory\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Standard Grace Test\n\nTest content for [[decay-metadata]] standard memory 14d grace verification.\n"
        ),
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");
    let weight = metadata_value(&output, "standard-grace-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert_eq!(weight, 1.0, "unexpected output:\n{output}");
}

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

    assert!(weight < 1.0, "unexpected output:\n{output}");
}

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

    assert!(output.contains("last_accessed"));
    assert!(output.contains("decay_weight"));
    assert!(!metadata_value(&output, "no-accessed-test", "last_accessed").is_empty());
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
