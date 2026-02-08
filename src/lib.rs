//! Orchestr8 - Universal Runtime Control Plane
//!
//! One spec. Four runtimes. One tool.

pub mod adapters;
pub mod ai;
pub mod api;
pub mod audit;
pub mod backup;
pub mod completions;
pub mod config;
pub mod cost;
pub mod dependencies;
pub mod drift;
pub mod engine;
pub mod environments;
pub mod events;
pub mod metrics;
pub mod migration;
pub mod orchestrator;
pub mod policy;
pub mod runtime;
pub mod scheduler;
pub mod secrets;
pub mod sla;
pub mod spec;
pub mod state;
pub mod templates;
pub mod ui;

pub use runtime::Runtime;
pub use spec::Workload;

/// Result type alias for Orchestr8
pub type Result<T> = anyhow::Result<T>;
