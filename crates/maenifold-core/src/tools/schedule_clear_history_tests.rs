use super::*;
use std::fs;

#[test]
fn clear_history_unknown_schedule_returns_invalid_argument() {
    let (_t, c) = ctx();
    let result = schedule(&c, &json!({"action":"clear_history","id":"missing"}));
    assert!(matches!(
        result,
        Err(MaenifoldError::InvalidArgument(message))
            if message == "unknown scheduleId 'missing'"
    ));
}

#[test]
fn clear_history_empty_logs_returns_zero_count() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    let payload = parse(
        &schedule(&c, &json!({"action":"clear_history","id":"alpha"})).expect("clear history"),
    );
    assert_eq!(
        payload,
        json!({"action":"clear_history","id":"alpha","deletedCount":0,"deletedFiles":[]})
    );
}

#[test]
fn clear_history_deletes_existing_log_files_and_returns_paths() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    let log_dir = schedule_store(&c).task_dir("alpha").join("logs");
    let first = log_dir.join("20260429T140000Z.log");
    let second = log_dir.join("20260429T150000Z.log");
    fs::write(&first, "first").expect("write first log");
    fs::write(&second, "second").expect("write second log");

    let payload = parse(
        &schedule(&c, &json!({"action":"clear_history","id":"alpha"})).expect("clear history"),
    );

    assert_eq!(payload["deletedCount"], 2);
    assert_eq!(
        payload["deletedFiles"],
        json!([first.display().to_string(), second.display().to_string()])
    );
    assert!(!first.exists());
    assert!(!second.exists());
}

#[test]
fn clear_history_post_deletion_listing_is_empty() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    let log_dir = schedule_store(&c).task_dir("alpha").join("logs");
    fs::write(log_dir.join("20260429T140000Z.log"), "first").expect("write log");

    schedule(&c, &json!({"action":"clear_history","id":"alpha"})).expect("clear history");
    let logs = parse(&schedule(&c, &json!({"action":"logs","id":"alpha"})).expect("list logs"));

    assert_eq!(logs, json!([]));
}
