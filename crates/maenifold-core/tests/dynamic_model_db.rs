#[allow(clippy::duplicate_mod)]
#[path = "dynamic_model/support.rs"]
mod dynamic_model_support;

use dynamic_model_support::{
    TEST_MUTEX, TestContext, packaged_assets_root, table_exists, vec0_extension_path,
};
use maenifold_core::{GraphDatabase, db, vector};

// T-9.6: RTM FR-8 / RTM-057 — database setup should always create the model_metadata table.
#[test]
fn t9_6_model_metadata_table_created_after_init() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();

    GraphDatabase::initialize_database(ctx.config()).expect("db init");
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");

    assert!(
        table_exists(&conn, "model_metadata"),
        "model_metadata table should always exist after db init"
    );
}

// T-9.6: RTM FR-8 / RTM-057 — get_model_metadata returns None for keys that were never
// inserted. This is critical: ensure_vec0_tables branches on
// get_model_metadata(...).and_then(|v| v.parse().ok()) returning None. If it silently
// returned a default instead of None, the fallback dimension chain would break.
#[test]
fn t9_6_get_model_metadata_returns_none_for_missing_key() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();

    GraphDatabase::initialize_database(ctx.config()).expect("db init");
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");

    assert!(
        db::get_model_metadata(&conn, "nonexistent").is_none(),
        "get_model_metadata should return None for a key that was never inserted"
    );
    assert!(
        db::get_model_metadata(&conn, "model_filename").is_none(),
        "get_model_metadata should return None for model_filename when no model loaded"
    );
}

// T-9.6: RTM FR-8 / RTM-060 — when Config::assets_path contains no
// native/{platform}/vec0.{ext} binary, initialize_database creates model_metadata
// but skips vec_concepts and vec_memory_files without panicking.
//
// The vec0 tables are absent because config.assets_path is an isolated temp dir with
// no native/ subdirectory; load_vec0_extension returns Err, causing ensure_vec0_tables
// to return early without issuing vec0 DDL.
#[test]
fn t9_6_vec0_tables_skipped_when_extension_unavailable() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();

    vector::cleanup();
    GraphDatabase::initialize_database(ctx.config()).expect("db init without vec0 extension");
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");

    assert!(
        table_exists(&conn, "model_metadata"),
        "model_metadata should still be created even without vec0"
    );
    assert!(
        !table_exists(&conn, "vec_concepts"),
        "vec_concepts should not exist when vec0 extension is unavailable"
    );
    assert!(
        !table_exists(&conn, "vec_memory_files"),
        "vec_memory_files should not exist when vec0 extension is unavailable"
    );
}

// T-9.6: RTM FR-8 / RTM-059 — when the vec0 extension binary is present under
// assets_path/native/{platform}/, ensure_vec0_tables creates both vec_concepts and
// vec_memory_files virtual tables.
#[test]
fn t9_6_vec0_tables_created_when_extension_available() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();

    let assets = packaged_assets_root();
    if vec0_extension_path(&assets).is_none() {
        eprintln!(
            "Skipping: vec0 extension not available at {}",
            assets.display()
        );
        return;
    }

    vector::cleanup();
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");
    db::ensure_vec0_tables(&conn, &assets).expect("ensure_vec0_tables with extension");

    assert!(
        table_exists(&conn, "model_metadata"),
        "model_metadata should be created"
    );
    assert!(
        table_exists(&conn, "vec_concepts"),
        "vec_concepts should be created when vec0 extension is available"
    );
    assert!(
        table_exists(&conn, "vec_memory_files"),
        "vec_memory_files should be created when vec0 extension is available"
    );
}

// T-9.6: RTM FR-8 / RTM-057 — ensure_vec0_tables writes the runtime embedding
// dimensions to model_metadata through the production upsert path, not via manual
// INSERT. With no model loaded, the default dimension (384) should be persisted.
#[test]
fn t9_6_ensure_vec0_tables_writes_embedding_dimensions_metadata() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();

    let assets = packaged_assets_root();
    if vec0_extension_path(&assets).is_none() {
        eprintln!(
            "Skipping: vec0 extension not available at {}",
            assets.display()
        );
        return;
    }

    // No model loaded → ensure_vec0_tables falls back to DEFAULT_EMBEDDING_DIMENSIONS.
    vector::cleanup();
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");
    db::ensure_vec0_tables(&conn, &assets).expect("ensure_vec0_tables");

    let expected = vector::DEFAULT_EMBEDDING_DIMENSIONS.to_string();
    assert_eq!(
        db::get_model_metadata(&conn, "embedding_dimensions").as_deref(),
        Some(expected.as_str()),
        "ensure_vec0_tables should persist the runtime embedding dimensions via the production upsert path"
    );
}
