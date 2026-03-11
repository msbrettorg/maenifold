// T-12.0: RTM NFR-1
#[path = "access_boosting/support.rs"]
mod support;

use support::{
    READ_SLEEP_MS, TEST_FOLDER, TEST_MUTEX, create_and_sync_test_file, get_last_accessed,
    isolated_env, read_memory_output, rfc3339_days_ago, set_last_accessed,
};

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
#[test]
fn read_memory_updates_last_accessed_on_every_read() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = create_and_sync_test_file(
        &env,
        "Access Boosting Read Test",
        "Testing [[access-boosting]] behavior for [[ReadMemory]].",
        TEST_FOLDER,
    );

    let seeded = rfc3339_days_ago(2);
    set_last_accessed(&env.cfg, &uri, &seeded);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "ReadMemory should succeed: {read_result}"
    );

    let after_read_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert!(
        after_read_last_accessed.is_some(),
        "last_accessed should be set after ReadMemory"
    );
    assert!(
        after_read_last_accessed > initial_last_accessed,
        "last_accessed should become more recent after ReadMemory"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
#[test]
fn read_memory_updates_last_accessed_on_subsequent_reads() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = create_and_sync_test_file(
        &env,
        "Multiple Reads Test",
        "Testing [[access-boosting]] with multiple reads for [[decay-weighting]].",
        TEST_FOLDER,
    );

    let seeded = rfc3339_days_ago(2);
    set_last_accessed(&env.cfg, &uri, &seeded);

    let first_read_result = read_memory_output(&env, &uri);
    assert!(
        !first_read_result.starts_with("ERROR:"),
        "first ReadMemory should succeed: {first_read_result}"
    );
    let first_read_timestamp = get_last_accessed(&env.cfg, &uri).expect("first read timestamp");

    std::thread::sleep(std::time::Duration::from_millis(READ_SLEEP_MS));

    let second_read_result = read_memory_output(&env, &uri);
    assert!(
        !second_read_result.starts_with("ERROR:"),
        "second ReadMemory should succeed: {second_read_result}"
    );
    let second_read_timestamp = get_last_accessed(&env.cfg, &uri).expect("second read timestamp");

    assert!(
        second_read_timestamp > first_read_timestamp,
        "each ReadMemory call should refresh last_accessed"
    );
}

// T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1
#[test]
fn read_memory_by_title_updates_last_accessed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Title Based Access Test";
    let uri = create_and_sync_test_file(
        &env,
        title,
        "Testing [[access-boosting]] when reading by [[title]].",
        TEST_FOLDER,
    );

    let seeded = rfc3339_days_ago(2);
    set_last_accessed(&env.cfg, &uri, &seeded);
    let initial_last_accessed = get_last_accessed(&env.cfg, &uri);

    let read_result = read_memory_output(&env, title);
    assert!(
        !read_result.starts_with("ERROR:"),
        "ReadMemory by title should succeed: {read_result}"
    );

    let after_read_last_accessed = get_last_accessed(&env.cfg, &uri);
    assert!(
        after_read_last_accessed > initial_last_accessed,
        "reading by title should update last_accessed"
    );
}
