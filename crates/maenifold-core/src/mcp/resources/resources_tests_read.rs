use super::resources_tests_common::*;

#[test]
#[serial]
fn read_pipeline_app_resource_returns_self_contained_html() {
    let (_tmp, _guard) = setup_test_env();

    let result = handle_read_resource(crate::mcp::tool_utils::PIPELINE_UI_RESOURCE_URI)
        .expect("pipeline app resource should read");
    let Some(rmcp::model::ResourceContents::TextResourceContents {
        text, mime_type, ..
    }) = result.contents.first()
    else {
        panic!("pipeline app resource should be text");
    };

    assert_eq!(
        mime_type.as_deref(),
        Some(crate::mcp::tool_utils::MCP_APP_HTML_MIME_TYPE)
    );
    assert!(text.starts_with("<!doctype html>"));
    for required_key in [
        "nodes",
        "edges",
        "status",
        "ready",
        "active",
        "completed",
        "running",
    ] {
        assert!(
            text.contains(required_key),
            "pipeline app HTML should handle {required_key}"
        );
    }
}

/// T-205.716: Regression test for concurrent handle_read_resource() calls on pipeline sessions.
///
/// **CRITICAL PATH**: This test exercises the ACTUAL deadlock surface:
/// - handle_read_resource("pipeline://session/{id}") → pipeline::handle_read_resource()
/// - → read_session_payload() → Pipeline tool with { "sessionId": ..., "view": true }
/// - → view_pipeline_session() → build_pipeline_view_result()
/// - → session_lock_is_held() → fd_lock::RwLock::try_write()
///
/// Unlike list_resources() which only reads markdown, read_resource() for a specific
/// session probes the fd_lock to determine if the pipeline is "running". This is the
/// path that could interact with concurrent lock holders.
#[test]
#[serial]
fn concurrent_read_resource_session_does_not_deadlock() {
    let (_tmp, _guard) = setup_test_env();
    let config = Config::current();

    // Create pipeline sessions to read
    let session_ids: Vec<String> = (0..5)
        .map(|i| format!("pipeline-1745366400000-read{i}"))
        .collect();
    for session_id in &session_ids {
        create_pipeline_session(&config, session_id);
    }

    let deadlock_count = Arc::new(AtomicUsize::new(0));
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    // Spawn threads that READ pipeline session resources (probes fd_lock)
    for session_id in session_ids.clone() {
        let deadlocks = Arc::clone(&deadlock_count);
        let successes = Arc::clone(&success_count);
        let errors = Arc::clone(&error_count);
        let uri = format!("pipeline://session/{session_id}");

        handles.push(thread::spawn(move || {
            for _ in 0..30 {
                match handle_read_resource(&uri) {
                    Ok(result) => {
                        // Verify we got a valid pipeline view response
                        if let Some(rmcp::model::ResourceContents::TextResourceContents {
                            text,
                            ..
                        }) = result.contents.first()
                        {
                            if text.contains("pipelineId") {
                                successes.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        eprintln!("DEBUG: Session read error: {msg}");
                        if msg.contains("deadlock") || msg.contains("os error 11") {
                            deadlocks.fetch_add(1, Ordering::Relaxed);
                        }
                        errors.fetch_add(1, Ordering::Relaxed);
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
    let success_final = success_count.load(Ordering::Relaxed);

    // The primary assertion: no deadlocks should occur
    assert_eq!(
        deadlock_final, 0,
        "Detected {deadlock_final} deadlock errors during concurrent session reads. \
             This is the bug T-205.716 is tracking - session reads probe fd_lock."
    );

    // Secondary: most calls should succeed
    assert!(
        success_final > 50,
        "Expected at least 50 successful session reads, got {success_final}"
    );
}

/// T-205.716: The most likely deadlock scenario - concurrent session reads with lock holders.
///
/// This test creates the exact contention pattern that could trigger EDEADLK:
/// - Thread A: holds exclusive fd_lock on pipeline session (simulating active pipeline driver)
/// - Thread B: calls handle_read_resource() for same session → session_lock_is_held() → try_write()
///
/// The theory is that when try_write() encounters a held lock, some interaction
/// with Config::OVERRIDES or other internal state causes the kernel to detect
/// a circular dependency.
#[test]
#[serial]
fn concurrent_read_resource_with_active_locks_does_not_deadlock() {
    let (_tmp, _guard) = setup_test_env();
    let config = Config::current();

    // Create pipeline sessions
    let session_ids: Vec<String> = (0..3)
        .map(|i| format!("pipeline-1745366400000-contend{i}"))
        .collect();
    for session_id in &session_ids {
        create_pipeline_session(&config, session_id);
    }

    let deadlock_count = Arc::new(AtomicUsize::new(0));
    let lock_acquired_count = Arc::new(AtomicUsize::new(0));
    let read_success_count = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    // Spawn threads that HOLD exclusive fd_locks (simulating pipeline drivers)
    for session_id in session_ids.clone() {
        let deadlocks = Arc::clone(&deadlock_count);
        let locks = Arc::clone(&lock_acquired_count);
        let config_clone = config.clone();

        handles.push(thread::spawn(move || {
            let lock_path = pipeline_lock_path(&session_id, &config_clone);
            if let Some(parent) = lock_path.parent() {
                fs::create_dir_all(parent).ok();
            }

            // Acquire and hold the exclusive lock for a while
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
                            // Hold the lock while doing multiple iterations
                            for _ in 0..100 {
                                guard.write_all(b".").ok();
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

    // Spawn threads that READ session resources (probes the held locks via session_lock_is_held)
    for session_id in session_ids.clone() {
        let deadlocks = Arc::clone(&deadlock_count);
        let successes = Arc::clone(&read_success_count);
        let uri = format!("pipeline://session/{session_id}");

        handles.push(thread::spawn(move || {
            for _ in 0..50 {
                match handle_read_resource(&uri) {
                    Ok(result) => {
                        // Check that "running" field is present (set by session_lock_is_held)
                        if let Some(rmcp::model::ResourceContents::TextResourceContents {
                            text,
                            ..
                        }) = result.contents.first()
                        {
                            // "running": true/false is set by session_lock_is_held()
                            if text.contains("\"running\"") {
                                successes.fetch_add(1, Ordering::Relaxed);
                            }
                        }
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
    let lock_final = lock_acquired_count.load(Ordering::Relaxed);
    let read_final = read_success_count.load(Ordering::Relaxed);

    // The primary assertion: no deadlocks
    assert_eq!(
        deadlock_final, 0,
        "Detected {deadlock_final} deadlock errors when reading sessions with active locks. \
             This is the bug T-205.716 is tracking - concurrent session_lock_is_held() probing."
    );

    // Verify both lock holders and readers executed
    assert!(
        lock_final > 0,
        "Expected at least some locks to be acquired, got {lock_final}"
    );
    assert!(
        read_final > 0,
        "Expected at least some session reads to succeed, got {read_final}"
    );
}
