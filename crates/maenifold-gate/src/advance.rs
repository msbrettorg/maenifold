use std::{fs, path::Path};

use anyhow::{Context, Result, anyhow, bail};

use crate::{
    AdvanceArgs, CommandStubError, StageVerdict, TaskLocation, TaskStage, TaskStatus,
    db::GateDb,
    fs::{move_task, read_task, resolve_task_path},
    parser::parse_task_file,
    validate_advance_contract,
};

pub fn run(args: &AdvanceArgs) -> Result<(), CommandStubError> {
    let root = args.root.clone().map(Ok).unwrap_or_else(|| {
        std::env::current_dir().context("failed to determine current working directory")
    });
    run_advance(
        args,
        &root.map_err(|err| CommandStubError::new(err.to_string()))?,
    )
    .map_err(|err| CommandStubError::new(err.to_string()))
}

pub fn run_advance(args: &AdvanceArgs, root: &Path) -> Result<()> {
    validate_advance_contract(args.stage, args.verdict).map_err(|err| anyhow!(err.to_string()))?;
    let file = task_name(&args.task)?;
    let (path, _) = resolve_task_path(root, file)?;
    let current = parse_task_file(&path).with_context(|| format!("parsing {}", path.display()))?;
    ensure_transition(file, args.stage, current.status)?;

    let notes = args.notes.as_deref().unwrap_or("");
    let mut content = read_task(&path)?;
    content.push_str(&section_for(args.stage, args.verdict, notes));
    fs::write(&path, &content).with_context(|| format!("writing task file {}", path.display()))?;

    let old_status = current.status;
    let new_status = next_status(args.stage, args.verdict);
    let attempts =
        current.attempts + u32::from(args.verdict.is_some_and(StageVerdict::counts_as_attempt));
    let mut location = current.location;

    let action_suffix = match (args.stage, args.verdict) {
        (TaskStage::Blue, Some(StageVerdict::Pass)) => {
            move_task(&path, &root.join("done"))?;
            location = TaskLocation::Done;
            " ✓ moved to done/"
        }
        (TaskStage::Red, Some(StageVerdict::Reject))
        | (TaskStage::Blue, Some(StageVerdict::Fail)) => " ✗ rejected — remains in todo/",
        _ => "",
    };

    update_cache(root, args, file, &content, new_status, location, attempts);
    println!(
        "{file}: {} → {} {}{}",
        old_status.as_str(),
        new_status.as_str(),
        summary_suffix(args.stage, args.verdict),
        action_suffix
    );
    Ok(())
}

fn task_name(task: &std::path::Path) -> Result<&str> {
    task.file_name()
        .and_then(|name| name.to_str())
        .or_else(|| task.to_str())
        .context("task path must be valid UTF-8")
}

fn ensure_transition(file: &str, stage: TaskStage, status: TaskStatus) -> Result<()> {
    let legal = matches!(
        (stage, status),
        (TaskStage::Swe, TaskStatus::Pending)
            | (TaskStage::Red, TaskStatus::Red)
            | (TaskStage::Blue, TaskStatus::Blue)
    );
    if legal {
        Ok(())
    } else {
        bail!(
            "Cannot advance {file} to {}: current status is {}",
            stage.as_str(),
            status.as_str()
        )
    }
}

fn section_for(stage: TaskStage, verdict: Option<StageVerdict>, notes: &str) -> String {
    match (stage, verdict) {
        (TaskStage::Swe, None) => format!("\n## SWE Remediation\n\n{notes}\n"),
        (TaskStage::Red | TaskStage::Blue, Some(verdict)) => format!(
            "\n{}\n\n**Verdict**: {}\n\n{notes}\n",
            stage.canonical_heading(),
            verdict.as_str()
        ),
        _ => unreachable!("validated advance contract"),
    }
}

fn next_status(stage: TaskStage, verdict: Option<StageVerdict>) -> TaskStatus {
    match (stage, verdict) {
        (TaskStage::Swe, None) => TaskStatus::Red,
        (TaskStage::Red, Some(StageVerdict::Pass)) => TaskStatus::Blue,
        (TaskStage::Red, Some(StageVerdict::Reject))
        | (TaskStage::Blue, Some(StageVerdict::Fail)) => TaskStatus::Pending,
        (TaskStage::Blue, Some(StageVerdict::Pass)) => TaskStatus::Done,
        _ => unreachable!("validated advance contract"),
    }
}

fn summary_suffix(stage: TaskStage, verdict: Option<StageVerdict>) -> String {
    match verdict {
        Some(verdict) => format!("[{} {}]", stage.as_str(), verdict.as_str()),
        None => format!("[{}]", stage.as_str()),
    }
}

fn update_cache(
    root: &Path,
    args: &AdvanceArgs,
    file: &str,
    content: &str,
    status: TaskStatus,
    location: TaskLocation,
    attempts: u32,
) {
    let db_path = args.db.clone().unwrap_or_else(|| root.join("pipeline.db"));
    if !db_path.exists() {
        eprintln!(
            "warning: skipping cache update; {} does not exist",
            db_path.display()
        );
        return;
    }

    let outcome = || -> Result<()> {
        let db = GateDb::open(&db_path)?;
        let requirement = requirement_from(content).context("missing **Requirement** metadata")?;
        db.upsert_task(
            file,
            title_from(content),
            requirement,
            status.as_str(),
            location.as_str(),
            attempts as i32,
        )?;
        db.insert_work_log(
            file,
            args.stage.as_str(),
            args.model.as_deref(),
            args.verdict.map(StageVerdict::as_str),
            None,
            args.notes.as_deref(),
        )
    }();

    if let Err(err) = outcome {
        eprintln!("warning: cache update skipped for {file}: {err}");
    }
}

fn title_from(content: &str) -> Option<&str> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
}

fn requirement_from(content: &str) -> Option<&str> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("**Requirement**:").map(str::trim))
        .and_then(|value| value.split_whitespace().next())
}
