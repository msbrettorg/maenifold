use super::resources_tests_common::*;

    /// T-205.716: Verify handle_list_resources returns expected resource types.
    #[test]
    #[serial]
    fn list_resources_returns_expected_resource_types() {
        let (_tmp, _guard) = setup_test_env();
        let config = Config::current();
        
        // Create a pipeline session
        create_pipeline_session(&config, "pipeline-1745366400000-contract");
        
        let result = handle_list_resources().expect("list_resources should succeed");
        
        // Should have at least the pipeline sessions resource
        let pipeline_uris: Vec<_> = result
            .resources
            .iter()
            .filter(|r| r.uri.starts_with("pipeline://"))
            .map(|r| r.uri.as_str())
            .collect();
        
        assert!(
            pipeline_uris.contains(&"pipeline://sessions"),
            "Expected pipeline://sessions resource, got: {:?}",
            pipeline_uris
        );
    }

    /// T-205.716: Concurrent ListAssets tool invocations via ToolRegistry.
    ///
    /// This test exercises the exact production code path:
    /// - ToolRegistry::global().try_invoke("ListAssets", ...) - the MCP server entry point
    /// - Concurrent with handle_list_resources() calls
    /// - With active pipeline sessions
    ///
    /// The test verifies no deadlock or I/O errors occur during concurrent tool invocation.
    #[test]
    #[serial]
    fn concurrent_tool_invocation_does_not_deadlock() {
        let (_tmp, _guard) = setup_test_env();
        let config = Config::current();

        // Create pipeline sessions and asset files
        for i in 0..5 {
            create_pipeline_session(&config, &format!("pipeline-1745366400000-tool{i}"));
        }

        // Create some asset files for ListAssets to find
        for kind in ["workflows", "roles"] {
            let asset_path = config.assets_path.join(kind).join("test-asset.json");
            fs::write(
                &asset_path,
                serde_json::to_string_pretty(&json!({
                    "id": format!("test-{kind}"),
                    "name": format!("Test {kind}"),
                    "description": "Test asset for concurrency testing"
                }))
                .unwrap(),
            )
            .expect("write asset");
        }

        let deadlock_count = Arc::new(AtomicUsize::new(0));
        let tool_success_count = Arc::new(AtomicUsize::new(0));
        let resource_success_count = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();

        // Spawn threads that invoke ListAssets via ToolRegistry
        for _ in 0..6 {
            let deadlocks = Arc::clone(&deadlock_count);
            let successes = Arc::clone(&tool_success_count);

            handles.push(thread::spawn(move || {
                for _ in 0..20 {
                    // Untyped ListAssets
                    match ToolRegistry::global().try_invoke(
                        "ListAssets",
                        &json!({}),
                        OutputFormat::Json,
                    ) {
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

                    // Typed ListAssets
                    match ToolRegistry::global().try_invoke(
                        "ListAssets",
                        &json!({"type": "workflow"}),
                        OutputFormat::Json,
                    ) {
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

        // Spawn threads that call handle_list_resources concurrently
        for _ in 0..6 {
            let deadlocks = Arc::clone(&deadlock_count);
            let successes = Arc::clone(&resource_success_count);

            handles.push(thread::spawn(move || {
                for _ in 0..20 {
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
        let tool_final = tool_success_count.load(Ordering::Relaxed);
        let resource_final = resource_success_count.load(Ordering::Relaxed);

        // Primary assertion: no deadlocks
        assert_eq!(
            deadlock_final, 0,
            "Detected {deadlock_final} deadlock errors during concurrent ListAssets/list_resources. \
             This is the bug T-205.716 is tracking."
        );

        // Secondary: verify operations succeeded
        assert!(
            tool_final > 100,
            "Expected at least 100 successful ListAssets calls, got {tool_final}"
        );
        assert!(
            resource_final > 100,
            "Expected at least 100 successful list_resources calls, got {resource_final}"
        );
    }
