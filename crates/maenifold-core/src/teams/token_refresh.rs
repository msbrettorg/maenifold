//! Background token-refresh loop for the Teams supervisor (T-238.50).
//!
//! Port of agency-cowork's `_token_refresh_loop` and `_compute_refresh_sleep`
//! in `skills/teams/scripts/monitor/service.py` L719-L760 and L330-L344.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::teams::ReRegisterHandle;
use crate::teams::TokenInfo;
use crate::teams::set_chatsvc_token;
use crate::teams::token_provider::{MAX_REFRESH_SLEEP, TeamsTokenProvider};
use crate::teams::token_refresh_meta::{expires_in, is_valid, refresh_due, seconds_until_refresh};

const FAILURE_BACKOFF: Duration = Duration::from_secs(30);

/// Spawn the token-refresh loop. Returns a watch receiver carrying the
/// latest chatsvcagg token (the runner reads this on every connect /
/// periodic re-register) and a join handle for graceful shutdown.
///
/// Behavior mirrors `_token_refresh_loop` in agency-cowork
/// `skills/teams/scripts/monitor/service.py` L719-L760: sleep until the
/// refresh deadline, force-refresh, push the new token to the watch +
/// chatsvc cell, then immediately re-register with Trouter so we don't
/// wait for the next 45-minute periodic re-registration tick.
pub fn spawn_refresh_loop(
    initial: TokenInfo,
    provider: Arc<TeamsTokenProvider>,
    fallback_refresh: Duration,
    tx: watch::Sender<String>,
    re_register: ReRegisterHandle,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    set_chatsvc_token(initial.access_token.clone());
    let _ = tx.send(initial.access_token.clone());
    let mut current = initial;

    tokio::spawn(async move {
        loop {
            let sleep_for = compute_refresh_sleep(&current, fallback_refresh);
            tokio::select! {
                _ = sleep(sleep_for) => {}
                _ = cancel.cancelled() => break,
            }
            if !refresh_due(&current, Utc::now()) {
                continue;
            }
            tracing::info!("teams token refresh due; force-refreshing");
            match provider.acquire_token(true).await {
                Ok(acq) => {
                    let new_token = acq.token.access_token.clone();
                    if tx.send(new_token.clone()).is_err() {
                        tracing::warn!("teams token watch closed; refresh loop exiting");
                        break;
                    }
                    set_chatsvc_token(new_token.clone());
                    match re_register.re_register(&new_token).await {
                        Ok(()) => tracing::info!("re-registered with Trouter after token refresh"),
                        Err(err) => tracing::warn!(
                            error = %err,
                            "re-registration after token refresh failed"
                        ),
                    }
                    current = acq.token;
                    tracing::info!(
                        next_refresh_s = seconds_until_refresh(&current, Utc::now()),
                        "teams token refreshed"
                    );
                }
                Err(err) if is_valid(&current, Utc::now()) => {
                    tracing::warn!(
                        %err,
                        remaining_s = expires_in(&current, Utc::now()),
                        "teams token refresh failed; reusing cached token"
                    );
                }
                Err(err) => {
                    tracing::error!(
                        %err,
                        "teams token expired and refresh failed; retrying in 30s"
                    );
                    let backoff = FAILURE_BACKOFF.min(fallback_refresh);
                    tokio::select! {
                        _ = sleep(backoff) => {}
                        _ = cancel.cancelled() => break,
                    }
                }
            }
        }
    })
}

/// Port of `_compute_refresh_sleep` in agency-cowork
/// `skills/teams/scripts/monitor/service.py` L330-L344.
fn compute_refresh_sleep(token: &TokenInfo, fallback_refresh: Duration) -> Duration {
    let remaining = seconds_until_refresh(token, Utc::now());
    if remaining <= 0 {
        return Duration::ZERO;
    }
    let fallback = fallback_refresh.max(Duration::from_secs(30));
    Duration::from_secs(remaining as u64)
        .min(fallback)
        .min(MAX_REFRESH_SLEEP)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn token(exp_secs_from_now: i64) -> TokenInfo {
        let now = Utc::now();
        TokenInfo {
            access_token: "tok".into(),
            expires_at: now + chrono::Duration::seconds(exp_secs_from_now),
            refresh_token: None,
        }
    }

    #[test]
    fn compute_sleep_caps_at_max_buffer() {
        // 1h remaining → seconds_until_refresh ≈ 3300; capped to MAX_REFRESH_SLEEP=300.
        let sleep = compute_refresh_sleep(&token(3600), Duration::from_secs(3000));
        assert_eq!(sleep, Duration::from_secs(300));
    }

    #[test]
    fn compute_sleep_uses_fallback_when_smaller() {
        // 1h remaining, but fallback=120 floors below MAX.
        let sleep = compute_refresh_sleep(&token(3600), Duration::from_secs(120));
        assert_eq!(sleep, Duration::from_secs(120));
    }

    #[test]
    fn compute_sleep_zero_when_overdue() {
        let sleep = compute_refresh_sleep(&token(10), Duration::from_secs(120));
        assert_eq!(sleep, Duration::ZERO);
    }

    #[test]
    fn compute_sleep_floors_fallback_at_thirty_seconds() {
        // fallback=5s gets bumped to 30s minimum (matches Python `max(30, ...)`).
        let sleep = compute_refresh_sleep(&token(3600), Duration::from_secs(5));
        assert_eq!(sleep, Duration::from_secs(30));
    }

    #[test]
    fn token_struct_at_epoch_does_not_panic() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let t = TokenInfo {
            access_token: "x".into(),
            expires_at: now,
            refresh_token: None,
        };
        // Just exercise the seconds_until_refresh call to confirm no overflow.
        let _ = seconds_until_refresh(&t, now);
    }
}
