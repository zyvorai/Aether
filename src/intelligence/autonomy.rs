// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Autonomy status — effective self-healing and auto-execute policy.

use crate::intelligence::policy::{AutonomyPolicy, AutonomyTier};
use crate::spec::{AutonomyLevel, Workload};
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyEnvFlags {
    pub aether_auto_restart: bool,
    pub aether_auto_reconcile: bool,
    pub reconciliation_auto_reconcile: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadAutonomyOverride {
    pub workload: String,
    pub migration: String,
    pub healing: String,
    pub evolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyStatusReport {
    pub generated_at: String,
    pub effective_policy: AutonomyPolicy,
    pub env: AutonomyEnvFlags,
    pub autonomy_enabled: bool,
    pub workload_overrides: Vec<WorkloadAutonomyOverride>,
    pub recommendations: Vec<String>,
}

pub fn build_autonomy_status(state_path: &Path) -> anyhow::Result<AutonomyStatusReport> {
    let store = StateStore::load(state_path)?;
    let config = crate::config::Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);

    let env = AutonomyEnvFlags {
        aether_auto_restart: env_bool("AETHER_AUTO_RESTART", false),
        aether_auto_reconcile: env_bool("AETHER_AUTO_RECONCILE", false),
        reconciliation_auto_reconcile: config.reconciliation.auto_reconcile,
    };

    let mut overrides = Vec::new();
    for ws in store.list() {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        if let Some(autonomy) = &spec.autonomy {
            overrides.push(workload_override(&ws, autonomy));
        }
    }

    let autonomy_enabled = policy.auto_restart
        || policy.auto_reconcile_drift
        || policy.auto_migrate != AutonomyTier::Recommend
        || policy.auto_evolve != AutonomyTier::Recommend;

    let recommendations = build_recommendations(&policy, &overrides, store.list().len());

    Ok(AutonomyStatusReport {
        generated_at: crate::resources::now_rfc3339(),
        effective_policy: policy,
        env,
        autonomy_enabled,
        workload_overrides: overrides,
        recommendations,
    })
}

fn workload_override(ws: &WorkloadState, autonomy: &crate::spec::AutonomySpec) -> WorkloadAutonomyOverride {
    WorkloadAutonomyOverride {
        workload: ws.name.clone(),
        migration: level_label(&autonomy.migration),
        healing: level_label(&autonomy.healing),
        evolution: level_label(&autonomy.evolution),
    }
}

fn level_label(level: &AutonomyLevel) -> String {
    match level {
        AutonomyLevel::Recommend => "recommend".into(),
        AutonomyLevel::AutoLowRisk => "auto-low-risk".into(),
        AutonomyLevel::Auto => "auto".into(),
    }
}

fn build_recommendations(
    policy: &AutonomyPolicy,
    overrides: &[WorkloadAutonomyOverride],
    workload_count: usize,
) -> Vec<String> {
    let mut recs = Vec::new();
    if !policy.auto_restart && !policy.auto_reconcile_drift {
        recs.push(
            "Autonomous mode is off — set AETHER_AUTO_RESTART=1 or workload autonomy.healing: auto."
                .into(),
        );
    }
    if policy.auto_restart && !policy.auto_reconcile_drift {
        recs.push("Restart healing enabled — enable AETHER_AUTO_RECONCILE for drift self-healing.".into());
    }
    if workload_count > 0 && overrides.is_empty() {
        recs.push("No per-workload autonomy overrides — add an autonomy: block to specs for fine control.".into());
    }
    if policy.auto_migrate == AutonomyTier::Recommend && policy.auto_evolve == AutonomyTier::Recommend {
        recs.push("Migration and evolution agents are recommend-only — set autonomy.migration/evolution for auto.".into());
    }
    if recs.is_empty() {
        recs.push("Autonomous agents are active within configured policy bounds.".into());
    }
    recs
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autonomy_status_empty_fleet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let report = build_autonomy_status(&path).unwrap();
        assert!(!report.autonomy_enabled || report.env.aether_auto_restart);
        assert!(!report.recommendations.is_empty());
    }
}
