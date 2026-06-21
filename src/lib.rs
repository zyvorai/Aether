// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Aether - Universal Runtime Control Plane
//!
//! One spec. Four runtimes. One tool.

pub mod adapters;
pub mod ai;
pub mod api;
pub mod audit;
pub mod backup;
pub mod backup_remote;
pub mod completions;
pub mod compose;
pub mod config;
pub mod zeus;
#[deprecated(note = "use crate::zeus")]
pub mod copilot {
    pub use crate::zeus::*;
}
pub mod cost;
pub mod dependencies;
pub mod drift;
pub mod ecosystem;
pub mod engine;
pub mod environments;
pub mod events;
pub mod fleet;
pub mod gitops;
pub mod ha;
pub mod health;
pub mod helm;
pub mod hosted;
pub mod intelligence;
pub mod kubecluster;
pub mod ldap;
pub mod legacy_workload_yaml;
pub mod metrics;
pub mod migration;
pub mod mock_idp;
pub mod observability;
pub mod oidc;
pub mod opa;
pub mod orchestrator;
pub mod output;
pub mod plugin;
pub mod policy;
pub mod ragnarok;
pub mod rbac;
pub mod resources;
pub mod runtime;
pub mod saml;
pub mod saml_c14n;
pub mod saml_decrypt;
pub mod saml_verify;
pub mod sbom;
pub mod scheduler;
pub mod secrets;
pub mod sla;
pub mod spec;
pub mod state;
pub mod state_postgres;
pub mod templates;
pub mod ui;
pub mod license;

pub use runtime::Runtime;
pub use spec::Workload;

/// Result type alias for Aether
pub type Result<T> = anyhow::Result<T>;
