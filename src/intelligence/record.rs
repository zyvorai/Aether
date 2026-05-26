// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Record deployment and migration outcomes into the intelligence store.

use crate::ai::affinity::DeploymentOutcome;
use crate::health::HealthHistory;
use crate::intelligence::store::{affinity_class_from_spec, IntelligenceStore};
use crate::runtime::RuntimeKind;
use crate::spec::Workload;

pub fn record_deployment_outcome(
    workload: &Workload,
    runtime: RuntimeKind,
    success: bool,
    failure_reason: Option<String>,
) -> anyhow::Result<()> {
    let path = IntelligenceStore::default_path();
    let mut store = IntelligenceStore::load(&path).unwrap_or_default();
    let health = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
    let uptime = health.uptime_percent(&workload.metadata.name);
    let restarts = health.restart_count(&workload.metadata.name);

    store.record_outcome(DeploymentOutcome {
        workload_name: workload.metadata.name.clone(),
        workload_class: affinity_class_from_spec(workload),
        runtime,
        success,
        uptime_pct: if uptime > 0.0 { Some(uptime) } else { None },
        avg_latency_ms: None,
        error_rate_pct: None,
        restarts,
        cost_per_day: None,
        timestamp: crate::resources::now_rfc3339(),
        failure_reason,
    });
    store.save(&path)?;
    let _ = store.sync_affinity();
    Ok(())
}

pub fn record_migration_outcome(
    workload_name: &str,
    workload: &Workload,
    source: RuntimeKind,
    target: RuntimeKind,
    success: bool,
) -> anyhow::Result<()> {
    let path = IntelligenceStore::default_path();
    let mut store = IntelligenceStore::load(&path).unwrap_or_default();
    if let Some(hist) = store.runtime_history.get_mut(&format!("{source}")) {
        hist.total_migrations_from += 1;
    }
    if let Some(hist) = store.runtime_history.get_mut(&format!("{target}")) {
        hist.total_migrations_to += 1;
    }
    store.record_outcome(DeploymentOutcome {
        workload_name: workload_name.to_string(),
        workload_class: affinity_class_from_spec(workload),
        runtime: target,
        success,
        uptime_pct: None,
        avg_latency_ms: None,
        error_rate_pct: None,
        restarts: 0,
        cost_per_day: None,
        timestamp: crate::resources::now_rfc3339(),
        failure_reason: if success {
            None
        } else {
            Some(format!("migration from {source} to {target} failed"))
        },
    });
    store.save(&path)?;
    let _ = store.sync_affinity();
    Ok(())
}
