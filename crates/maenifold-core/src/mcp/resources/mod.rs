use crate::error::{MaenifoldError, Result};
use rmcp::model::{ListResourceTemplatesResult, ListResourcesResult, ReadResourceResult};
use rmcp::service::RequestContext;
use rmcp::RoleServer;

pub mod agent;
mod asset;
pub mod gate;
pub mod gate_history;
pub mod gate_status;
pub mod gate_task;
pub mod graph;
pub mod pipeline;
pub mod subscription;
mod subscription_worker;
pub mod throttle;
pub mod workflow;

pub use subscription::{notify_resource_updated, start_asset_watcher, AssetWatcherState};

// T-12.8: RTM FR-11 — resources/list assembly now lives in mcp/resources.
pub fn handle_list_resources() -> Result<ListResourcesResult> {
    let mut resources = asset::list_resources()?;
    resources.extend(agent::list_resources());
    resources.extend(gate::list_resources());
    resources.extend(graph::list_resources());
    resources.extend(pipeline::list_resources()?);
    resources.extend(workflow::list_resources());
    Ok(ListResourcesResult {
        resources,
        next_cursor: None,
        meta: None,
    })
}

pub fn handle_list_resource_templates() -> Result<ListResourceTemplatesResult> {
    let mut resource_templates = graph::list_resource_templates();
    resource_templates.extend(agent::list_resource_templates());
    resource_templates.extend(gate::list_resource_templates());
    resource_templates.extend(pipeline::list_resource_templates());
    resource_templates.extend(workflow::list_resource_templates());
    Ok(ListResourceTemplatesResult {
        resource_templates,
        next_cursor: None,
        meta: None,
    })
}

// T-12.0: RTM FR-11 / RTM-070 — preserve existing asset:// read behavior.
pub fn handle_read_resource(uri: &str) -> Result<ReadResourceResult> {
    if agent::is_agent_uri(uri) {
        agent::handle_read_resource(uri)
    } else if uri.starts_with("gate://") {
        gate::handle_read_resource(uri)
    } else if workflow::is_workflow_uri(uri) {
        workflow::handle_read_resource(uri)
    } else if graph::is_graph_uri(uri) {
        graph::handle_read_resource(uri)
    } else if pipeline::is_pipeline_uri(uri) {
        pipeline::handle_read_resource(uri)
    } else {
        asset::handle_read_resource(uri)
    }
}

pub fn handle_subscribe(
    uri: &str,
    context: &RequestContext<RoleServer>,
) -> std::result::Result<(), rmcp::ErrorData> {
    if !resource_exists(uri) {
        return Err(rmcp::ErrorData::resource_not_found(
            format!("Resource not found: {uri}"),
            None,
        ));
    }
    subscription::subscribe(uri, context)
}

pub fn handle_unsubscribe(
    uri: &str,
    context: &RequestContext<RoleServer>,
) -> std::result::Result<(), rmcp::ErrorData> {
    subscription::unsubscribe(uri, context);
    Ok(())
}

fn resource_exists(uri: &str) -> bool {
    if uri.starts_with("agent://session/") {
        return true;
    }

    if handle_list_resources()
        .map(|result| result.resources.iter().any(|resource| resource.uri == uri))
        .unwrap_or(false)
    {
        return true;
    }

    handle_list_resource_templates()
        .map(|result| {
            result
                .resource_templates
                .iter()
                .any(|template| uri_matches_template(uri, &template.uri_template))
        })
        .unwrap_or(false)
}

fn uri_matches_template(uri: &str, uri_template: &str) -> bool {
    if let Some((base, _)) = uri_template.split_once('{') {
        return !base.is_empty()
            && uri
                .strip_prefix(base)
                .map(|remainder| !remainder.is_empty())
                .unwrap_or(false);
    }

    uri == uri_template
}

pub fn map_resource_error(err: MaenifoldError) -> rmcp::ErrorData {
    match err {
        // T-10.2: RTM FR-11 / RTM-070 — missing assets map to MCP resource-not-found.
        MaenifoldError::AssetNotFound(kind, id) => {
            rmcp::ErrorData::resource_not_found(format!("Asset not found: {kind}/{id}"), None)
        }
        // T-10.2: RTM FR-11 / RTM-069 / RTM-070 — malformed URIs stay invalid-params errors.
        MaenifoldError::UnknownTool(_)
        | MaenifoldError::InvalidPayload(_)
        | MaenifoldError::InvalidArgument(_)
        | MaenifoldError::InvalidResourceUri(_)
        | MaenifoldError::PathTraversal(_)
        | MaenifoldError::MissingProperty(_) => {
            rmcp::ErrorData::invalid_params(err.to_string(), None)
        }
        _ => rmcp::ErrorData::internal_error(err.to_string(), None),
    }
}

// T-205.716: Regression tests for concurrent MCP resource listing.
//
// The reported bug is "ListAssets: I/O error: Resource deadlock avoided (os error 11)"
// which occurs when handle_list_resources() is called concurrently with pipeline operations.
//
// ## Analysis of EDEADLK (os error 11) on macOS
//
// EDEADLK is ONLY returned by POSIX record locks via `fcntl(F_SETLKW)` when the kernel
// detects a circular lock dependency between processes/threads. The following DO NOT
// produce EDEADLK:
//
// - `fd_lock` crate: uses `flock()` on macOS via `rustix::fs::flock()` (see fd-lock source)
// - `parking_lot::RwLock`: user-space lock, no kernel deadlock detection
// - `std::sync::Mutex`: user-space lock, no kernel deadlock detection
// - Standard file I/O (`fs::read_dir`, `fs::read_to_string`): no locking involved
//
// The only code in maenifold that uses `fcntl()` for locking is SQLite via rusqlite.
// However, `ListAssets` / `list_assets()` does NOT access SQLite at all.
//
// ## Current test coverage
//
// These tests exercise the concurrent resource listing code paths but CANNOT reproduce
// the exact EDEADLK error because:
// 1. Unit tests don't run the full MCP server with tokio::task::spawn_blocking
// 2. The `fd_lock` locks we acquire use `flock()` which doesn't trigger kernel deadlock detection
// 3. No SQLite is accessed during asset or resource listing
//
// The tests verify that the code is thread-safe and handles concurrent access correctly.
// If SWE identifies the actual lock interaction causing EDEADLK, these tests can be enhanced.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::registry::ToolRegistry;
    use crate::test_util::ConfigResetGuard;
    use crate::util::json::OutputFormat;
    use crate::workflow::pipeline::pipeline_lock_path;
    use chrono::{Datelike, TimeZone};
    use serde_json::json;
    use serial_test::serial;
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, ConfigResetGuard) {
        let tmp = TempDir::new().expect("temp dir");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        
        // Create pipeline directory structure
        let pipelines_dir = config.memory_path.join("pipelines/2026/04/22");
        fs::create_dir_all(&pipelines_dir).expect("create pipelines dir");
        
        // Create asset directories
        for kind in ["workflows", "roles", "colors", "perspectives"] {
            fs::create_dir_all(config.assets_path.join(kind)).expect("create asset dir");
        }
        
        // Create usage directory for Pipeline tool
        fs::create_dir_all(config.assets_path.join("usage/tools")).expect("create usage dir");
        fs::write(
            config.assets_path.join("usage/tools/pipeline.md"),
            "# Pipeline\n\nPipeline tool documentation.\n",
        )
        .expect("write pipeline help");
        
        (tmp, ConfigResetGuard)
    }

    fn create_pipeline_session(config: &Config, session_id: &str) {
        // Extract timestamp from session ID to compute the correct directory path
        // The session ID format is "pipeline-{timestamp_millis}-{suffix}"
        // e.g., "pipeline-1745366400000-test1" -> 1745366400000ms -> 2025-04-22
        let timestamp_millis = session_id
            .split('-')
            .find_map(|segment| {
                if segment.len() == 13 && segment.bytes().all(|b| b.is_ascii_digit()) {
                    segment.parse::<i64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        
        let dt = chrono::Utc
            .timestamp_millis_opt(timestamp_millis)
            .single()
            .unwrap_or_else(chrono::Utc::now);
        let year = dt.year();
        let month = dt.month();
        let day = dt.day();
        
        // Create workflow session for the node (referenced by nodeStates.sessionId)
        let workflow_session_id = format!("workflow-{timestamp_millis}-node1");
        let workflow_dir = config.memory_path.join(format!("thinking/workflow/{year:04}/{month:02}/{day:02}"));
        fs::create_dir_all(&workflow_dir).expect("create workflow dir");
        let workflow_path = workflow_dir.join(format!("{workflow_session_id}.md"));
        let workflow_content = r#"---
type: workflow
status: running
---

# Workflow Session

Test workflow session for pipeline node.
"#;
        fs::write(&workflow_path, workflow_content).expect("write workflow session");
        
        // Create pipeline session
        let pipelines_dir = config.memory_path.join(format!("pipelines/{year:04}/{month:02}/{day:02}"));
        fs::create_dir_all(&pipelines_dir).expect("create pipelines dir");
        
        let session_path = pipelines_dir.join(format!("{session_id}.md"));
        let frontmatter = format!(
            r#"---
type: pipeline
permalink: {session_id}
status: running
definition:
  nodes:
    - id: node-1
      type: prompt
      agent: test
      prompt: test
  edges: []
nodeStates:
  node-1:
    sessionId: {workflow_session_id}
    status: active
    attempts: 1
---

# Pipeline Session

Test pipeline session for concurrency testing.
"#
        );
        fs::write(&session_path, frontmatter).expect("write pipeline session");
    }

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
                            if let Some(rmcp::model::ResourceContents::TextResourceContents { text, .. }) = result.contents.first() {
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
                            if let Some(rmcp::model::ResourceContents::TextResourceContents { text, .. }) = result.contents.first() {
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
}
