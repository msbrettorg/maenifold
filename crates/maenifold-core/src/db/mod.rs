// T-12.0: RTM NFR-1 — preserve the existing `crate::db` entrypoint while delegating
// implementation details to focused submodules.
mod connection;
mod pipeline_lookup;
mod schema;
mod vec0;

pub use connection::{open_connection, run_full_maintenance, run_light_maintenance};
pub use schema::GraphDatabase;
pub use vec0::{ensure_vec0_tables, get_model_metadata};
// T-9.2 / T-3.1: RTM FR-8, FR-6 — shared vec0 extension helpers used across search,
// similar, sync, and repair modules to eliminate duplication and loading gaps.
pub(crate) use vec0::{load_vec0_extension, vec0_extension_available, verify_vec0_load};
pub(crate) use pipeline_lookup::lookup_pipeline_degraded_json;
