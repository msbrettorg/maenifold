use crate::error::Result;
use crate::vector;
use rusqlite::{Connection, params};

const MODEL_METADATA_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS model_metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
"#;

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
pub fn ensure_vec0_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(MODEL_METADATA_TABLE_SQL)?;

    let Some(current_dims) = vector::embedding_dimensions() else {
        return Ok(());
    };

    let stored_dims = get_model_metadata(conn, "embedding_dimensions")
        .and_then(|value| value.parse::<usize>().ok());
    let vec_concepts_dims = vec0_table_dimensions(conn, "vec_concepts");
    let vec_memory_files_dims = vec0_table_dimensions(conn, "vec_memory_files");
    let schema_matches =
        vec_concepts_dims == Some(current_dims) && vec_memory_files_dims == Some(current_dims);

    if stored_dims != Some(current_dims) || !schema_matches {
        if recreate_vec0_tables(conn, current_dims).is_err() {
            return Ok(());
        }
    }

    upsert_model_metadata(conn, "embedding_dimensions", &current_dims.to_string())?;
    if let Some(model_filename) = vector::model_filename() {
        upsert_model_metadata(conn, "model_filename", &model_filename)?;
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
