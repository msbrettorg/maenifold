use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Returns the journal mode appropriate for the current environment.
/// CI environments use DELETE mode (tmpfs has issues with WAL); production uses WAL.
/// Matches C# `SqliteExtensions.JournalMode` logic.
fn journal_mode() -> &'static str {
    // T-15.22: RTM FR-11 — CI environments (GitHub Actions, etc.) use tmpfs for /tmp
    // which has issues with WAL mode. Use DELETE journal mode in CI for reliability.
    if std::env::var("CI").ok().as_deref() == Some("true")
        || std::env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true")
    {
        "DELETE"
    } else {
        "WAL"
    }
}

// T-0.3f: RTM-047
pub fn open_connection(path: &Path) -> Result<Connection> {
    // T-10.9: RTM FR-15
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_millis(
        Config::current().sqlite_busy_timeout_ms(),
    ))?;
    // T-15.11: RTM FR-11 — enable foreign key enforcement on every connection,
    // matching C# GraphDatabase.cs: conn.Execute("PRAGMA foreign_keys=ON")
    conn.pragma_update(None, "foreign_keys", "ON")?;
    // T-15.22: RTM FR-11 — set journal_mode on every connection, not just at init.
    // VACUUM resets journal_mode to DELETE; C# restores WAL via OpenWithWAL() on
    // every connection open (SqliteExtensions.cs). Without this, any connection
    // opened after a VACUUM stays in DELETE mode until the next init.
    conn.pragma_update(None, "journal_mode", journal_mode())?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "wal_autocheckpoint", 256)?;
    conn.pragma_update(None, "journal_size_limit", 67108864)?;
    // T-15.23: RTM FR-11 — C# distinguishes read vs write connections via
    // OpenReadOnly() (sets PRAGMA query_only=ON) vs OpenWithWAL(). The Rust
    // implementation uses a single open_connection() for all callers. This is an
    // intentional simplification: rusqlite's synchronous Connection model doesn't
    // benefit from a read/write pool split the way C#'s per-request ADO.NET
    // connections do. If a read-only path is added later, set query_only=ON there.
    Ok(conn)
}

// T-QUAL-SYNC-001.3: RTM FR-25.6
pub fn run_full_maintenance(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "INSERT INTO file_search(file_search) VALUES('optimize');\n         INSERT INTO file_search(file_search) VALUES('rebuild');\n         PRAGMA wal_checkpoint(TRUNCATE);",
    )?;

    let database_path = file_backed_database_path(conn)?;
    let placeholder = Connection::open_in_memory()?;
    let existing = std::mem::replace(conn, placeholder);
    existing.close().map_err(|(_, error)| error)?;

    {
        let vacuum_conn = open_connection(&database_path)?;
        vacuum_conn.execute_batch("VACUUM;")?;
    }

    // T-15.22: RTM FR-11 — open_connection() now sets journal_mode and synchronous
    // on every connection, so the post-VACUUM restore is automatic.
    let reopened = open_connection(&database_path)?;
    *conn = reopened;
    Ok(())
}

// T-QUAL-SYNC-001.3: RTM FR-25.6
pub fn run_light_maintenance(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "INSERT INTO file_search(file_search) VALUES('optimize');\n         PRAGMA wal_checkpoint(FULL);",
    )?;
    Ok(())
}

fn file_backed_database_path(conn: &Connection) -> Result<PathBuf> {
    conn.path()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            MaenifoldError::Message(
                "Database maintenance requires a file-backed SQLite connection".to_string(),
            )
        })
}
