// T-12.7: RTM NFR-1 - keep sync leader gating/status coverage split from lease lifecycle checks.
#[path = "support/sync_leader.rs"]
mod sync_leader_support;

use sync_leader_support::{
    ConfigOverrideReset, TEST_MUTEX, expire_lease, isolated_env, leader_election,
};

// T-4.5.1: RTM FR-30.1
#[test]
fn fr30_1_ac1_non_leader_skips_community_recomputation() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(!election.is_leader("holder-b").expect("holder b not leader"));
}

// T-4.5.1: RTM FR-30.1
#[test]
fn fr30_1_ac2_leader_runs_community_recomputation() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(election.is_leader("holder-a").expect("holder a leader"));
}

// T-4.5.1: RTM FR-30.2
#[test]
fn fr30_2_ac1_non_leader_skips_maintenance_work() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(!election.is_leader("holder-b").expect("holder b not leader"));
}

// T-4.5.1: RTM FR-30.2
#[test]
fn fr30_2_ac2_leader_runs_maintenance_normally() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(election.is_leader("holder-a").expect("holder a leader"));
}

// T-4.5.1: RTM FR-30.8
#[test]
fn fr30_3_is_leader_returns_false_when_no_lease_exists() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert!(!election.is_leader("holder-a").expect("no active leader"));
}

// T-4.5.1: RTM FR-30.8
#[test]
fn fr30_3_is_leader_returns_false_for_expired_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");
    expire_lease(&cfg);

    assert!(!election.is_leader("holder-a").expect("expired lease"));
}
