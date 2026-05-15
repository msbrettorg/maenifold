use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusFilter {
    pub state: Option<String>,
    pub ready_only: bool,
    pub blocked_only: bool,
    pub include_done: bool,
}

impl StatusFilter {
    pub fn matches(&self, row: &Value) -> bool {
        if self.ready_only && row["ready"].as_bool() != Some(true) {
            return false;
        }

        if self.blocked_only
            && !row["blockedBy"]
                .as_array()
                .is_some_and(|blocked_by| !blocked_by.is_empty())
        {
            return false;
        }

        if let Some(expected) = &self.state {
            return row["state"].as_str() == Some(expected.as_str());
        }

        if !self.include_done
            && (row["state"].as_str() == Some("done") || row["state"].as_str() == Some("cancelled"))
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::StatusFilter;

    #[test]
    fn ready_only_matches_pending_todo_rows() {
        let filter = StatusFilter {
            state: None,
            ready_only: true,
            blocked_only: false,
            include_done: false,
        };
        assert!(filter.matches(&json!({ "ready": true, "state": "pending", "location": "todo" })));
        assert!(
            !filter.matches(&json!({ "ready": false, "state": "pending", "location": "todo" }))
        );
    }

    #[test]
    fn state_filter_matches_requested_state() {
        let filter = StatusFilter {
            state: Some("red-team".to_string()),
            ready_only: false,
            blocked_only: false,
            include_done: false,
        };
        assert!(filter.matches(&json!({ "state": "red-team", "location": "todo" })));
        assert!(!filter.matches(&json!({ "state": "swe", "location": "todo" })));
    }

    #[test]
    fn default_filter_excludes_done_rows() {
        let filter = StatusFilter {
            state: None,
            ready_only: false,
            blocked_only: false,
            include_done: false,
        };
        assert!(!filter.matches(&json!({ "state": "done", "location": "done" })));
        assert!(filter.matches(&json!({ "state": "pending", "location": "todo" })));
    }

    #[test]
    fn include_done_opt_in_restores_done_rows() {
        let filter = StatusFilter {
            state: None,
            ready_only: false,
            blocked_only: false,
            include_done: true,
        };
        assert!(filter.matches(&json!({ "state": "done", "location": "done" })));
    }

    #[test]
    fn state_filter_for_done_preserves_done_lookup() {
        let filter = StatusFilter {
            state: Some("done".to_string()),
            ready_only: false,
            blocked_only: false,
            include_done: false,
        };
        assert!(filter.matches(&json!({ "state": "done", "location": "done" })));
        assert!(!filter.matches(&json!({ "state": "pending", "location": "todo" })));
    }

    #[test]
    fn blocked_only_matches_rows_with_blockers() {
        let filter = StatusFilter {
            state: None,
            ready_only: false,
            blocked_only: true,
            include_done: false,
        };
        assert!(filter.matches(&json!({ "state": "pending", "blockedBy": ["T-1"] })));
        assert!(!filter.matches(&json!({ "state": "pending", "blockedBy": [] })));
    }

    #[test]
    fn ready_only_excludes_frontmatter_blocked_rows() {
        let filter = StatusFilter {
            state: None,
            ready_only: true,
            blocked_only: false,
            include_done: false,
        };
        assert!(!filter.matches(&json!({
            "ready": false,
            "state": "blocked",
            "blocked": true,
            "location": "todo"
        })));
    }
}
