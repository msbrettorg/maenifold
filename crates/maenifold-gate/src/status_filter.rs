use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusFilter {
    pub state: Option<String>,
    pub ready_only: bool,
}

impl StatusFilter {
    pub fn matches(&self, row: &Value) -> bool {
        if self.ready_only
            && (row["location"].as_str() != Some("todo") || row["state"].as_str() != Some("pending"))
        {
            return false;
        }

        match &self.state {
            Some(expected) => row["state"].as_str() == Some(expected.as_str()),
            None => true,
        }
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
        };
        assert!(filter.matches(&json!({ "state": "pending", "location": "todo" })));
        assert!(!filter.matches(&json!({ "state": "pending", "location": "done" })));
        assert!(!filter.matches(&json!({ "state": "swe", "location": "todo" })));
    }

    #[test]
    fn state_filter_matches_requested_state() {
        let filter = StatusFilter {
            state: Some("red-team".to_string()),
            ready_only: false,
        };
        assert!(filter.matches(&json!({ "state": "red-team", "location": "todo" })));
        assert!(!filter.matches(&json!({ "state": "swe", "location": "todo" })));
    }
}
