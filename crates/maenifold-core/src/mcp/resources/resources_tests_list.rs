use super::resources_tests_common::*;

    /// T-205.716: Regression test for concurrent handle_list_resources() calls.
    ///
    /// This test reproduces the scenario that triggers "Resource deadlock avoided (os error 11)":
    /// - Multiple threads call handle_list_resources() concurrently
    /// - Pipeline sessions exist and are being processed
    /// - The aggregation path goes: handle_list_resources() -> pipeline::list_resources()
    ///   -> read_sessions_payload() -> ToolRegistry::try_invoke("Pipeline", {list: true})
    ///   -> list_pipelines() -> read_markdown() -> FRONTMATTER_WARNINGS mutex
    ///
    /// The deadlock occurs when concurrent calls create lock ordering conflicts between:
    /// - Config::OVERRIDES (parking_lot::RwLock)
    /// - FRONTMATTER_WARNINGS (std::sync::Mutex)
    /// - fd_lock file locks on pipeline sessions
    #[test]
    #[serial]
    fn concurrent_list_resources_does_not_deadlock() {
        let (_tmp, _guard) = setup_test_env();
        let config = Config::current();
        
        // Create multiple pipeline sessions to exercise the listing path
        for i in 0..5 {
            create_pipeline_session(&config, &format!("pipeline-1745366400000-test{i}"));
        }

        let success_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let deadlock_count = Arc::new(AtomicUsize::new(0));
        
        // Spawn many threads to maximize contention
        let num_threads = 16;
        let iterations_per_thread = 25;
        let mut handles = Vec::with_capacity(num_threads);
        
        for _ in 0..num_threads {
            let success = Arc::clone(&success_count);
            let errors = Arc::clone(&error_count);
            let deadlocks = Arc::clone(&deadlock_count);
            
            handles.push(thread::spawn(move || {
                for _ in 0..iterations_per_thread {
                    match handle_list_resources() {
                        Ok(result) => {
                            // Verify the result contains expected resource types
                            let has_pipeline = result
                                .resources
                                .iter()
                                .any(|r| r.uri.starts_with("pipeline://"));
                            let has_asset = result
                                .resources
                                .iter()
                                .any(|r| r.uri.starts_with("asset://"));
                            if has_pipeline || has_asset || result.resources.is_empty() {
                                success.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            if msg.contains("deadlock") || msg.contains("os error 11") {
                                deadlocks.fetch_add(1, Ordering::Relaxed);
                            }
                            errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    // Small yield to increase thread interleaving
                    thread::yield_now();
                }
            }));
        }

        for handle in handles {
            handle.join().expect("thread panicked");
        }

        let deadlock_final = deadlock_count.load(Ordering::Relaxed);
        let success_final = success_count.load(Ordering::Relaxed);
        let error_final = error_count.load(Ordering::Relaxed);
        
        // The primary assertion: no deadlocks should occur
        assert_eq!(
            deadlock_final, 0,
            "Detected {deadlock_final} deadlock errors (os error 11). \
             This is the bug T-205.716 is tracking."
        );
        
        // Secondary: most calls should succeed
        let total = num_threads * iterations_per_thread;
        assert!(
            success_final > total / 2,
            "Only {success_final}/{total} calls succeeded, {error_final} errors"
        );
    }

    /// T-205.716: Regression test combining resource listing with pipeline lock acquisition.
    ///
    /// This test exercises the exact scenario that causes EDEADLK:
    /// - Thread A: holds fd_lock on a pipeline session, waits to read config
    /// - Thread B: holds config lock, calls list_resources which tries to check session locks
    /// - Kernel detects circular dependency and returns EDEADLK
    #[test]
    #[serial]
    fn concurrent_list_resources_with_active_pipeline_locks_does_not_deadlock() {
        let (_tmp, _guard) = setup_test_env();
        let config = Config::current();
        
        // Create pipeline sessions
        let session_ids: Vec<String> = (0..3)
            .map(|i| format!("pipeline-1745366400000-lock{i}"))
            .collect();
        for session_id in &session_ids {
            create_pipeline_session(&config, session_id);
        }

        let deadlock_count = Arc::new(AtomicUsize::new(0));
        let lock_acquired_count = Arc::new(AtomicUsize::new(0));
        let list_success_count = Arc::new(AtomicUsize::new(0));
        
        let mut handles = Vec::new();
        
        // Spawn threads that hold pipeline locks (simulating active pipeline drivers)
        for session_id in session_ids.clone() {
            let deadlocks = Arc::clone(&deadlock_count);
            let locks = Arc::clone(&lock_acquired_count);
            let config_clone = config.clone();
            
            handles.push(thread::spawn(move || {
                let lock_path = pipeline_lock_path(&session_id, &config_clone);
                if let Some(parent) = lock_path.parent() {
                    fs::create_dir_all(parent).ok();
                }
                
                // Try to acquire and hold the lock
                match std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&lock_path)
                {
                    Ok(file) => {
                        let mut rw = fd_lock::RwLock::new(file);
                        match rw.try_write() {
                            Ok(mut guard) => {
                                locks.fetch_add(1, Ordering::Relaxed);
                                // Hold the lock while doing some work
                                for _ in 0..50 {
                                    guard.write_all(b"x").ok();
                                    thread::yield_now();
                                }
                                // Guard drops here, releasing lock
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                if msg.contains("deadlock") || msg.contains("os error 11") {
                                    deadlocks.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }));
        }
        
        // Spawn threads that call list_resources concurrently
        for _ in 0..8 {
            let deadlocks = Arc::clone(&deadlock_count);
            let successes = Arc::clone(&list_success_count);
            
            handles.push(thread::spawn(move || {
                for _ in 0..30 {
                    match handle_list_resources() {
                        Ok(_) => {
                            successes.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            if msg.contains("deadlock") || msg.contains("os error 11") {
                                deadlocks.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    thread::yield_now();
                }
            }));
        }

        for handle in handles {
            handle.join().expect("thread panicked");
        }

        let deadlock_final = deadlock_count.load(Ordering::Relaxed);
        
        // The primary assertion: no deadlocks
        assert_eq!(
            deadlock_final, 0,
            "Detected {deadlock_final} deadlock errors while mixing fd_lock acquisition \
             with list_resources calls. This is the bug T-205.716 is tracking."
        );
    }
