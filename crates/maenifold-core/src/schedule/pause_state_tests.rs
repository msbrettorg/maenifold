use super::{ScheduleStore, ScheduledTask, pause_state};
use chrono::Utc;
use tempfile::TempDir;

fn state(entries: &[(&str, bool)]) -> pause_state::PauseState {
    pause_state::PauseState {
        paused_at: Utc::now(),
        prior_enabled: entries
            .iter()
            .map(|(id, enabled)| ((*id).to_string(), *enabled))
            .collect(),
    }
}

fn task(id: &str) -> ScheduledTask {
    ScheduledTask {
        id: id.to_string(),
        name: id.to_string(),
        schedule: "0 9 * * *".to_string(),
        schedule_human: Some("every day at 9am".to_string()),
        agent: "swe".to_string(),
        prompt: "Run scheduled maintenance".to_string(),
        workdir: None,
        auto_grant_permissions: false,
        allowed_tools: Vec::new(),
        timeout_seconds: 300,
        enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_run: None,
    }
}

#[test]
fn pause_state_round_trip() {
    let temp = TempDir::new().expect("temp dir");
    let expected = state(&[("alpha", true), ("beta", false)]);

    assert!(matches!(
        pause_state::try_create(temp.path(), &expected).expect("create pause state"),
        pause_state::CreateOutcome::Created
    ));

    let actual = pause_state::read(temp.path())
        .expect("read pause state")
        .expect("pause state exists");
    assert_eq!(actual.prior_enabled, expected.prior_enabled);
}

#[test]
fn pause_state_try_create_idempotent() {
    let temp = TempDir::new().expect("temp dir");
    let first = state(&[("alpha", true)]);
    let second = state(&[("alpha", false), ("beta", true)]);

    assert!(matches!(
        pause_state::try_create(temp.path(), &first).expect("first create"),
        pause_state::CreateOutcome::Created
    ));
    let outcome = pause_state::try_create(temp.path(), &second).expect("second create");

    match outcome {
        pause_state::CreateOutcome::AlreadyExists(existing) => {
            assert_eq!(existing.prior_enabled, first.prior_enabled);
        }
        pause_state::CreateOutcome::Created => panic!("second create must be idempotent"),
    }
}

#[test]
fn pause_state_clear_idempotent() {
    let temp = TempDir::new().expect("temp dir");
    let expected = state(&[("alpha", true)]);

    pause_state::try_create(temp.path(), &expected).expect("create pause state");
    pause_state::clear(temp.path()).expect("clear existing");
    assert!(!pause_state::path(temp.path()).exists());
    pause_state::clear(temp.path()).expect("clear absent");
}

#[test]
fn pause_state_path_isolated_from_store_list() {
    let temp = TempDir::new().expect("temp dir");
    let store = ScheduleStore::new(temp.path().join("schedules"));
    store.create(&task("alpha")).expect("create schedule");
    pause_state::try_create(temp.path(), &state(&[("alpha", false)])).expect("create pause state");

    let ids = store
        .list()
        .expect("list schedules")
        .into_iter()
        .map(|task| task.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["alpha"]);
    assert_eq!(
        pause_state::path(temp.path()),
        temp.path().join("schedules-pause-state.json")
    );
}
