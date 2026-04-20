use crate::error::{MaenifoldError, Result};
use crate::vector;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

const MODEL_METADATA_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS model_metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
"#;

// T-9.2 / T-3.1: RTM FR-8, FR-6 — single source of truth for locating the pre-compiled
// [[vec0]] SQLite extension binary under `assets/native/{platform}/vec0.{ext}`.
// Previously duplicated in `search/vector.rs` and `sync/vector_embeddings.rs`.
pub(crate) fn vec0_extension_path(assets_path: &Path) -> Option<PathBuf> {
    let (platform_dir, filename) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => ("osx-arm64", "vec0.dylib"),
        ("macos", "x86_64") => ("osx-x64", "vec0.dylib"),
        ("linux", "x86_64") => ("linux-x64", "vec0.so"),
        ("linux", "aarch64") => ("linux-arm64", "vec0.so"),
        ("windows", "x86_64") => ("win-x64", "vec0.dll"),
        _ => return None,
    };
    let path = assets_path.join("native").join(platform_dir).join(filename);
    path.exists().then_some(path)
}

// T-9.2 / T-3.1: RTM FR-8, FR-6 — shared helper that loads the [[vec0]] extension into
// a rusqlite connection. Used by `ensure_vec0_tables`, `repair/semantics.rs`,
// `search/vector.rs`, and `similar/vector_query.rs` to avoid duplicated unsafe blocks.
pub(crate) fn load_vec0_extension(conn: &Connection, assets_path: &Path) -> Result<()> {
    if let Some(ext_path) = vec0_extension_path(assets_path) {
        unsafe {
            conn.load_extension_enable()?;
            conn.load_extension(&ext_path, None::<&str>)?;
            conn.load_extension_disable()?;
        }
        Ok(())
    } else {
        Err(MaenifoldError::InvalidArgument(
            "vec0 extension not found in assets directory".to_string(),
        ))
    }
}

// T-9.2 / T-3.1: RTM FR-8, FR-6 — convenience wrapper that returns a bool for callers
// that need a simple available/unavailable check (e.g. sync's `vector_ready` flag).
pub(crate) fn vec0_extension_available(conn: &Connection, assets_path: &Path) -> bool {
    if load_vec0_extension(conn, assets_path).is_err() {
        return false;
    }

    // T-21.2: RTM FR-8 (RTM-149) — verify vec0 table accessibility, not just extension load.
    conn.query_row("SELECT COUNT(*) FROM vec_concepts", [], |row| {
        row.get::<_, i64>(0)
    })
    .is_ok()
}

// T-9.2: RTM FR-8 — expose persisted [[model metadata]] so schema refresh callers can
// compare the current runtime [[embedding]] model against the last initialized vec0 state.
pub fn get_model_metadata(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM model_metadata WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .ok()
}

// T-9.2: RTM FR-8 — vec0 schema dimensions must follow the runtime-discovered
// [[embedding]] size from [[dynamic model loading]] instead of a hardcoded 384.
// `assets_path` is required so the [[vec0]] extension can be loaded before issuing
// CREATE VIRTUAL TABLE DDL (fixes silent failure on fresh databases).
pub fn ensure_vec0_tables(conn: &Connection, assets_path: &Path) -> Result<()> {
    conn.execute_batch(MODEL_METADATA_TABLE_SQL)?;

    // T-21.1: RTM FR-8 (RTM-058, RTM-059, RTM-060) — when the ONNX model is not yet
    // lazily loaded, prefer dimensions from a prior run (stored metadata or existing
    // table schema) over the hardcoded default. This prevents destructive recreation
    // of non-384 vec0 tables on cold start while still creating fresh databases.
    let current_dims = vector::embedding_dimensions().unwrap_or_else(|| {
        get_model_metadata(conn, "embedding_dimensions")
            .and_then(|v| v.parse::<usize>().ok())
            .or_else(|| vec0_table_dimensions(conn, "vec_concepts"))
            .unwrap_or(vector::DEFAULT_EMBEDDING_DIMENSIONS)
    });

    // T-9.2: RTM FR-8 — load the vec0 extension before any DDL that references
    // `USING vec0(...)`, otherwise CREATE VIRTUAL TABLE fails with "no such module: vec0".
    if load_vec0_extension(conn, assets_path).is_err() {
        // Extension not available — skip vec0 table creation gracefully.
        return Ok(());
    }

    // T-202.9: Detect if the active model has changed. If the user activated a different
    // model since the last sync, we must rebuild vectors even if dimensions match
    // (different models with the same dimensionality corrupt the index).
    let stored_model_id = get_model_metadata(conn, "active_model_id");
    let current_model_id = vector::model_id();
    // T-202.9 P0: `vector::model_id()` returns None at cold-start because the
    // ONNX model is loaded lazily. Without the `current_model_id.is_some()` guard,
    // every restart sees stored=Some, current=None → false-positive "model changed"
    // → vec0 tables nuked on every boot. Only treat it as a change when both sides
    // are known and actually differ.
    let model_changed = stored_model_id.is_some()
        && current_model_id.is_some()
        && stored_model_id != current_model_id;

    let stored_dims = get_model_metadata(conn, "embedding_dimensions")
        .and_then(|value| value.parse::<usize>().ok());
    let vec_concepts_dims = vec0_table_dimensions(conn, "vec_concepts");
    let vec_memory_files_dims = vec0_table_dimensions(conn, "vec_memory_files");
    let schema_matches =
        vec_concepts_dims == Some(current_dims) && vec_memory_files_dims == Some(current_dims);

    // Rebuild vectors if: (1) model changed, (2) dimensions differ, or (3) schema mismatch
    if (model_changed || stored_dims != Some(current_dims) || !schema_matches)
        && recreate_vec0_tables(conn, current_dims).is_err()
    {
        return Ok(());
    }

    upsert_model_metadata(conn, "embedding_dimensions", &current_dims.to_string())?;
    if let Some(model_filename) = vector::model_filename() {
        upsert_model_metadata(conn, "model_filename", &model_filename)?;
    }
    // T-202.9: Store the active model ID so we can detect future model changes.
    if let Some(model_id) = current_model_id {
        upsert_model_metadata(conn, "active_model_id", &model_id)?;
    }

    Ok(())
}

fn upsert_model_metadata(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO model_metadata (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn recreate_vec0_tables(conn: &Connection, dims: usize) -> Result<()> {
    let recreate_sql = format!(
        r#"
        SAVEPOINT ensure_vec0_schema;
        DROP TABLE IF EXISTS vec_concepts;
        DROP TABLE IF EXISTS vec_memory_files;
        CREATE VIRTUAL TABLE IF NOT EXISTS vec_concepts USING vec0(
            concept_name TEXT PRIMARY KEY,
            embedding FLOAT[{dims}]
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS vec_memory_files USING vec0(
            file_path TEXT PRIMARY KEY,
            embedding FLOAT[{dims}]
        );
        RELEASE ensure_vec0_schema;
        "#
    );

    match conn.execute_batch(&recreate_sql) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = conn.execute_batch(
                "ROLLBACK TO ensure_vec0_schema;
                 RELEASE ensure_vec0_schema;",
            );
            Err(error.into())
        }
    }
}

fn vec0_table_dimensions(conn: &Connection, table_name: &str) -> Option<usize> {
    let schema_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get(0),
        )
        .ok()?;

    parse_vec0_dimensions(&schema_sql)
}

fn parse_vec0_dimensions(schema_sql: &str) -> Option<usize> {
    let normalized = schema_sql.to_ascii_uppercase();
    let start = normalized.find("FLOAT[")? + "FLOAT[".len();
    let end = normalized[start..].find(']')? + start;
    normalized[start..end].trim().parse::<usize>().ok()
}
