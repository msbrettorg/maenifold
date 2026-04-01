// T-12.7: RTM NFR-1 - keep sync leader is_leader status coverage split from lease lifecycle checks.
#[allow(clippy::duplicate_mod)]
#[path = "support/sync_leader.rs"]
mod sync_leader_support;

use sync_leader_support::{
    ConfigOverrideReset, TEST_MUTEX, expire_lease, isolated_env, leader_election,
};

// T-4.5.1: RTM FR-30.8
#[test]
fn is_leader_returns_false_for_non_holder_with_active_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(!election.is_leader("holder-b").unwrap());
}

// T-4.5.1: RTM FR-30.8
#[test]
fn is_leader_returns_true_for_current_holder_with_active_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(election.is_leader("holder-a").unwrap());
}

// T-4.5.1: RTM FR-30.8
#[test]
fn fr30_3_is_leader_returns_false_when_no_lease_exists() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert!(!election.is_leader("holder-a").unwrap());
}

// T-4.5.1: RTM FR-30.8
#[test]
fn fr30_3_is_leader_returns_false_for_expired_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 60_000).expect("acquire");
    expire_lease(&cfg);

    assert!(!election.is_leader("holder-a").unwrap());
}
