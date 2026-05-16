use super::*;

fn assert_stage(root: &std::path::Path, stage: &str) {
    assert_eq!(single_status(root)["state"], stage);
}

#[test]
fn cascade_simulation_without_expected_stage_allows_at_most_one_advance() {
    let root = new_root();
    write_task(root.path(), "T-325.1", "tdd");

    let first = GateLifecycle::pass(note(root.path(), "T-325.1", "pm", "first"));
    let second = GateLifecycle::pass(note(root.path(), "T-325.1", "pm", "second"));
    let third = GateLifecycle::pass(note(root.path(), "T-325.1", "pm", "third"));
    let fourth = GateLifecycle::pass(note(root.path(), "T-325.1", "pm", "fourth"));

    assert!(first.is_ok());
    assert!(second.is_err());
    assert!(third.is_err());
    assert!(fourth.is_err());
    assert_stage(root.path(), "blue-team");
}

#[test]
fn expected_stage_mismatch_returns_stage_mismatch_error() {
    let root = new_root();
    write_task(root.path(), "T-325.2", "tdd");

    GateLifecycle::pass(note(root.path(), "T-325.2", "pm", "start")).expect("initial pass");
    GateLifecycle::pass(note_with_expected_stage(
        root.path(),
        "T-325.2",
        "blue",
        "blue-team",
        "advance to swe",
    ))
    .expect("expected stage pass");

    let error = GateLifecycle::pass(note_with_expected_stage(
        root.path(),
        "T-325.2",
        "swe",
        "blue-team",
        "stale pass",
    ))
    .expect_err("stale expected stage rejected");
    let message = error.to_string();
    assert!(message.contains("STAGE_MISMATCH") || message.contains("expected stage"));
    assert_stage(root.path(), "swe");
}

#[test]
fn missing_expected_stage_on_non_initial_stage_returns_error() {
    let root = new_root();
    write_task(root.path(), "T-325.3", "tdd");

    GateLifecycle::pass(note(root.path(), "T-325.3", "pm", "initial")).expect("initial pass");

    let error = GateLifecycle::pass(note(root.path(), "T-325.3", "blue", "missing expected"))
        .expect_err("expected stage required after initial stage");
    let message = error.to_string();
    assert!(message.contains("EXPECTED_STAGE_REQUIRED") || message.contains("expectedStage"));
    assert_stage(root.path(), "blue-team");
}
