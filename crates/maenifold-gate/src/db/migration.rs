use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

use super::extract_task_id;

pub(super) fn ensure_task_ids(conn: &Connection) -> Result<()> {
    if !has_task_id_column(conn)? {
        conn.execute("ALTER TABLE tasks ADD COLUMN id TEXT", [])
            .context("failed to add tasks.id column")?;
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_id ON tasks(id)",
            [],
        )
        .context("failed to create unique task id index")?;
    }

    let mut stmt = conn
        .prepare("SELECT file FROM tasks WHERE id IS NULL OR id = ''")
        .context("failed to prepare task id backfill query")?;
    let files = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .context("failed to load tasks missing ids")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to decode files missing ids")?;
    drop(stmt);

    for file in files {
        conn.execute(
            "UPDATE tasks SET id = ?1 WHERE file = ?2",
            params![extract_task_id(&file), file],
        )
        .with_context(|| format!("failed to backfill task id for '{file}'"))?;
    }
    Ok(())
}

fn has_task_id_column(conn: &Connection) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM pragma_table_info('tasks') WHERE name = 'id' LIMIT 1",
            [],
            |_| Ok(()),
        )
        .optional()
        .context("failed to inspect tasks schema")?
        .is_some())
}
