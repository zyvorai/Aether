//! Orchestr8 - Universal Runtime Control Plane
//!
//! One spec. Three runtimes. One tool.

pub mod adapters;
pub mod engine;
pub mod runtime;
pub mod spec;
pub mod state;
pub mod ui;

pub use runtime::Runtime;
pub use spec::Workload;

/// Result type alias for Orchestr8
pub type Result<T> = anyhow::Result<T>;
