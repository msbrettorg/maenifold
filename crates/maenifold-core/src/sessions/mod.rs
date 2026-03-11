mod assumption_ledger;
mod cleanup;
mod support;
mod validation;

// T-12.7: RTM NFR-1 - preserve the `sessions` public surface while splitting the
// implementation into cohesive submodules that stay below the LOC budget.
pub use assumption_ledger::assumption_ledger;
pub use cleanup::cleanup_abandoned_sessions;
pub use validation::{
    validate_assumption_append_input, validate_assumption_read_input, validate_assumption_status,
    validate_assumption_update_input,
};

pub(crate) use support::{
    date_parts_from_session_id, display_utc_now, extract_session_timestamp_millis,
    frontmatter_string, load_help_doc, now_iso8601_roundtrip, workflow_session_uri,
};
