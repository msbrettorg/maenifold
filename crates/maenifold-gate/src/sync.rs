use std::path::Path;

use anyhow::{Context, Result};

use crate::{
    CommandStubError, SyncArgs,
    db::{GateDb, extract_task_id},
    fs, parser,
    pipeline::load_pipeline_def,
    types::{TaskLocation, TaskState},
};

pub fn run(args: &SyncArgs) -> Result<(), CommandStubError> {
    run_sync(args, &args.root).map_err(|error| CommandStubError::new(error.to_string()))
}

pub fn run_sync(args: &SyncArgs, root: &Path) -> Result<()> {
    let pipeline = load_pipeline_def(args.pipeline_path.as_deref(), root)?;
    let db_path = args.db.clone().unwrap_or_else(|| root.join("gate.db"));
    let mut found = fs::discover_tasks(root)?;
    let mut rows = Vec::new();
    let mut errors = 0;
    found.sort_by(|a, b| a.path.cmp(&b.path));
    for task in found {
        let content = match fs::read_task(&task.path) {
            Ok(content) => content,
            Err(error) => {
                eprintln!("warning: {}: {error}", task.path.display());
                errors += 1;
                continue;
            }
        };
        let state = match parser::parse_task_content_with_pipeline(&task.path, &content, &pipeline)
        {
            Ok(state) => state,
            Err(parser::ParseTaskError::LocationStatusMismatch {
                location: TaskLocation::Done,
                ..
            }) => {
                TaskState::done_override_from_content_with_pipeline(&task.path, &content, &pipeline)
            }
            Err(error) => {
                eprintln!("warning: {}: {error}", task.path.display());
                errors += 1;
                continue;
            }
        };
        let requirement = extract_requirement(&content).unwrap_or_else(|| {
            if task.location == TaskLocation::Todo {
                eprintln!(
                    "warning: {}: missing **Requirement** metadata; using UNKNOWN",
                    task.path.display()
                );
            }
            "UNKNOWN".into()
        });
        rows.push((
            task.path
                .file_name()
                .and_then(|name| name.to_str())
                .context("task path is missing a UTF-8 filename")?
                .to_string(),
            extract_title(&content),
            requirement,
            state,
        ));
    }

    let db = GateDb::open(&db_path)?;
    if args.rebuild {
        db.delete_all_tasks()?;
        for (file, title, requirement, state) in &rows {
            db.upsert_task(
                &extract_task_id(file),
                file,
                title.as_deref(),
                requirement,
                state.effective_status(),
                state.location.as_str(),
                state.attempts as i32,
            )?;
        }
        println!("Rebuilt {} tasks from disk ({} errors)", rows.len(), errors);
        return Ok(());
    }

    let (mut updated, mut unchanged) = (0, 0);
    for (file, title, requirement, state) in rows {
        let status = state.effective_status();
        let location = state.location.as_str();
        let changed = match db.get_task(&file)? {
            Some(existing) => {
                if existing.status != status || existing.location != location {
                    eprintln!(
                        "warning: {}: cache drift {}:{} -> {}:{}",
                        file, existing.location, existing.status, location, status
                    );
                }
                existing.title.as_deref() != title.as_deref()
                    || existing.requirement != requirement
                    || existing.status != status
                    || existing.location != location
                    || existing.attempts != state.attempts as i32
            }
            None => true,
        };
        db.upsert_task(
            &extract_task_id(&file),
            &file,
            title.as_deref(),
            &requirement,
            status,
            location,
            state.attempts as i32,
        )?;
        if changed {
            updated += 1;
        } else {
            unchanged += 1;
        }
    }
    println!(
        "Synced {} tasks ({} updated, {} unchanged, {} errors)",
        updated + unchanged,
        updated,
        unchanged,
        errors
    );
    Ok(())
}

fn extract_title(content: &str) -> Option<String> {
    let body = content
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))?
        .trim();
    Some(
        [" — ", " – ", ": "]
            .into_iter()
            .find_map(|sep| body.split_once(sep).map(|(_, title)| title.trim()))
            .unwrap_or(body)
            .to_string(),
    )
}

fn extract_requirement(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let value = line.trim().strip_prefix("**Requirement**:")?.trim();
        let id = value.split_whitespace().next()?;
        valid_requirement(id).then(|| id.to_string())
    })
}

fn valid_requirement(id: &str) -> bool {
    ["FR-", "NFR-"].into_iter().any(|prefix| {
        id.strip_prefix(prefix)
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    })
}
