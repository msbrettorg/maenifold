// T-01.16: Facade for the graph::analyzer module.
//
// Implementation lives in sibling submodules; this file only re-exports the
// public surface so external call sites (graph::random_walk, graph::visualize)
// continue to compile unchanged.

mod render;
mod subgraph;
mod visualize;
mod walk;

#[cfg(test)]
mod tests;

pub use visualize::visualize;
pub use walk::random_walk;
