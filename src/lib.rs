//! Orchestr8 - Universal Runtime Control Plane
//!
//! One spec. Four runtimes. One tool.

pub mod adapters;
pub mod api;
pub mod backup;
pub mod completions;
pub mod cost;
pub mod engine;
pub mod metrics;
pub mod migration;
pub mod runtime;
pub mod spec;
pub mod state;
pub mod ui;

pub use runtime::Runtime;
pub use spec::Workload;

/// Result type alias for Orchestr8
pub type Result<T> = anyhow::Result<T>;
