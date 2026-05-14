use super::*;
use chrono::TimeZone;
use tempfile::TempDir;

fn test_store() -> (TempDir, ScheduleStore) {
    let temp = TempDir::new().expect("temp dir");
    let store = ScheduleStore::new(temp.path().join("schedules"));
    (temp, store)
}

fn test_task(id: &str) -> ScheduledTask {
    ScheduledTask {
        id: id.to_owned(),
        name: format!("task-{id}"),
        schedule: "0 * * * *".to_owned(),
        schedule_human: Some("hourly".to_owned()),
        agent: "swe".to_owned(),
        prompt: format!("run {id}"),
        workdir: Some(PathBuf::from("/tmp/workdir")),
        auto_grant_permissions: false,
        allowed_tools: Vec::new(),
        mcp_action: None,
        mcp_parameters: None,
        timeout_seconds: 900,
        enabled: true,
        created_at: Utc
            .with_ymd_and_hms(2026, 4, 21, 7, 0, 0)
            .single()
            .expect("valid test timestamp"),
        updated_at: Utc
            .with_ymd_and_hms(2026, 4, 21, 7, 0, 0)
            .single()
            .expect("valid test timestamp"),
        last_run: None,
    }
}

#[test]
fn create_and_read_round_trip() {
    let (_temp, store) = test_store();
    let task = test_task("round-trip");
    store.create(&task).expect("create task");
    let saved = store.read(&task.id).expect("read task");
    assert_eq!(
        serde_json::to_value(&saved).expect("serialize saved"),
        serde_json::to_value(&task).expect("serialize task")
    );
}

#[test]
fn duplicate_create_returns_error() {
    let (_temp, store) = test_store();
    let task = test_task("duplicate");
    store.create(&task).expect("initial create");
    assert!(store.create(&task).is_err());
}

#[test]
fn update_existing_persists_changes() {
    let (_temp, store) = test_store();
    let mut task = test_task("update");
    store.create(&task).expect("create task");
    let original_updated_at = task.updated_at;
    task.name = "updated-name".to_owned();
    task.prompt = "updated prompt".to_owned();
    task.enabled = false;
    store.update(&task).expect("update task");
    let saved = store.read(&task.id).expect("read updated task");
    assert_eq!(saved.name, "updated-name");
    assert_eq!(saved.prompt, "updated prompt");
    assert!(!saved.enabled);
    assert!(saved.updated_at > original_updated_at);
}

#[test]
fn restore_snapshot_if_current_rejects_concurrent_change() {
    let (_temp, store) = test_store();
    let original = test_task("cas-restore");
    store.create(&original).expect("create task");
    let mut failed_update = original.clone();
    failed_update.prompt = "failed update".to_owned();
    store.update(&failed_update).expect("write failed update");
    let failed_update = store.read(&original.id).expect("read failed update");
    let mut concurrent = failed_update.clone();
    concurrent.prompt = "concurrent update".to_owned();
    store.update(&concurrent).expect("write concurrent update");

    let result = store.restore_snapshot_if_current(&original, &failed_update);

    assert!(result.is_err());
    let saved = store.read(&original.id).expect("read saved task");
    assert_eq!(saved.prompt, "concurrent update");
}

#[test]
fn delete_existing_removes_task() {
    let (_temp, store) = test_store();
    let task = test_task("delete-existing");
    store.create(&task).expect("create task");
    store.delete(&task.id).expect("delete task");
    assert!(store.read(&task.id).is_err());
}

#[test]
fn delete_nonexistent_is_idempotent() {
    let (_temp, store) = test_store();
    assert!(store.delete("never-created-task").is_ok());
}

#[test]
fn list_skips_malformed_json() {
    let (_temp, store) = test_store();
    let alpha = test_task("alpha");
    let beta = test_task("beta");
    store.create(&alpha).expect("create alpha");
    store.create(&beta).expect("create beta");
    fs::write(store.base_dir().join("malformed.json"), b"{not-json").expect("write malformed");
    let tasks = store.list().expect("list tasks");
    assert_eq!(
        tasks.into_iter().map(|task| task.id).collect::<Vec<_>>(),
        vec!["alpha", "beta"]
    );
}

#[test]
fn invalid_schedule_ids_are_rejected() {
    let (_temp, store) = test_store();
    assert!(store.create(&test_task("../foo")).is_err());
    assert!(store.create(&test_task("bad$id")).is_err());
}

#[test]
fn exists_reflects_create_and_delete() {
    let (_temp, store) = test_store();
    let task = test_task("exists");
    assert!(!store.exists(&task.id));
    store.create(&task).expect("create task");
    assert!(store.exists(&task.id));
    store.delete(&task.id).expect("delete task");
    assert!(!store.exists(&task.id));
}
