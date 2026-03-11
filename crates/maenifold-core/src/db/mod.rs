// T-12.0: RTM NFR-1 — preserve the existing `crate::db` entrypoint while delegating
// implementation details to focused submodules.
mod connection;
mod schema;
mod vec0;

pub use connection::{open_connection, run_full_maintenance, run_light_maintenance};
pub use schema::GraphDatabase;
pub use vec0::{ensure_vec0_tables, get_model_metadata};
