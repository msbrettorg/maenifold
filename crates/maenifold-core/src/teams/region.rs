//! Chatsvc region discovery via API probe.
//!
//! Ports the region-probing logic from agency-cowork's Teams monitor:
//! `agency-cowork/skills/teams/scripts/monitor/message_handler.py`
//! - `probe_chatsvc_region()` (L151-228)
//! - `_extract_region_from_url()` (L244-252)
//!
//! The Microsoft Teams chatsvc API is region-sharded. A token issued for one
//! tenant may be served from `noam-pilot2`, `emea`, etc. When the configured
//! region is wrong, chatsvc responds with a redirect (301/302/307) or 404
//! whose `Location` header points at the correct shard. We probe a known-safe
//! endpoint (`48:notes`, the user's self-chat) to discover the canonical
//! region and report any change to the caller.
//!
//! The HTTP client is configured with `redirect::Policy::none()` so the
//! redirect can be inspected; reqwest would otherwise transparently follow
//! it and we would lose the routing information we need.
//!
//! Wired into the supervisor / handler by sub-dispatch B (T-238.52).
#![allow(dead_code)]

use std::sync::LazyLock;

use regex::Regex;
use reqwest::redirect;
use tracing::{debug, info, warn};

const TEAMS_CLOUD_HOST: &str = "https://teams.cloud.microsoft";
/// URL-encoded form of `48:notes` (the self-chat conversation id).
const PROBE_CONVERSATION: &str = "48%3Anotes";

/// Captures the region slug between `/api/chatsvc/` and `/v<digit>`.
///
/// Mirrors the Python regex at agency-cowork message_handler.py L251.
static REGION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)/api/chatsvc/([a-z0-9_-]+)/v\d").expect("chatsvc region regex")
});

/// Outcome of a region probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    /// Probe returned 2xx and the configured region is correct.
    /// The wrapped string is the confirmed region.
    Confirmed(String),
    /// Probe revealed a different region; caller should adopt it.
    Updated(String),
    /// Probe completed with no usable signal; keep current region.
    Unchanged,
}

/// Extract the chatsvc region slug from a URL or response body.
///
/// Matches patterns like `https://.../api/chatsvc/noam-pilot2/v1/...`.
/// Returns `None` if no chatsvc URL is present.
///
/// Ported from `_extract_region_from_url` (agency-cowork message_handler.py L244-252).
pub fn extract_region_from_url(s: &str) -> Option<String> {
    REGION_RE
        .captures(s)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_lowercase())
}

/// Probe the chatsvc API to discover the canonical region for `token`.
///
/// Issues a `GET .../users/ME/conversations/48:notes/messages?pageSize=1` against
/// the `current_region` shard and inspects the response:
///
/// * `200/201` → region is confirmed (also honours an `X-MS-Region` header hint).
/// * `301/302/307/404` with a `Location` header (or body) referencing a different
///   `/api/chatsvc/<region>/v<n>/...` URL → returns `Updated(new_region)`.
/// * Any other outcome (network error, opaque status) → `Unchanged`.
///
/// Ported from `probe_chatsvc_region` (agency-cowork message_handler.py L151-228).
/// Persistence is deliberately the caller's concern; this function is pure I/O.
pub async fn probe_chatsvc_region(token: &str, current_region: &str) -> ProbeResult {
    if token.is_empty() {
        debug!("probe_chatsvc_region: empty token, skipping");
        return ProbeResult::Unchanged;
    }

    let url = format!(
        "{TEAMS_CLOUD_HOST}/api/chatsvc/{current_region}/v1/users/ME/conversations/{PROBE_CONVERSATION}/messages?pageSize=1"
    );

    let client = match reqwest::Client::builder()
        .redirect(redirect::Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("probe_chatsvc_region: client build failed: {e}");
            return ProbeResult::Unchanged;
        }
    };

    let resp = match client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("Region probe failed: {e} — keeping '{current_region}'");
            return ProbeResult::Unchanged;
        }
    };

    let status = resp.status();
    let status_code = status.as_u16();

    if status.is_success() {
        // Honour optional canonical-region header hint even on 2xx.
        let header_hint = resp
            .headers()
            .get("X-MS-Region")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_lowercase());
        if let Some(canon) = header_hint {
            if !canon.is_empty() && canon != current_region.to_lowercase() {
                info!("Region probe: header suggests '{canon}' instead of '{current_region}'");
                return ProbeResult::Updated(canon);
            }
        }
        info!("Region probe: '{current_region}' confirmed (status {status_code})");
        return ProbeResult::Confirmed(current_region.to_string());
    }

    // 301/302/307/404 may carry a Location pointing at the right shard.
    let location = resp
        .headers()
        .get("Location")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_default();
    let body = resp.text().await.unwrap_or_default();

    let redirect_like = matches!(status_code, 301 | 302 | 307 | 404);
    let body_hint = body.to_lowercase().contains("different cloud");

    if redirect_like && (!location.is_empty() || body_hint) {
        let candidate = if location.is_empty() {
            &body
        } else {
            &location
        };
        if let Some(new_region) = extract_region_from_url(candidate) {
            if new_region != current_region.to_lowercase() {
                info!(
                    "Region probe: discovered '{new_region}' (was '{current_region}', status {status_code})"
                );
                return ProbeResult::Updated(new_region);
            }
        }
    }

    info!("Region probe: status {status_code} with no redirect hint — keeping '{current_region}'");
    ProbeResult::Unchanged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_region_from_typical_url() {
        let url = "https://teams.cloud.microsoft/api/chatsvc/noam-pilot2/v1/users/ME";
        assert_eq!(extract_region_from_url(url).as_deref(), Some("noam-pilot2"));
    }

    #[test]
    fn extracts_region_case_insensitively_and_lowercases() {
        let url = "https://teams.cloud.microsoft/api/CHATSVC/EMEA/V1/foo";
        assert_eq!(extract_region_from_url(url).as_deref(), Some("emea"));
    }

    #[test]
    fn extracts_region_from_error_body() {
        let body = r#"{"error":"Try https://teams.cloud.microsoft/api/chatsvc/apac/v1/x"}"#;
        assert_eq!(extract_region_from_url(body).as_deref(), Some("apac"));
    }

    #[test]
    fn returns_none_when_no_chatsvc_url_present() {
        assert_eq!(extract_region_from_url("hello world"), None);
        assert_eq!(extract_region_from_url("https://example.com/v1"), None);
    }
}
