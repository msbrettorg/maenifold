use super::wikilinks::{extract_wikilinks, normalize_concept};
use crate::error::Result;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// T-15.24: RTM FR-19 prefix-pattern support for WikiLink filtering.
#[derive(Debug, Clone)]
struct CachedWikiLinkFilter {
    modified: SystemTime,
    /// Exact-match entries (keys without trailing `*`).
    blocked_exact: BTreeMap<String, String>,
    /// Prefix-match entries (keys that ended with `*`, with the `*` stripped and normalized).
    blocked_prefixes: Vec<(String, String)>,
}

// T-10.16: RTM FR-19 keeps `.wikilink-filter.json` reads behind an mtime cache.
static WIKILINK_FILTER_CACHE: Lazy<Mutex<BTreeMap<PathBuf, CachedWikiLinkFilter>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

#[derive(Debug, Clone)]
pub struct WikiLinkFilter {
    filter_path: PathBuf,
}

impl WikiLinkFilter {
    pub fn new(memory_path: &Path) -> Self {
        Self {
            // T-10.16: RTM FR-19 matches C#'s shared-root `.wikilink-filter.json` path.
            filter_path: memory_path.join(".wikilink-filter.json"),
        }
    }

    pub fn check(&self, content: &str) -> std::result::Result<(), Vec<(String, String)>> {
        self.load_blocked_concepts()
            .map(|cached| self.check_against(content, &cached))
            .unwrap_or(Ok(()))
    }

    pub fn try_check(
        &self,
        content: &str,
    ) -> Result<std::result::Result<(), Vec<(String, String)>>> {
        let cached = self.load_blocked_concepts()?;
        Ok(self.check_against(content, &cached))
    }

    /// Returns true if the given pre-normalized concept name is blocked by the filter.
    /// Gracefully returns false if the filter file is missing or unreadable.
    /// Supports exact and prefix-pattern matches per RTM-151, RTM-152, and RTM-155.
    // T-22.1: RTM FR-19 (RTM-151, RTM-152, RTM-155) exposes single-concept filter checks.
    pub fn is_concept_blocked(&self, concept: &str) -> bool {
        self.load_blocked_concepts()
            .map(|cached| {
                // Exact match first
                if cached.blocked_exact.contains_key(concept) {
                    return true;
                }
                // Prefix scan
                for (prefix, _reason) in &cached.blocked_prefixes {
                    if concept.starts_with(prefix.as_str()) {
                        return true;
                    }
                }
                false
            })
            .unwrap_or(false)
    }

    // T-15.24: RTM FR-19 check_against supports both exact and prefix matching.
    fn check_against(
        &self,
        content: &str,
        cached: &CachedWikiLinkFilter,
    ) -> std::result::Result<(), Vec<(String, String)>> {
        if cached.blocked_exact.is_empty() && cached.blocked_prefixes.is_empty() {
            return Ok(());
        }

        let blocked_links = extract_wikilinks(content)
            .into_iter()
            .filter_map(|concept| {
                // T-15.24: exact match first, then prefix scan.
                if let Some(reason) = cached.blocked_exact.get(&concept) {
                    return Some((concept.clone(), reason.clone()));
                }
                for (prefix, reason) in &cached.blocked_prefixes {
                    if concept.starts_with(prefix.as_str()) {
                        return Some((concept.clone(), reason.clone()));
                    }
                }
                None
            })
            .collect::<Vec<_>>();

        if blocked_links.is_empty() {
            Ok(())
        } else {
            Err(blocked_links)
        }
    }

    fn load_blocked_concepts(&self) -> Result<CachedWikiLinkFilter> {
        let empty = CachedWikiLinkFilter {
            modified: SystemTime::UNIX_EPOCH,
            blocked_exact: BTreeMap::new(),
            blocked_prefixes: Vec::new(),
        };

        // T-10.16: RTM FR-19 rejects symlinked `.wikilink-filter.json` files so policy stays rooted in memory.
        let metadata = match fs::symlink_metadata(&self.filter_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                WIKILINK_FILTER_CACHE.lock().remove(&self.filter_path);
                return Ok(empty);
            }
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            WIKILINK_FILTER_CACHE.lock().remove(&self.filter_path);
            return Ok(empty);
        }
        let modified = metadata.modified()?;

        if let Some(cached) = WIKILINK_FILTER_CACHE.lock().get(&self.filter_path).cloned()
            && cached.modified == modified
        {
            return Ok(cached);
        }

        // T-15.24: RTM FR-19 partition entries into exact matches and prefix patterns.
        let mut blocked_exact = BTreeMap::new();
        let mut blocked_prefixes = Vec::new();

        // T-10.16: RTM FR-19 parses the JSON dotfile only when mtime changes.
        for (concept, reason) in serde_json::from_str::<BTreeMap<String, String>>(
            &fs::read_to_string(&self.filter_path)?,
        )? {
            if let Some(prefix_raw) = concept.strip_suffix('*') {
                // T-22.5-fix: RTM FR-19 (RTM-152) normalize prefix without
                // trim_matches('-'), preserving trailing separators like `t-`.
                let normalized = prefix_raw.trim().to_lowercase();
                if normalized.is_empty() {
                    eprintln!(
                        "Warning: skipping empty prefix in .wikilink-filter.json entry: {concept}"
                    );
                    continue;
                }
                blocked_prefixes.push((normalized, reason));
            } else {
                let normalized = normalize_concept(&concept);
                if normalized.is_empty() {
                    eprintln!(
                        "Warning: skipping empty concept in .wikilink-filter.json entry: {concept}"
                    );
                    continue;
                }
                blocked_exact.insert(normalized, reason);
            }
        }

        let cached = CachedWikiLinkFilter {
            modified,
            blocked_exact,
            blocked_prefixes,
        };

        WIKILINK_FILTER_CACHE
            .lock()
            .insert(self.filter_path.clone(), cached.clone());

        Ok(cached)
    }
}

#[cfg(test)]
mod tests {
    use super::WikiLinkFilter;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    #[test]
    fn wikilink_filter_allows_content_when_filter_file_is_missing() {
        let tmp = TempDir::new().expect("tmp");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Allowed [[Agent]] content");

        assert!(matches!(result, Ok(Ok(()))));
    }

    #[test]
    fn wikilink_filter_blocks_normalized_concepts_with_reasons() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter.json"),
            r#"{"agent":"Too generic - hub concept","system":"Also blocked"}"#,
        )
        .expect("write filter");

        let result = WikiLinkFilter::new(tmp.path())
            .try_check("Testing [[Agent]] and [[system]] in one note");

        assert_eq!(
            result.expect("filter read"),
            Err(vec![
                ("agent".to_string(), "Too generic - hub concept".to_string()),
                ("system".to_string(), "Also blocked".to_string()),
            ])
        );
    }

    #[test]
    fn wikilink_filter_returns_error_for_invalid_json_in_try_check() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter.json"),
            "not valid json {{{",
        )
        .expect("write filter");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Testing [[agent]] in one note");

        assert!(result.is_err(), "invalid JSON should error: {result:?}");
    }

    #[test]
    fn wikilink_filter_skips_empty_normalized_concepts() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter.json"),
            r#"{"   ":"missing concept","agent":"blocked"}"#,
        )
        .expect("write filter");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Testing [[agent]] in one note");

        assert_eq!(
            result.expect("filter read"),
            Err(vec![("agent".to_string(), "blocked".to_string())])
        );
    }

    #[cfg(unix)]
    #[test]
    fn wikilink_filter_ignores_symlinked_filter_files() {
        let tmp = TempDir::new().expect("tmp");
        let external = tmp.path().join("external-filter");
        std::fs::write(&external, r#"{"agent":"blocked"}"#).expect("write external filter");
        unix_fs::symlink(&external, tmp.path().join(".wikilink-filter.json"))
            .expect("create symlink");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Testing [[agent]] in one note");

        assert!(matches!(result, Ok(Ok(()))));
    }

    // T-15.24: RTM FR-19 prefix pattern tests below.

    #[test]
    fn wikilink_filter_prefix_pattern_blocks_matching_concepts() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter.json"),
            r#"{"t-*":"TODO backlog item IDs; process metadata not domain concepts"}"#,
        )
        .expect("write filter");

        let result =
            WikiLinkFilter::new(tmp.path()).try_check("Referencing [[T-14.1]] in this note");

        assert_eq!(
            result.expect("filter read"),
            Err(vec![(
                "t-14.1".to_string(),
                "TODO backlog item IDs; process metadata not domain concepts".to_string(),
            )])
        );
    }

    #[test]
    fn wikilink_filter_prefix_pattern_blocks_rtm_ids() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter.json"),
            r#"{"rtm-*":"RTM line item IDs; process metadata not domain concepts"}"#,
        )
        .expect("write filter");

        let result = WikiLinkFilter::new(tmp.path()).try_check("See [[RTM-042]] for details");

        assert_eq!(
            result.expect("filter read"),
            Err(vec![(
                "rtm-042".to_string(),
                "RTM line item IDs; process metadata not domain concepts".to_string(),
            )])
        );
    }

    #[test]
    fn wikilink_filter_exact_and_prefix_entries_work_together() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter.json"),
            r#"{"agent":"Too generic","t-*":"Process metadata","fr-*":"Requirement IDs"}"#,
        )
        .expect("write filter");

        let result = WikiLinkFilter::new(tmp.path())
            .try_check("Note about [[Agent]] and [[T-15.2]] and [[FR-31.14]]");

        let blocked = result.expect("filter read").unwrap_err();
        assert_eq!(blocked.len(), 3);
        assert!(blocked.contains(&("agent".to_string(), "Too generic".to_string())));
        assert!(blocked.contains(&("t-15.2".to_string(), "Process metadata".to_string())));
        assert!(blocked.contains(&("fr-31.14".to_string(), "Requirement IDs".to_string())));
    }

    #[test]
    fn wikilink_filter_exact_key_does_not_match_as_prefix() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter.json"),
            r#"{"test":"Exact only"}"#,
        )
        .expect("write filter");

        let result =
            WikiLinkFilter::new(tmp.path()).try_check("Note about [[test]] and [[testing]]");

        // Only "test" should be blocked, not "testing"
        assert_eq!(
            result.expect("filter read"),
            Err(vec![("test".to_string(), "Exact only".to_string())])
        );
    }

    #[test]
    fn wikilink_filter_skips_empty_prefix_after_normalization() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter.json"),
            r#"{"   *":"empty prefix should be skipped","agent":"blocked"}"#,
        )
        .expect("write filter");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Testing [[agent]] in one note");

        // The whitespace-only prefix is skipped; only "agent" exact match fires.
        assert_eq!(
            result.expect("filter read"),
            Err(vec![("agent".to_string(), "blocked".to_string())])
        );
    }
}
