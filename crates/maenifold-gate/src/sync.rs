use std::path::Path;

use anyhow::{Context, Result};

use crate::{
    CommandStubError, SyncArgs,
    db::{GateDb, TaskUpsert, extract_task_id},
    fs,
    pipeline::{PipelineDef, load_embedded_pipeline},
    registry,
    types::{TaskLocation, TaskState},
};

pub fn run(args: &SyncArgs) -> Result<(), CommandStubError> {
    run_sync(args, &args.root).map_err(|error| CommandStubError::new(error.to_string()))
}

pub fn run_sync(args: &SyncArgs, root: &Path) -> Result<()> {
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
        let pipeline = match load_task_pipeline(&task.path, &content) {
            Ok(pipeline) => pipeline,
            Err(error) => {
                eprintln!("warning: {}: {error}", task.path.display());
                errors += 1;
                continue;
            }
        };
        let state = match resolve_task_state(&task.path, &content, &pipeline) {
            Ok(state) => state,
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
            let id = extract_task_id(file);
            db.upsert_task(&TaskUpsert {
                id: &id,
                file,
                title: title.as_deref(),
                requirement,
                status: state.status.as_str(),
                location: state.location.as_str(),
                attempts: state.attempts as i32,
            })?;
        }
        println!("Rebuilt {} tasks from disk ({} errors)", rows.len(), errors);
        warn_auto_register(root);
        return Ok(());
    }

    let (mut updated, mut unchanged) = (0, 0);
    for (file, title, requirement, state) in rows {
        let status = state.status.as_str();
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
        let id = extract_task_id(&file);
        db.upsert_task(&TaskUpsert {
            id: &id,
            file: &file,
            title: title.as_deref(),
            requirement: &requirement,
            status,
            location,
            attempts: state.attempts as i32,
        })?;
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
    warn_auto_register(root);
    Ok(())
}

pub(crate) fn load_task_pipeline(
    path: &Path,
    content: &str,
) -> Result<PipelineDef> {
    if let Some((yaml_str, _body)) = crate::frontmatter::split_frontmatter(content)
        && let Ok(frontmatter) = serde_yaml::from_str::<crate::frontmatter::Frontmatter>(yaml_str)
        && let Some(name) = frontmatter
            .get("pipeline")
            .and_then(serde_yaml::Value::as_str)
    {
        return load_embedded_pipeline(name);
    }

    anyhow::bail!(
        "{}: missing 'pipeline' field in frontmatter",
        path.display()
    )
}

fn warn_auto_register(root: &Path) {
    if let Err(error) = registry::auto_register(root) {
        eprintln!(
            "warning: failed to auto-register gate root {}: {error}",
            root.display()
        );
    }
}

pub fn extract_title(content: &str) -> Option<String> {
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

pub fn extract_requirement(content: &str) -> Option<String> {
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

/// Resolve task state from frontmatter. No frontmatter = pending.
pub fn resolve_task_state(
    path: &Path,
    content: &str,
    _pipeline: &crate::pipeline::PipelineDef,
) -> Result<TaskState> {
    let location = if path.components().any(|c| c.as_os_str() == "done") {
        TaskLocation::Done
    } else {
        TaskLocation::Todo
    };

    if let Some((yaml_str, _body)) = crate::frontmatter::split_frontmatter(content) {
        let fm: std::collections::BTreeMap<String, serde_yaml::Value> =
            serde_yaml::from_str(yaml_str).unwrap_or_default();
        let status = fm
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string();
        let attempts = fm.get("attempts").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        return Ok(TaskState::new(
            path.to_path_buf(),
            location,
            status,
            attempts,
            Vec::new(),
        ));
    }

    // No frontmatter: file hasn't been migrated yet. Store raw "pending" status
    // regardless of location — the location field tracks where the file lives.
    Ok(TaskState::new(
        path.to_path_buf(),
        location,
        "pending".into(),
        0,
        Vec::new(),
    ))
}
