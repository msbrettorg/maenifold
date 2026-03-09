use crate::config::Config;
use crate::error::Result;
use crate::vector;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

pub struct GraphDatabase;

const MODEL_METADATA_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS model_metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
"#;

// T-0.3f: RTM-047
pub fn open_connection(path: &Path) -> Result<Connection> {
    // T-10.9: RTM FR-15
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_millis(
        Config::current().sqlite_busy_timeout_ms(),
    ))?;
    Ok(conn)
}

// T-QUAL-SYNC-001.3: RTM FR-25.6
pub fn run_full_maintenance(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "INSERT INTO file_search(file_search) VALUES('optimize');
         INSERT INTO file_search(file_search) VALUES('rebuild');
         PRAGMA wal_checkpoint(TRUNCATE);",
    )?;

    let database_path = file_backed_database_path(conn)?;
    let placeholder = Connection::open_in_memory()?;
    let existing = std::mem::replace(conn, placeholder);
    existing.close().map_err(|(_, error)| error)?;

    {
        let vacuum_conn = open_connection(&database_path)?;
        vacuum_conn.execute_batch("VACUUM;")?;
    }

    let reopened = open_connection(&database_path)?;
    reopened.pragma_update(None, "journal_mode", "WAL")?;
    reopened.pragma_update(None, "synchronous", "NORMAL")?;
    *conn = reopened;
    Ok(())
}

// T-QUAL-SYNC-001.3: RTM FR-25.6
pub fn run_light_maintenance(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "INSERT INTO file_search(file_search) VALUES('optimize');
         PRAGMA wal_checkpoint(PASSIVE);",
    )?;
    Ok(())
}

fn file_backed_database_path(conn: &Connection) -> Result<PathBuf> {
    conn.path()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            crate::error::MaenifoldError::Message(
                "Database maintenance requires a file-backed SQLite connection".to_string(),
            )
        })
}

impl GraphDatabase {
    pub fn initialize_database(config: &Config) -> Result<()> {
        config.ensure_directories()?;
        let conn = open_connection(&config.database_path)?;
        let journal_mode = if std::env::var("CI").ok().as_deref() == Some("true")
            || std::env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true")
        {
            "DELETE"
        } else {
            "WAL"
        };
        conn.pragma_update(None, "journal_mode", journal_mode)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::create_file_tables(&conn)?;
        Self::create_graph_tables(&conn)?;
        Self::create_community_tables(&conn)?;
        Self::create_vector_tables(&conn);
        Self::create_lease_table(&conn)?;
        Self::add_missing_columns(&conn)?;
        Ok(())
    }

    fn create_file_tables(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS file_content (
                file_path TEXT PRIMARY KEY,
                title TEXT,
                content TEXT,
                summary TEXT,
                last_indexed TEXT,
                status TEXT,
                file_md5 TEXT,
                file_size INTEGER,
                created TEXT,
                last_accessed TEXT
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS file_search USING fts5(
                title,
                content,
                summary,
                content=file_content,
                content_rowid=rowid,
                tokenize='unicode61 remove_diacritics 0'
            );
            CREATE TRIGGER IF NOT EXISTS file_content_ai AFTER INSERT ON file_content BEGIN
                INSERT INTO file_search(rowid, title, content, summary)
                VALUES (new.rowid, new.title, new.content, new.summary);
            END;
            CREATE TRIGGER IF NOT EXISTS file_content_ad AFTER DELETE ON file_content BEGIN
                INSERT INTO file_search(file_search, rowid, title, content, summary)
                VALUES('delete', old.rowid, old.title, old.content, old.summary);
            END;
            CREATE TRIGGER IF NOT EXISTS file_content_au AFTER UPDATE ON file_content BEGIN
                INSERT INTO file_search(file_search, rowid, title, content, summary)
                VALUES('delete', old.rowid, old.title, old.content, old.summary);
                INSERT INTO file_search(rowid, title, content, summary)
                VALUES (new.rowid, new.title, new.content, new.summary);
            END;
            "#,
        )?;
        Ok(())
    }

    fn create_graph_tables(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS concepts (
                concept_name TEXT PRIMARY KEY,
                first_seen TEXT,
                occurrence_count INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS concept_mentions (
                concept_name TEXT NOT NULL,
                source_file TEXT NOT NULL,
                mention_count INTEGER DEFAULT 1,
                PRIMARY KEY (concept_name, source_file),
                FOREIGN KEY (concept_name) REFERENCES concepts(concept_name) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_mentions_source_file ON concept_mentions(source_file);
            CREATE TABLE IF NOT EXISTS concept_graph (
                concept_a TEXT NOT NULL,
                concept_b TEXT NOT NULL,
                -- T-4.6: RTM FR-4.4
                co_occurrence_count REAL,
                source_files TEXT,
                PRIMARY KEY (concept_a, concept_b),
                FOREIGN KEY (concept_a) REFERENCES concepts(concept_name) ON DELETE CASCADE,
                FOREIGN KEY (concept_b) REFERENCES concepts(concept_name) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_graph_concept_b ON concept_graph(concept_b);
            "#,
        )?;
        Ok(())
    }

    fn create_community_tables(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS concept_communities (
                concept_name TEXT PRIMARY KEY,
                community_id INTEGER NOT NULL,
                modularity REAL NOT NULL,
                resolution REAL NOT NULL,
                detected_at TEXT NOT NULL,
                FOREIGN KEY (concept_name) REFERENCES concepts(concept_name) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_communities_community_id ON concept_communities(community_id);
            "#,
        )?;
        Ok(())
    }

    fn create_vector_tables(conn: &Connection) {
        let _ = ensure_vec0_tables(conn);
    }

    fn create_lease_table(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sync_leader_lease (
                lock_name TEXT PRIMARY KEY,
                holder_id TEXT NOT NULL,
                epoch INTEGER NOT NULL DEFAULT 1,
                lease_expires_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn add_missing_columns(conn: &Connection) -> Result<()> {
        for sql in [
            "ALTER TABLE file_content ADD COLUMN status TEXT",
            "ALTER TABLE file_content ADD COLUMN file_md5 TEXT",
            "ALTER TABLE file_content ADD COLUMN file_size INTEGER",
            "ALTER TABLE file_content ADD COLUMN last_accessed TEXT",
            "ALTER TABLE file_content ADD COLUMN created TEXT",
            "ALTER TABLE concept_graph ADD COLUMN source_files TEXT",
            "ALTER TABLE concepts ADD COLUMN first_seen TEXT",
            "ALTER TABLE concepts ADD COLUMN occurrence_count INTEGER DEFAULT 0",
        ] {
            let _ = conn.execute(sql, []);
        }
        Ok(())
    }
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
