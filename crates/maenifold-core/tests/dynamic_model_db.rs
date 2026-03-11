#[path = "dynamic_model/support.rs"]
mod dynamic_model_support;

use dynamic_model_support::{TEST_MUTEX, TestContext, table_exists};
use maenifold_core::{GraphDatabase, db, vector};
use rusqlite::params;

// T-9.6: RTM FR-8 / RTM-057 — database setup should always create the model_metadata table,
// and callers should be able to read persisted values through get_model_metadata.
#[test]
fn t9_6_model_metadata_table_created_and_readable() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();

    GraphDatabase::initialize_database(ctx.config()).expect("db init");
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");

    assert!(
        table_exists(&conn, "model_metadata"),
        "model_metadata table should always exist after db init"
    );

    conn.execute(
        "INSERT INTO model_metadata (key, value) VALUES (?1, ?2)",
        params!["model_filename", "test-model.onnx"],
    )
    .expect("insert model filename metadata");
    conn.execute(
        "INSERT INTO model_metadata (key, value) VALUES (?1, ?2)",
        params!["embedding_dimensions", "384"],
    )
    .expect("insert embedding dimension metadata");

    assert_eq!(
        db::get_model_metadata(&conn, "model_filename").as_deref(),
        Some("test-model.onnx")
    );
    assert_eq!(
        db::get_model_metadata(&conn, "embedding_dimensions").as_deref(),
        Some("384")
    );
}

// T-9.6: RTM FR-8 / RTM-060 — with no loaded ONNX model, db initialization should
// not crash, should still create model_metadata, and should skip vec0 virtual tables.
#[test]
fn t9_6_no_model_db_setup_skips_vec0_tables_without_crashing() {
    let _guard = TEST_MUTEX.lock();
    let mut ctx = TestContext::new();
    ctx.hide_packaged_model();

    vector::cleanup();
    GraphDatabase::initialize_database(ctx.config()).expect("db init without model");
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");

    assert!(table_exists(&conn, "model_metadata"));
    assert!(!table_exists(&conn, "vec_concepts"));
    assert!(!table_exists(&conn, "vec_memory_files"));
}
