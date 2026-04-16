//! Aether - Universal Runtime Control Plane
//!
//! One spec. Four runtimes. One tool.

pub mod adapters;
pub mod ai;
pub mod api;
pub mod audit;
pub mod backup;
pub mod completions;
pub mod compose;
pub mod config;
pub mod cost;
pub mod dependencies;
pub mod drift;
pub mod engine;
pub mod environments;
pub mod events;
pub mod gitops;
pub mod health;
pub mod helm;
pub mod metrics;
pub mod migration;
pub mod orchestrator;
pub mod output;
pub mod plugin;
pub mod policy;
pub mod rbac;
pub mod resources;
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

/// Result type alias for Aether
pub type Result<T> = anyhow::Result<T>;
