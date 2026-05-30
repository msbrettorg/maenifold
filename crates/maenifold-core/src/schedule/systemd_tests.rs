use super::cron_to_oncalendar;

#[test]
fn converts_weekday_schedule() {
    let calendar = cron_to_oncalendar("0 9 * * 1-5").expect("cron should convert");
    assert_eq!(calendar, "Mon..Fri *-*-* 09:00:00");
}

#[test]
fn converts_daily_midnight_schedule() {
    let calendar = cron_to_oncalendar("0 0 * * *").expect("cron should convert");
    assert_eq!(calendar, "*-*-* 00:00:00");
}

#[test]
fn converts_monthly_schedule() {
    let calendar = cron_to_oncalendar("0 12 1 * *").expect("cron should convert");
    assert_eq!(calendar, "*-*-01 12:00:00");
}

#[cfg(all(test, target_os = "linux"))]
mod isolation_tests {
    use super::super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

    struct CountingSystemctlBackend {
        daemon_reload_count: Arc<AtomicUsize>,
        enable_count: Arc<AtomicUsize>,
        disable_count: Arc<AtomicUsize>,
    }

    impl SystemctlBackend for CountingSystemctlBackend {
        fn daemon_reload(&self) -> Result<()> {
            self.daemon_reload_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn enable(&self, _unit: &str) -> Result<()> {
            self.enable_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn disable(&self, _unit: &str) -> Result<()> {
            self.disable_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn fake_bin_in(temp: &TempDir) -> PathBuf {
        let bin = temp.path().join("fake-maenifold");
        fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("write fake bin");
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&bin).expect("fake bin metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin, permissions).expect("set fake bin permissions");
        bin
    }

    fn task(task_id: &str) -> ScheduledTask {
        ScheduledTask {
            id: task_id.to_string(),
            name: task_id.to_string(),
            schedule: "0 9 * * 1-5".to_string(),
            schedule_human: Some("every weekday at 9am".to_string()),
            agent: "swe".to_string(),
            prompt: "Run scheduled maintenance".to_string(),
            workdir: None,
            auto_grant_permissions: false,
            allowed_tools: Vec::new(),
            mcp_action: None,
            mcp_parameters: None,
            timeout_seconds: 300,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run: None,
        }
    }

    fn user_units_dir() -> PathBuf {
        PathBuf::from(std::env::var_os("HOME").expect("HOME should be set"))
            .join(".config/systemd/user")
    }

    fn host_schedule_unit_count() -> usize {
        let units_dir = user_units_dir();
        fs::read_dir(&units_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                name.starts_with(UNIT_PREFIX)
                    && (name.ends_with(".timer") || name.ends_with(".service"))
            })
            .count()
    }

    #[test]
    fn install_and_uninstall_stay_within_temp_units_dir() {
        let before_count = host_schedule_unit_count();
        let temp = TempDir::new().expect("temp dir");
        let fake_bin = fake_bin_in(&temp);
        let task_id = format!(
            "bluteam-isolation-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        );
        let task = task(&task_id);
        let temp_units_dir = temp.path().join(".config/systemd/user");
        let temp_timer_path = temp_units_dir.join(timer_unit_name(&task_id));
        let temp_service_path = temp_units_dir.join(service_unit_name(&task_id));
        let host_timer_path = user_units_dir().join(timer_unit_name(&task_id));
        let host_service_path = user_units_dir().join(service_unit_name(&task_id));
        let daemon_reload_count = Arc::new(AtomicUsize::new(0));
        let enable_count = Arc::new(AtomicUsize::new(0));
        let disable_count = Arc::new(AtomicUsize::new(0));
        let backend = CountingSystemctlBackend {
            daemon_reload_count: Arc::clone(&daemon_reload_count),
            enable_count: Arc::clone(&enable_count),
            disable_count: Arc::clone(&disable_count),
        };
        let manager = SystemdManager::new_with(temp_units_dir.clone(), fake_bin, Arc::new(backend))
            .expect("create systemd manager");

        manager.install(&task).expect("install schedule");

        assert!(
            temp_timer_path.exists(),
            "temp timer should exist after install"
        );
        assert!(
            temp_service_path.exists(),
            "temp service should exist after install"
        );
        assert!(
            !host_timer_path.exists(),
            "host user units should not contain test timer"
        );
        assert!(
            !host_service_path.exists(),
            "host user units should not contain test service"
        );
        assert_eq!(daemon_reload_count.load(Ordering::SeqCst), 1);
        assert_eq!(enable_count.load(Ordering::SeqCst), 1);
        assert_eq!(disable_count.load(Ordering::SeqCst), 1);

        manager.uninstall(&task_id).expect("uninstall schedule");

        assert!(
            !temp_timer_path.exists(),
            "temp timer should be removed after uninstall"
        );
        assert!(
            !temp_service_path.exists(),
            "temp service should be removed after uninstall"
        );
        let temp_entries = fs::read_dir(&temp_units_dir)
            .expect("temp units dir should exist")
            .filter_map(|entry| entry.ok())
            .count();
        assert_eq!(temp_entries, 0, "temp units dir should be clean");
        assert_eq!(daemon_reload_count.load(Ordering::SeqCst), 2);
        assert_eq!(enable_count.load(Ordering::SeqCst), 1);
        assert_eq!(disable_count.load(Ordering::SeqCst), 2);

        let after_count = host_schedule_unit_count();
        assert_eq!(
            after_count, before_count,
            "host user unit count should remain unchanged"
        );
    }
}

#[cfg(test)]
mod cross_platform_trait_tests {
    use super::super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    struct MockBackend {
        daemon_reload_calls: Arc<AtomicUsize>,
        enable_calls: Arc<AtomicUsize>,
        disable_calls: Arc<AtomicUsize>,
    }

    impl SystemctlBackend for MockBackend {
        fn daemon_reload(&self) -> Result<()> {
            self.daemon_reload_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn enable(&self, _unit: &str) -> Result<()> {
            self.enable_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn disable(&self, _unit: &str) -> Result<()> {
            self.disable_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn fake_bin_in(temp: &TempDir) -> PathBuf {
        let bin = temp.path().join("fake-maenifold");
        fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("write fake bin");
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&bin).expect("fake bin metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin, permissions).expect("set fake bin permissions");
        bin
    }

    fn task(task_id: &str) -> ScheduledTask {
        ScheduledTask {
            id: task_id.to_string(),
            name: task_id.to_string(),
            schedule: "0 9 * * 1-5".to_string(),
            schedule_human: Some("every weekday at 9am".to_string()),
            agent: "swe".to_string(),
            prompt: "Run scheduled maintenance".to_string(),
            workdir: None,
            auto_grant_permissions: false,
            allowed_tools: Vec::new(),
            mcp_action: None,
            mcp_parameters: None,
            timeout_seconds: 300,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run: None,
        }
    }

    fn user_units_dir() -> PathBuf {
        PathBuf::from(std::env::var_os("HOME").expect("HOME should be set"))
            .join(".config/systemd/user")
    }

    fn host_schedule_unit_count() -> usize {
        let units_dir = user_units_dir();
        fs::read_dir(&units_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                name.starts_with(UNIT_PREFIX)
                    && (name.ends_with(".timer") || name.ends_with(".service"))
            })
            .count()
    }

    #[test]
    fn install_uninstall_routes_through_backend_and_writes_to_temp_units_dir() {
        let before_count = host_schedule_unit_count();
        let temp = TempDir::new().expect("temp dir");
        let fake_bin = fake_bin_in(&temp);
        let temp_units_dir = temp.path().join(".config/systemd/user");
        let task = task("alpha-test-1");
        let temp_timer_path = temp_units_dir.join(timer_unit_name(&task.id));
        let temp_service_path = temp_units_dir.join(service_unit_name(&task.id));
        let host_timer_path = user_units_dir().join(timer_unit_name(&task.id));
        let host_service_path = user_units_dir().join(service_unit_name(&task.id));
        let daemon_reload_calls = Arc::new(AtomicUsize::new(0));
        let enable_calls = Arc::new(AtomicUsize::new(0));
        let disable_calls = Arc::new(AtomicUsize::new(0));
        let backend = MockBackend {
            daemon_reload_calls: Arc::clone(&daemon_reload_calls),
            enable_calls: Arc::clone(&enable_calls),
            disable_calls: Arc::clone(&disable_calls),
        };
        let manager = SystemdManager::new_with(temp_units_dir.clone(), fake_bin, Arc::new(backend))
            .expect("create systemd manager");

        manager.install(&task).expect("install schedule");

        assert!(
            temp_timer_path.exists(),
            "temp timer should exist after install"
        );
        assert!(
            temp_service_path.exists(),
            "temp service should exist after install"
        );
        assert!(
            !host_timer_path.exists(),
            "host user units should not contain test timer"
        );
        assert!(
            !host_service_path.exists(),
            "host user units should not contain test service"
        );
        assert_eq!(daemon_reload_calls.load(Ordering::SeqCst), 1);
        assert_eq!(enable_calls.load(Ordering::SeqCst), 1);
        assert_eq!(disable_calls.load(Ordering::SeqCst), 1);

        manager.uninstall(&task.id).expect("uninstall schedule");

        assert!(
            !temp_timer_path.exists(),
            "temp timer should be removed after uninstall"
        );
        assert!(
            !temp_service_path.exists(),
            "temp service should be removed after uninstall"
        );
        let temp_entries = fs::read_dir(&temp_units_dir)
            .expect("temp units dir should exist")
            .filter_map(|entry| entry.ok())
            .count();
        assert_eq!(temp_entries, 0, "temp units dir should be clean");
        assert_eq!(disable_calls.load(Ordering::SeqCst), 2);
        assert_eq!(daemon_reload_calls.load(Ordering::SeqCst), 2);
        assert_eq!(enable_calls.load(Ordering::SeqCst), 1);

        let after_count = host_schedule_unit_count();
        assert_eq!(
            before_count, after_count,
            "host user unit count should remain unchanged"
        );
    }
}
