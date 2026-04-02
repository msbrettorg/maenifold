use std::fs;
use std::path::Path;

use crate::cache_db::GateCacheDb;
use crate::cli::SyncArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
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
    let requested = args
        .context
        .requested_pipeline_name
        .as_deref()
        .unwrap_or("sdd");
    let definition = GateDefinitionCatalog::resolve(&args.context.root, requested)?;

    if args.rebuild {
        GateCacheDb::rebuild(&args.context.root, &definition, &tasks)?;
    } else {
        let db = GateCacheDb::open(&args.context.root)?;
        GateCacheDb::rebuild(&args.context.root, &definition, &tasks)?;
        db.insert_work_log("sync completed")?;
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
