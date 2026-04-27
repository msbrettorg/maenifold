use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::cli::CreateArgs;
use crate::error::GateError;
use crate::gate_catalog::GateDefinitionCatalog;
use crate::task_repository::TaskRepository;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn create_task(args: CreateArgs) -> Result<(), GateError> {
    validate_task_id_format(&args.task_id)?;
    reject_duplicate_task_id(&args)?;
    GateDefinitionCatalog::resolve_by_name(&args.context.root, &args.pipeline)?;
    validate_task_id_format(&args.pipeline).map_err(|_| {
        GateError::Cli(format!(
            "pipeline name '{}' contains invalid characters",
            args.pipeline
        ))
    })?;

    let todo_dir = ensure_todo_dir(&args.context.root)?;
    let destination = todo_dir.join(format!(
        "{}-{}.md",
        args.task_id,
        slug_from_title(&args.title)
    ));
    if destination.exists() {
        return Err(GateError::Cli(format!(
            "cannot create task '{}'; target file already exists at {}",
            args.task_id,
            destination.display()
        )));
    }

    let markdown = task_markdown(&args);
    write_atomically(&destination, &markdown)
}

fn ensure_todo_dir(root: &Path) -> Result<PathBuf, GateError> {
    let todo_dir = root.join("todo");
    fs::create_dir_all(&todo_dir)?;
    let canonical_root = fs::canonicalize(root)?;
    let canonical_todo = fs::canonicalize(&todo_dir)?;
    if canonical_todo.parent() != Some(canonical_root.as_path()) {
        return Err(GateError::Cli(format!(
            "todo directory must be a direct child of gate root; got {}",
            canonical_todo.display()
        )));
    }
    Ok(todo_dir)
}

fn validate_task_id_format(task_id: &str) -> Result<(), GateError> {
    let valid = !task_id.is_empty()
        && !task_id.contains("..")
        && !task_id.contains('/')
        && !task_id.contains('\\')
        && task_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err(GateError::Cli(format!(
            "invalid taskId '{task_id}'; task IDs must match ^[A-Za-z0-9._-]+$ and must not contain '..', '/', or '\\'"
        )))
    }
}

fn reject_duplicate_task_id(args: &CreateArgs) -> Result<(), GateError> {
    for task in TaskRepository::list_tasks(args.context.root.clone())? {
        if task.task_id == args.task_id {
            let path = TaskRepository::canonical_task_markdown_path(&args.context.root, &task)
                .unwrap_or(task.path);
            return Err(GateError::Cli(format!(
                "cannot create duplicate task '{}'; existing task file is in {} at {}",
                args.task_id,
                task.location,
                path.display()
            )));
        }
    }
    Ok(())
}

fn task_markdown(args: &CreateArgs) -> String {
    let title = sanitize_title(&args.title);
    let body = sanitize_body(args.body.as_deref().unwrap_or("").trim());
    let acceptance = if body.is_empty() {
        "- [ ] Define acceptance criteria"
    } else {
        body.as_str()
    };
    format!(
        "---\nid: {}\ntitle: \"{}\"\npipeline: {}\npriority: {}\n---\n\n# {} — {}\n\n**Pipeline**: {}\n\n## Goal\n\n{}\n\n## Acceptance Criteria\n\n{}\n",
        args.task_id,
        yaml_double_quoted(&title),
        args.pipeline,
        sanitize_priority(&args.priority),
        args.task_id,
        title,
        args.pipeline,
        body,
        acceptance
    )
}

fn sanitize_title(title: &str) -> String {
    title
        .chars()
        .filter(|ch| *ch != '\n' && *ch != '\r')
        .collect()
}

fn sanitize_priority(value: &str) -> &str {
    match value.trim() {
        "P0" => "P0",
        "P1" => "P1",
        _ => "P2",
    }
}

fn sanitize_body(body: &str) -> String {
    body.lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("## ") {
                format!("### {rest}")
            } else if is_gate_record_line(line) {
                format!("\\{line}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_gate_record_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- Stage ID:")
        || trimmed.starts_with("- Record Kind:")
        || trimmed.starts_with("- Outcome:")
        || trimmed.starts_with("- Agent:")
        || trimmed.starts_with("- Recorded At:")
        || trimmed.starts_with("- Model:")
        || trimmed.starts_with("- Notes:")
}

fn yaml_double_quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn slug_from_title(title: &str) -> String {
    let mut slug = String::new();
    for ch in title.to_ascii_lowercase().chars() {
        if slug.len() >= 50 {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if ch.is_ascii_whitespace() {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "task".to_string()
    } else {
        slug
    }
}

fn write_atomically(path: &Path, text: &str) -> Result<(), GateError> {
    let parent = path
        .parent()
        .ok_or_else(|| GateError::MissingTaskPath(path.to_path_buf()))?;
    for _ in 0..8 {
        let temp_path = temp_path_for(path)?;
        let result = (|| -> Result<(), GateError> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp_path, path)?;
            sync_directory(parent)?;
            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        match result {
            Ok(()) => return Ok(()),
            Err(GateError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                continue;
            }
            Err(error) => return Err(error),
        }
    }
    Err(GateError::Cli(format!(
        "failed to allocate atomic write temp file for {}",
        path.display()
    )))
}

fn temp_path_for(path: &Path) -> Result<PathBuf, GateError> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| GateError::MissingTaskPath(path.to_path_buf()))?;
    let suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(path.with_file_name(format!(".{file_name}.{suffix}.tmp")))
}

fn sync_directory(path: &Path) -> Result<(), GateError> {
    fs::File::open(path)?.sync_all()?;
    Ok(())
}
