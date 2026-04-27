//! Token refresh-timing helpers ported from agency-cowork's TokenManager.
//!
//! Reference: `skills/teams/scripts/api/auth.py` L78-L181 (`TokenManager`).

use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::teams::TokenInfo;
use crate::teams::token_provider::{
    MAX_REFRESH_BUFFER_SECS, MIN_REFRESH_BUFFER_SECS, REFRESH_BUFFER_RATIO,
};

const MIN_VALIDITY_REUSE_SECONDS: i64 = 30;

/// Compute the proactive-refresh buffer for a token whose remaining lifetime is `lifetime`.
/// Port of `TokenManager._set_token` clamp logic in agency-cowork
/// `skills/teams/scripts/api/auth.py` L131-L134.
pub fn refresh_buffer_for(lifetime: Duration) -> Duration {
    let scaled_secs = lifetime.as_secs_f64() * REFRESH_BUFFER_RATIO;
    let bounded = scaled_secs
        .max(MIN_REFRESH_BUFFER_SECS as f64)
        .min(MAX_REFRESH_BUFFER_SECS as f64);
    Duration::from_secs(bounded as u64)
}

/// Seconds until proactive refresh is due (negative when already overdue).
/// Port of `TokenManager.seconds_until_refresh` in agency-cowork
/// `skills/teams/scripts/api/auth.py` L107-L111.
pub fn seconds_until_refresh(token: &TokenInfo, now: DateTime<Utc>) -> i64 {
    let remaining = token.expires_at.signed_duration_since(now).num_seconds();
    let lifetime = Duration::from_secs(remaining.max(0) as u64);
    let buffer = refresh_buffer_for(lifetime).as_secs() as i64;
    remaining - buffer
}

/// True when the proactive-refresh deadline has passed.
/// Port of `TokenManager.refresh_due` in agency-cowork
/// `skills/teams/scripts/api/auth.py` L118-L121.
pub fn refresh_due(token: &TokenInfo, now: DateTime<Utc>) -> bool {
    seconds_until_refresh(token, now) <= 0
}

/// Seconds remaining until the JWT `exp` (0 if already expired).
/// Port of `TokenManager.expires_in` in agency-cowork
/// `skills/teams/scripts/api/auth.py` L101-L105.
pub fn expires_in(token: &TokenInfo, now: DateTime<Utc>) -> i64 {
    token
        .expires_at
        .signed_duration_since(now)
        .num_seconds()
        .max(0)
}

/// True when the cached token still has the minimum reuse TTL.
/// Port of `TokenManager.is_valid` in agency-cowork
/// `skills/teams/scripts/api/auth.py` L113-L116.
pub fn is_valid(token: &TokenInfo, now: DateTime<Utc>) -> bool {
    expires_in(token, now) > MIN_VALIDITY_REUSE_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn token_expiring_at(exp: DateTime<Utc>) -> TokenInfo {
        TokenInfo {
            access_token: "tok".into(),
            expires_at: exp,
            refresh_token: None,
        }
    }

    #[test]
    fn refresh_buffer_clamps_short_lifetime_up_to_minimum() {
        // 30s lifetime → 3s scaled → clamped to 60s minimum.
        assert_eq!(refresh_buffer_for(Duration::from_secs(30)).as_secs(), 60);
    }

    #[test]
    fn refresh_buffer_clamps_long_lifetime_down_to_maximum() {
        // 1h lifetime → 360s scaled → clamped to 300s maximum.
        assert_eq!(refresh_buffer_for(Duration::from_secs(3600)).as_secs(), 300);
    }

    #[test]
    fn refresh_buffer_uses_ratio_when_within_bounds() {
        // 1500s → 150s within [60, 300].
        assert_eq!(refresh_buffer_for(Duration::from_secs(1500)).as_secs(), 150);
    }

    #[test]
    fn refresh_due_true_after_deadline() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        // exp 30s out, 60s minimum buffer ⇒ refresh due 30s ago.
        let token = token_expiring_at(now + chrono::Duration::seconds(30));
        assert!(refresh_due(&token, now));
        assert_eq!(seconds_until_refresh(&token, now), -30);
    }

    #[test]
    fn refresh_due_false_when_buffer_not_reached() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        // exp 1h out, 300s buffer ⇒ refresh in 3300s.
        let token = token_expiring_at(now + chrono::Duration::seconds(3600));
        assert!(!refresh_due(&token, now));
        assert_eq!(seconds_until_refresh(&token, now), 3300);
    }

    #[test]
    fn is_valid_requires_reuse_ttl() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let stale = token_expiring_at(now + chrono::Duration::seconds(10));
        let fresh = token_expiring_at(now + chrono::Duration::seconds(120));
        assert!(!is_valid(&stale, now));
        assert!(is_valid(&fresh, now));
    }
}
