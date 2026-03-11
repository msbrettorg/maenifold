mod diagnostics;
mod metadata;
mod scoring;

pub(crate) use diagnostics::{
    emit_degraded_decay_diagnostic, emit_stderr_line, redact_diagnostic_path,
};
pub(crate) use metadata::{
    get_decay_weight_for_file, get_decay_weight_for_file_with_connection, parse_datetime,
    read_created_date,
};
pub(crate) use scoring::{compute_decay_weight, compute_decay_weight_with_created};

// T-12.5: RTM NFR-1 keeps the decay API surface stable while the implementation
// moves from a flat file to focused submodules.
