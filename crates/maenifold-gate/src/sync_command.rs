use std::fs;
use std::path::Path;

use crate::cache_db::GateCacheDb;
use crate::cli::SyncArgs;
use crate::error::GateError;
use crate::gate_tool_service::GateToolService;
use crate::task_repository::TaskRepository;

#[derive(Debug, Clone)]
pub struct SyncCommand;

impl SyncCommand {
    pub fn execute(args: SyncArgs) -> Result<(), GateError> {
        let _ = GateToolService::sync(args)?;
        Ok(())
    }
}

pub(crate) fn sync_gate(args: SyncArgs) -> Result<(), GateError> {
    ensure_gate_gitignore_entry(&args.context.root)?;
    let tasks = TaskRepository::list_tasks(args.context.root.clone())?;

    if args.rebuild {
        GateCacheDb::rebuild(&args.context.root, &tasks)?;
    } else {
        GateCacheDb::sync(&args.context.root, &tasks)?;
    }

    Ok(())
}

fn ensure_gate_gitignore_entry(root: &Path) -> Result<(), GateError> {
    let path = root.join(".gitignore");
    let existing = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(GateError::Io(error)),
    };

    if existing
        .lines()
        .any(|line| matches!(line.trim(), ".gate/" | ".gate"))
    {
        return Ok(());
    }

    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(".gate/\n");
    fs::write(path, updated)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;

    use crate::cli::GateContext;

    use super::{GateCacheDb, SyncArgs, sync_gate};

    #[test]
    fn sync_and_sync_rebuild_preserve_existing_db_files() {
        let temp = tempfile::tempdir().expect("create temp root");
        let root = temp.path();
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");
        fs::write(
            root.join("todo").join("T-1.1.md"),
            "# T-1.1 — Auth Login JWT\n\n**Pipeline**: sdd\n\n## Acceptance Criteria\n\n1. valid login returns token\n",
        )
        .expect("write task");

        let db_path = GateCacheDb::path_for_root(root);
        fs::create_dir_all(db_path.parent().expect("db parent")).expect("create .gate");
        let connection = Connection::open(&db_path).expect("create sqlite db");
        connection
            .execute("CREATE TABLE sentinel (value TEXT NOT NULL)", [])
            .expect("create sentinel table");
        connection
            .execute("INSERT INTO sentinel (value) VALUES ('kept')", [])
            .expect("insert sentinel row");
        connection
            .execute_batch("PRAGMA user_version = 2;")
            .expect("stamp current schema version so migration does not fire");
        drop(connection);

        sync_gate(SyncArgs {
            context: GateContext {
                root: root.to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            rebuild: false,
        })
        .expect("plain sync should succeed");

        let connection = Connection::open(&db_path).expect("reopen sqlite db");
        let sentinel_value: String = connection
            .query_row("SELECT value FROM sentinel", [], |row| row.get(0))
            .expect("sentinel table should survive plain sync");
        assert_eq!(sentinel_value, "kept");
        drop(connection);

        sync_gate(SyncArgs {
            context: GateContext {
                root: root.to_path_buf(),
                requested_pipeline_name: None,
                json_output: false,
            },
            rebuild: true,
        })
        .expect("sync --rebuild should succeed");

        let connection = Connection::open(&db_path).expect("reopen rebuilt sqlite db");
        let sentinel_value: String = connection
            .query_row("SELECT value FROM sentinel", [], |row| row.get(0))
            .expect("sentinel table should survive sync --rebuild");
        assert_eq!(sentinel_value, "kept");
    }
}
