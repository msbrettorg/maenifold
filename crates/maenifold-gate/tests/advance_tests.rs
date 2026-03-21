use std::fs;
use std::path::Path;
use tempfile::tempdir;

use maenifold_gate::advance::run_advance;
use maenifold_gate::fs::read_task;
use maenifold_gate::{AdvanceArgs, StageVerdict, TaskStage};

fn create_pending_task(root: &Path, name: &str) -> String {
    let todo = root.join("todo");
    fs::create_dir_all(&todo).unwrap();
    let task = todo.join(name);
    let content = format!("# Test Task\n\n**Requirement**: FR-1\n");
    fs::write(&task, &content).unwrap();
    content
}

#[test]
fn advance_full_lifecycle() {
    let root = tempdir().unwrap();
    let file = "task.md";
    create_pending_task(root.path(), file);

    // SWE advance
    let args = AdvanceArgs {
        task: file.into(),
        stage: TaskStage::Swe,
        verdict: None,
        notes: Some("Implemented".into()),
        model: None,
        root: Some(root.path().into()),
        db: None,
    };
    run_advance(&args, root.path()).unwrap();
    let content = read_task(&root.path().join("todo").join(file)).unwrap();
    assert!(content.contains("## SWE Remediation"));
    assert!(content.contains("Implemented"));

    // Red PASS advance
    let args = AdvanceArgs {
        stage: TaskStage::Red,
        verdict: Some(StageVerdict::Pass),
        notes: Some("Looks good".into()),
        ..args
    };
    run_advance(&args, root.path()).unwrap();
    let content = read_task(&root.path().join("todo").join(file)).unwrap();
    assert!(content.contains("## Red-team Audit"));
    assert!(content.contains("**Verdict**: PASS"));

    // Blue PASS advance (moves to done/)
    let args = AdvanceArgs {
        stage: TaskStage::Blue,
        verdict: Some(StageVerdict::Pass),
        notes: Some("Verified".into()),
        ..args
    };
    run_advance(&args, root.path()).unwrap();
    assert!(!root.path().join("todo").join(file).exists());
    assert!(root.path().join("done").join(file).exists());
    let content = read_task(&root.path().join("done").join(file)).unwrap();
    assert!(content.contains("## Blue-team Verification"));
    assert!(content.contains("**Verdict**: PASS"));
}

#[test]
fn advance_illegal_transition() {
    let root = tempdir().unwrap();
    let file = "task.md";
    create_pending_task(root.path(), file);

    // Cannot advance pending task to red
    let args = AdvanceArgs {
        task: file.into(),
        stage: TaskStage::Red,
        verdict: Some(StageVerdict::Pass),
        notes: None,
        model: None,
        root: Some(root.path().into()),
        db: None,
    };
    let err = run_advance(&args, root.path()).unwrap_err();
    assert!(err.to_string().contains("Cannot advance"));
    assert!(err.to_string().contains("pending"));
}

#[test]
fn advance_reject_cycle() {
    let root = tempdir().unwrap();
    let file = "task.md";
    create_pending_task(root.path(), file);

    // SWE advance
    let args = AdvanceArgs {
        task: file.into(),
        stage: TaskStage::Swe,
        verdict: None,
        notes: Some("First try".into()),
        model: None,
        root: Some(root.path().into()),
        db: None,
    };
    run_advance(&args, root.path()).unwrap();

    // Red REJECT (returns to pending)
    let args = AdvanceArgs {
        stage: TaskStage::Red,
        verdict: Some(StageVerdict::Reject),
        notes: Some("Needs work".into()),
        ..args
    };
    run_advance(&args, root.path()).unwrap();
    let content = read_task(&root.path().join("todo").join(file)).unwrap();
    assert!(content.contains("**Verdict**: REJECT"));
    assert!(content.contains("Needs work"));
    assert!(!root.path().join("done").join(file).exists());

    // Can advance again after reject
    let args = AdvanceArgs {
        stage: TaskStage::Swe,
        verdict: None,
        notes: Some("Second try".into()),
        ..args
    };
    run_advance(&args, root.path()).unwrap();
    let content = read_task(&root.path().join("todo").join(file)).unwrap();
    assert_eq!(content.matches("## SWE Remediation").count(), 2);
}

#[test]
fn advance_swe_verdict_contract() {
    let root = tempdir().unwrap();
    let file = "task.md";
    create_pending_task(root.path(), file);

    // SWE cannot have a verdict
    let args = AdvanceArgs {
        task: file.into(),
        stage: TaskStage::Swe,
        verdict: Some(StageVerdict::Pass),
        notes: None,
        model: None,
        root: Some(root.path().into()),
        db: None,
    };
    let err = run_advance(&args, root.path()).unwrap_err();
    assert!(err.to_string().contains("verdict"));
}
