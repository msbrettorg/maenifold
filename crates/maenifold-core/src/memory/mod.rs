mod access;
mod editing;
mod editing_helpers;
mod lifecycle;
mod listing;
mod support;

use crate::decay::{
    compute_decay_weight as decay_compute_decay_weight,
    get_decay_weight_for_file as decay_get_decay_weight_for_file,
};

pub use access::{read_memory, write_memory};
pub use editing::edit_memory;
pub use lifecycle::{delete_memory, move_memory};
pub use listing::list_memories;
pub use support::{get_help, sanitize_user_input};

// T-12.5: RTM FR-25.2 keeps `maenifold_core::memory` backward-compatible
// after splitting [[memory.rs]] into focused submodules.
pub fn compute_decay_weight(
    file_path: &str,
    last_accessed: Option<&str>,
    config: &crate::config::Config,
    status: Option<&str>,
) -> f64 {
    decay_compute_decay_weight(file_path, last_accessed, config, status)
}

// T-12.5: RTM FR-25.2 preserves the legacy decay helper facade used by
// tests and downstream callers.
pub fn get_decay_weight_for_file(file_identifier: &str, config: &crate::config::Config) -> f64 {
    decay_get_decay_weight_for_file(file_identifier, config)
}
