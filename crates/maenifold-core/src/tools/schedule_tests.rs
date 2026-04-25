use super::*;
use crate::acp_config::MaenifoldSettings;
use crate::config::Config;
use crate::util::json::OutputFormat;
use tempfile::TempDir;

fn test_context(temp: &TempDir) -> ToolContext {
    let root = temp.path().join("maenifold");
    ToolContext::new(
        Config {
            maenifold_root: root.clone(),
            memory_path: root.join("memory"),
            assets_path: root.join("assets"),
            thinking_path: root.join("memory/thinking"),
            sequential_path: root.join("memory/thinking/sequential"),
            database_path: root.join("memory.db"),
            settings: MaenifoldSettings::default(),
        },
        OutputFormat::Json,
    )
}

fn create_payload(id: &str, enabled: bool) -> Value {
    json!({
        "action": "create",
        "id": id,
        "schedule": "every weekday at 9am",
        "agent": "swe",
        "prompt": "Run scheduled maintenance",
        "enabled": enabled
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn create_enabled_os_install_failure_leaves_no_persisted_schedule() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let payload = create_payload("install-fails", true);
    let invalid_bin = PathBuf::from("/nonexistent/binary");

    let result = create_task_with_bin(&context, &payload, &invalid_bin);

    assert!(
        result.is_err(),
        "invalid scheduler binary should fail create"
    );
    let persisted = schedule_store(&context).list().expect("list schedules");
    assert!(
        persisted.iter().all(|task| task.id != "install-fails"),
        "failed OS install must not leave active-looking schedule config"
    );
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[test]
fn create_enabled_unsupported_platform_is_blocked_before_persisting() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let payload = create_payload("unsupported-enabled", true);
    let fake_bin = PathBuf::from("/nonexistent/binary");

    let result = create_task_with_bin(&context, &payload, &fake_bin);

    assert!(matches!(result, Err(MaenifoldError::InvalidArgument(_))));
    let persisted = schedule_store(&context).list().expect("list schedules");
    assert!(persisted.is_empty());
}
