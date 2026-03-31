#[path = "repair_concepts/support.rs"]
mod repair_concepts_support;

use maenifold_core::ToolRegistry;
use maenifold_core::json::OutputFormat;
use repair_concepts_support::*;
use serde_json::json;

// T-12.0: RTM NFR-1
#[test]
fn community_detection_populates_non_zero_community_count_after_sync() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_community_graph();

    let sync = sync_graph();
    let community_count = count_distinct_communities(&cfg);

    assert!(
        sync.contains("COMMUNITIES detected"),
        "unexpected sync output: {sync}"
    );
    assert!(
        community_count > 0,
        "expected at least one community after sync"
    );
}

// T-12.0: RTM NFR-1
#[test]
fn build_context_shows_real_community_ids_not_hardcoded_zero() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_community_graph();
    let _sync = sync_graph();

    let omega_community_id = load_community_id(&cfg, "omega");
    assert!(
        omega_community_id > 0,
        "expected omega to live outside the first community for this fixture"
    );

    let context = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({"conceptName":"omega","depth":1}),
            OutputFormat::Markdown,
        )
        .expect("invoke BuildContext");

    assert!(
        context.contains(&format!("[community {omega_community_id}]")),
        "expected real persisted community id in BuildContext output: {context}"
    );
    assert!(
        !context.contains("[community 0]"),
        "community id should not be hardcoded to zero for omega fixture: {context}"
    );
}

// T-12.0: RTM NFR-1
#[test]
fn build_context_with_community_data_shows_community_siblings_section() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, _cfg) = isolated_config();
    seed_community_graph();
    let _sync = sync_graph();

    let context = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({"conceptName":"omega","depth":1}),
            OutputFormat::Markdown,
        )
        .expect("invoke BuildContext");

    assert!(
        context.contains("Community siblings"),
        "expected sibling section in BuildContext output: {context}"
    );
    assert!(
        context.contains("lambda"),
        "expected indirect same-community sibling to appear: {context}"
    );
}

// T-12.0: RTM NFR-1
#[test]
fn build_context_gracefully_degrades_with_empty_concept_communities_table() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    seed_community_graph();
    let _sync = sync_graph();
    delete_community_assignments(&cfg);

    let context = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({"conceptName":"omega","depth":1}),
            OutputFormat::Markdown,
        )
        .expect("invoke BuildContext");

    assert!(
        context.contains("rho"),
        "expected relation output to remain available: {context}"
    );
    assert!(
        !context.contains("[community "),
        "graceful degradation should omit absent community ids: {context}"
    );
    assert!(
        !context.contains("Community siblings"),
        "sibling section should be omitted when community table is empty: {context}"
    );
}
