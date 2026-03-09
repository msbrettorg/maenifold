use crate::config::Config;
use crate::error::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub struct GraphDatabase;

// T-0.3f: RTM-047
pub fn open_connection(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_millis(
        Config::SQLITE_BUSY_TIMEOUT_MS,
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
        let _ = conn.execute_batch(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS vec_concepts USING vec0(concept_name TEXT PRIMARY KEY, embedding FLOAT[384]);
            CREATE VIRTUAL TABLE IF NOT EXISTS vec_memory_files USING vec0(file_path TEXT PRIMARY KEY, embedding FLOAT[384]);
            "#,
        );
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
