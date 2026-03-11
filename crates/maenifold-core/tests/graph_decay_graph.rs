// T-0.3: RTM NFR-4
#[path = "graph_decay/support.rs"]
mod support;

use support::{
    TEST_FOLDER, TEST_MUTEX, build_context_json, create_test_file_with_age, find_result,
    find_similar_concepts_markdown, isolated_env, read_memory_markdown, search_memories_json,
    sync_graph,
};

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn build_context_newer_related_concepts_rank_higher() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-api-patterns.md"),
        "Old API Patterns",
        "This discusses [[api-design]] with [[rest-architecture]] and [[old-pattern]] legacy approaches.",
        60,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-api-patterns.md"),
        "New API Patterns",
        "Modern [[api-design]] using [[graphql]] and [[new-pattern]] contemporary approaches.",
        2,
    );

    sync_graph(&env);

    let context_result = build_context_json(&env, "api-design");
    let relations = context_result["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations array");

    let new_index = relations
        .iter()
        .position(|entry| entry["Name"].as_str() == Some("new-pattern"));
    let old_index = relations
        .iter()
        .position(|entry| entry["Name"].as_str() == Some("old-pattern"));

    if let (Some(new_index), Some(old_index)) = (new_index, old_index) {
        assert!(
            new_index < old_index,
            "newer related concept should rank ahead of older one: {context_result}"
        );
    } else {
        panic!("expected both new-pattern and old-pattern in build context: {context_result}");
    }
}

// T-GRAPH-DECAY-001.1: RTM FR-7.5
#[test]
fn find_similar_concepts_newer_concepts_rank_higher() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/old-framework.md"),
        "Old Framework Doc",
        "This covers [[legacy-framework]] for [[software-architecture]] applications.",
        60,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/new-framework.md"),
        "New Framework Doc",
        "This covers [[modern-framework]] for [[software-architecture]] applications.",
        2,
    );

    sync_graph(&env);

    let similar_result = find_similar_concepts_markdown(&env, "software-architecture");

    assert!(
        !similar_result.contains("ERROR"),
        "FindSimilarConcepts should succeed: {similar_result}"
    );

    let modern_index = similar_result.find("modern-framework");
    let legacy_index = similar_result.find("legacy-framework");
    if let (Some(modern_index), Some(legacy_index)) = (modern_index, legacy_index) {
        assert!(
            modern_index < legacy_index,
            "newer framework concept should rank ahead of legacy concept: {similar_result}"
        );
    } else {
        panic!("expected both framework concepts in similar concepts output: {similar_result}");
    }
}

// T-GRAPH-DECAY-001.5: RTM NFR-7.5.4
#[test]
fn read_memory_bypasses_decay_and_returns_full_content_regardless_of_age() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/ancient-doc.md"),
        "Ancient Document",
        "This is the full content with [[decay-testing]] concepts that should be fully retrievable.",
        365,
    );

    let uri = format!("memory://{TEST_FOLDER}/ancient-doc");
    let read_result = read_memory_markdown(&env, &uri);

    assert!(!read_result.starts_with("ERROR:"), "{read_result}");
    assert!(read_result.contains("Ancient Document"), "{read_result}");
    assert!(read_result.contains("[[decay-testing]]"), "{read_result}");
    assert!(read_result.contains("fully retrievable"), "{read_result}");
}

// T-GRAPH-DECAY-001.5: RTM NFR-7.5.4
#[test]
fn read_memory_decayed_content_remains_fully_retrievable() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let decayed_uri = create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/decayed-file.md"),
        "Decayed File",
        "Content about [[retrieval-testing]] for verification.",
        90,
    );
    create_test_file_with_age(
        &env.cfg,
        &format!("{TEST_FOLDER}/fresh-file.md"),
        "Fresh File",
        "Content about [[retrieval-testing]] for verification.",
        1,
    );

    sync_graph(&env);

    let search_result = search_memories_json(&env, "retrieval testing", "Hybrid");
    let read_result = read_memory_markdown(&env, &decayed_uri);

    assert!(read_result.contains("Decayed File"), "{read_result}");
    assert!(
        read_result.contains("[[retrieval-testing]]"),
        "{read_result}"
    );
    assert!(
        find_result(&search_result, "Decayed File").is_object(),
        "decayed file should remain searchable: {search_result}"
    );
    assert!(
        find_result(&search_result, "Fresh File").is_object(),
        "fresh file should remain searchable: {search_result}"
    );
}
