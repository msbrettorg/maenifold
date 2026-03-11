// T-12.7: RTM NFR-1 - keep sync leader lifecycle coverage split from gating/status checks.
#[path = "support/sync_leader.rs"]
mod sync_leader_support;

use sync_leader_support::{
    ConfigOverrideReset, TEST_MUTEX, current_lease, expire_lease, isolated_env, leader_election,
    short_sleep,
};

// T-8.1: RTM FR-30.3
#[test]
fn fr30_3_ac1_try_acquire_succeeds_on_empty_table() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (acquired, reason, epoch) = election.try_acquire("holder-a", 10).expect("acquire");

    assert!(acquired, "first acquire should succeed");
    assert_eq!(reason, "acquired");
    assert_eq!(epoch, 1);

    let lease = current_lease(&cfg).expect("lease row");
    assert_eq!(lease.holder_id, "holder-a");
    assert_eq!(lease.epoch, 1);
    assert!(lease.lease_expires_at_ms > 0);
    assert!(lease.lease_expires_at_ms > lease.updated_at_ms);
}

// T-8.1: RTM FR-30.3 / FR-30.6
#[test]
fn fr30_3_ac2_try_renew_fails_for_non_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (_, _, epoch) = election.try_acquire("holder-a", 10).expect("acquire");
    let before = current_lease(&cfg).expect("lease before failed renew");

    let renewed = election.try_renew("holder-b", epoch, 10).expect("renew");

    assert!(!renewed, "non-holder renew must fail");
    assert_eq!(current_lease(&cfg), Some(before));
}

// T-8.1: RTM FR-30.3
#[test]
fn fr30_3_ac3_try_acquire_fails_when_lease_active() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election.try_acquire("holder-a", 10).expect("first acquire"),
        (true, "acquired", 1)
    );

    let outcome = election
        .try_acquire("holder-b", 10)
        .expect("contended acquire");

    assert_eq!(outcome, (false, "held_by_other", 1));
    assert_eq!(
        current_lease(&cfg).expect("lease row").holder_id,
        "holder-a"
    );
}

// T-8.1: RTM FR-30.3 / FR-30.4
#[test]
fn fr30_3_ac4_try_acquire_succeeds_on_expired_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election
            .try_acquire("holder-a", 10)
            .expect("initial acquire"),
        (true, "acquired", 1)
    );
    let before = current_lease(&cfg).expect("initial lease");
    assert_eq!(before.epoch, 1);
    expire_lease(&cfg);

    let outcome = election
        .try_acquire("holder-b", 10)
        .expect("takeover acquire");

    assert_eq!(outcome, (true, "acquired", 2));
    let after = current_lease(&cfg).expect("successor lease");
    assert_eq!(after.holder_id, "holder-b");
    assert_eq!(after.epoch, before.epoch + 1);
}

// T-8.1: RTM FR-30.3 / FR-30.6
#[test]
fn fr30_3_ac5_try_renew_succeeds_for_current_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (_, _, epoch) = election.try_acquire("holder-a", 1).expect("acquire");
    let before = current_lease(&cfg).expect("lease before renew");
    short_sleep();

    let renewed = election.try_renew("holder-a", epoch, 5).expect("renew");

    assert!(renewed, "current holder should renew active lease");
    let after = current_lease(&cfg).expect("lease after renew");
    assert_eq!(after.holder_id, "holder-a");
    assert_eq!(after.epoch, before.epoch);
    assert!(after.lease_expires_at_ms >= before.lease_expires_at_ms);
    assert!(after.updated_at_ms >= before.updated_at_ms);
}

// T-8.1: RTM FR-30.4 / FR-30.7
#[test]
fn fr30_3_ac6_release_succeeds_for_current_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (_, _, epoch) = election.try_acquire("holder-a", 10).expect("acquire");
    let before = current_lease(&cfg).expect("lease before release");
    short_sleep();

    let released = election.release("holder-a", epoch).expect("release");

    assert!(
        released,
        "release should succeed for matching holder and epoch"
    );
    let released_lease = current_lease(&cfg).expect("released lease row");
    assert_eq!(released_lease.holder_id, "");
    assert_eq!(released_lease.epoch, epoch);
    assert_eq!(released_lease.lease_expires_at_ms, 0);
    assert!(released_lease.updated_at_ms >= before.updated_at_ms);
    assert!(!election.is_leader("holder-a").expect("released leader"));
    assert_eq!(
        election.try_acquire("holder-b", 10).expect("reacquire"),
        (true, "acquired", epoch + 1)
    );
    let reacquired_lease = current_lease(&cfg).expect("lease after reacquire");
    assert_eq!(reacquired_lease.holder_id, "holder-b");
    assert_eq!(reacquired_lease.epoch, epoch + 1);
}

// T-8.1: RTM FR-30.7
#[test]
fn fr30_3_ac7_release_noop_for_wrong_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (_, _, epoch) = election.try_acquire("holder-a", 10).expect("acquire");
    let before = current_lease(&cfg).expect("lease before wrong release");

    let released = election
        .release("holder-b", epoch)
        .expect("wrong holder release");

    assert!(!released, "wrong holder release must fail");
    assert_eq!(current_lease(&cfg), Some(before));
    assert!(election.is_leader("holder-a").expect("holder a leader"));
}

// T-8.1: RTM FR-30.4 / FR-30.7
#[test]
fn fr30_3_ac8_try_acquire_returns_reason_codes() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let acquired = election.try_acquire("holder-a", 10).expect("acquired");
    let renewed = election.try_acquire("holder-a", 10).expect("renewed");
    let contended = election.try_acquire("holder-b", 10).expect("contended");
    let forced = election
        .force_acquire("holder-c", 10)
        .expect("force acquire");

    assert_eq!(acquired, (true, "acquired", 1));
    assert_eq!(renewed, (true, "renewed", 1));
    assert_eq!(contended, (false, "held_by_other", 1));
    assert_eq!(forced, (true, "force_acquired", 2));
    assert_eq!(
        current_lease(&cfg).expect("lease after force").holder_id,
        "holder-c"
    );
}

// T-8.1: RTM FR-30.3
#[test]
fn fr30_3_try_acquire_idempotent_for_same_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let first = election.try_acquire("holder-a", 1).expect("first acquire");
    let before = current_lease(&cfg).expect("lease before idempotent acquire");
    short_sleep();
    let second = election.try_acquire("holder-a", 5).expect("second acquire");
    let after = current_lease(&cfg).expect("lease after idempotent acquire");

    assert_eq!(first, (true, "acquired", 1));
    assert_eq!(second, (true, "renewed", 1));
    assert_eq!(after.holder_id, "holder-a");
    assert_eq!(after.epoch, before.epoch);
    assert!(after.lease_expires_at_ms >= before.lease_expires_at_ms);
}

// T-8.1: RTM FR-30.6 / FR-30.8
#[test]
fn fr30_3_try_renew_fails_for_expired_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (_, _, epoch) = election.try_acquire("holder-a", 10).expect("acquire");
    expire_lease(&cfg);
    let before = current_lease(&cfg).expect("expired lease row");

    let renewed = election
        .try_renew("holder-a", epoch, 10)
        .expect("renew after expiry");

    assert!(!renewed, "expired lease must not renew");
    assert_eq!(current_lease(&cfg), Some(before));
    assert!(!election.is_leader("holder-a").expect("expired leader"));
}

// T-8.1: RTM FR-30.6
#[test]
fn fr30_3_try_renew_fails_for_stale_epoch() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (_, _, epoch) = election.try_acquire("holder-a", 10).expect("acquire");
    let before = current_lease(&cfg).expect("lease before stale renew");

    let renewed = election
        .try_renew("holder-a", epoch + 1, 10)
        .expect("renew with stale epoch");

    assert!(!renewed, "renew must fail when epoch does not match");
    assert_eq!(current_lease(&cfg), Some(before));
}
