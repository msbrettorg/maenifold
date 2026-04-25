use std::fs;
use std::path::Path;

use crate::cache_db::GateCacheDb;
use crate::cli::SyncArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::gate_tool_service::GateToolService;
use crate::state_machine::StateMachine;
use crate::task_file::{GateProjectLock, TaskDocument};
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
    let _lock = GateProjectLock::acquire_exclusive(&args.context.root)?;
    sync_gate_unlocked(args)
}

pub(crate) fn sync_gate_unlocked(args: SyncArgs) -> Result<(), GateError> {
    ensure_gate_gitignore_entry(&args.context.root)?;
    recover_terminal_todo_tasks(&args)?;
    let tasks = TaskRepository::list_tasks(args.context.root.clone())?;

    if args.rebuild {
        GateCacheDb::rebuild(&args.context.root, &tasks)?;
    } else {
        GateCacheDb::sync(&args.context.root, &tasks)?;
    }

    Ok(())
}

fn recover_terminal_todo_tasks(args: &SyncArgs) -> Result<(), GateError> {
    let mut tasks = TaskRepository::list_tasks(args.context.root.clone())?;
    for task in tasks.iter_mut().filter(|task| task.location == "todo") {
        let document = match TaskRepository::read_task_document(&args.context.root, task) {
            Ok(document) => document,
            Err(_) => continue,
        };
        let definition = match GateDefinitionCatalog::resolve_for_task(
            &args.context.root,
            &document.metadata,
            args.context.requested_pipeline_name.as_deref(),
        ) {
            Ok(definition) => definition,
            Err(_) => continue,
        };
        let state = match derive_state_without_location_check(&document, &definition) {
            Ok(state) => state,
            Err(_) => continue,
        };
        if definition
            .find_state(&state)
            .is_some_and(|state_definition| state_definition.terminal)
        {
            TaskRepository::move_to_done(&args.context.root, task)?;
        }
    }
    Ok(())
}

fn derive_state_without_location_check(
    document: &TaskDocument,
    definition: &crate::gate_definition::GateDefinition,
) -> Result<String, GateError> {
    let initial = definition.initial_state()?.to_string();
    let mut state = if document.records.is_empty() {
        document
            .metadata
            .state
            .clone()
            .unwrap_or_else(|| initial.clone())
    } else {
        initial.clone()
    };
    for record in &document.records {
        match record.record_kind.as_str() {
            "append" => {}
            "cancel" => state = "cancelled".to_string(),
            "outcome" => {
                let outcome = record.outcome.as_deref().ok_or_else(|| {
                    GateError::TaskMetadataParse("outcome record missing Outcome".to_string())
                })?;
                let current = definition
                    .find_state(&state)
                    .ok_or_else(|| GateError::UnknownState(state.clone()))?;
                if current.terminal {
                    if outcome != "fail" || record.stage != state {
                        return Err(GateError::IllegalTransition {
                            state,
                            outcome: outcome.to_string(),
                        });
                    }
                    state = initial.clone();
                    continue;
                }
                let transition = match outcome {
                    "pass" => StateMachine::next_on_pass(&state, definition)?,
                    "fail" => StateMachine::next_on_fail(&state, definition)?,
                    other => {
                        return Err(GateError::IllegalTransition {
                            state,
                            outcome: other.to_string(),
                        });
                    }
                };
                if transition.acting_stage != record.stage {
                    return Err(GateError::IllegalTransition {
                        state,
                        outcome: outcome.to_string(),
                    });
                }
                state = transition.next_state;
            }
            other => {
                return Err(GateError::TaskMetadataParse(format!(
                    "unknown record kind: {other}"
                )));
            }
        }
    }
    Ok(state)
}

fn ensure_gate_gitignore_entry(root: &Path) -> Result<(), GateError> {
    let path = root.join(".gitignore");
    let existing = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(GateError::Cli(format!(
                "cannot read '{}': {error}; ensure the project root is accessible",
                path.display()
            )));
        }
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
    fs::write(&path, &updated).map_err(|error| {
        GateError::Cli(format!(
            "cannot write '{}': {error}; ensure the project root is writable or pass --root <writable-path>",
            path.display()
        ))
    })?;
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

    #[cfg(unix)]
    #[test]
    fn sync_gate_on_readonly_root_returns_actionable_error() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("create temp root");
        let root = temp.path().join("readonly_sync");
        fs::create_dir_all(root.join("todo")).expect("create todo");
        fs::create_dir_all(root.join("done")).expect("create done");
        // Make root read-only AFTER creating subdirs
        fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).expect("make root read-only");

        let err = sync_gate(SyncArgs {
            context: GateContext {
                root: root.clone(),
                requested_pipeline_name: None,
                json_output: false,
            },
            rebuild: true,
        })
        .expect_err("sync on read-only root should fail");

        let msg = err.to_string();
        assert!(
            msg.contains(&root.to_string_lossy().to_string()) || msg.contains(".gate"),
            "error should contain the path or .gate: {msg}"
        );

        // Cleanup
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).expect("restore permissions");
    }
}
