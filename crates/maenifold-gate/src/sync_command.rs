use crate::cache_db::GateCacheDb;
use crate::cli::SyncArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::task_repository::TaskRepository;

#[derive(Debug, Clone)]
pub struct SyncCommand;

impl SyncCommand {
    pub fn execute(args: SyncArgs) -> Result<(), GateError> {
        let tasks = TaskRepository::list_tasks(args.context.root.clone())?;
        let requested = args.context.requested_pipeline_name.as_deref().unwrap_or("sdd");
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
}
