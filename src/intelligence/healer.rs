// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Self-healing action executor for orchestrator actions.

use crate::audit::{ActionResult, AuditAction, AuditLog};
use crate::drift::DriftDetector;
use crate::intelligence::policy::AutonomyPolicy;
use crate::orchestrator::OrchestratorAction;
use crate::runtime::create_runtime;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Default)]
pub struct HealerResult {
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealerPreviewReport {
    pub generated_at: String,
    pub policy: AutonomyPolicy,
    pub orchestrator_actions: Vec<String>,
    pub drift_candidates: Vec<String>,
    pub would_execute: Vec<String>,
    pub would_skip: Vec<String>,
}

pub fn preview_orchestrator_actions(
    actions: &[OrchestratorAction],
    policy: &AutonomyPolicy,
    drift_candidates: &[String],
) -> HealerPreviewReport {
    let mut would_execute = Vec::new();
    let mut would_skip = Vec::new();
    let orchestrator_actions: Vec<String> = actions.iter().map(|a| a.to_string()).collect();

    for action in actions {
        match action {
            OrchestratorAction::Restart {
                workload, reason, ..
            } => {
                if policy.allows_restart() {
                    would_execute.push(format!("restart {workload}: {reason}"));
                } else {
                    would_skip.push(format!("restart {workload}: autonomy policy disabled"));
                }
            }
            OrchestratorAction::Alert { workload, message } => {
                would_execute.push(format!("alert {workload}: {message}"));
            }
            other => {
                would_skip.push(format!("{other} (no auto executor)"));
            }
        }
    }

    for name in drift_candidates {
        if policy.allows_drift_reconcile() {
            would_execute.push(format!("reconcile drift on {name}"));
        } else {
            would_skip.push(format!("drift reconcile {name}: autonomy policy disabled"));
        }
    }

    HealerPreviewReport {
        generated_at: crate::resources::now_rfc3339(),
        policy: policy.clone(),
        orchestrator_actions,
        drift_candidates: drift_candidates.to_vec(),
        would_execute,
        would_skip,
    }
}

pub async fn build_healer_preview(
    state: &StateStore,
    policy: &AutonomyPolicy,
) -> HealerPreviewReport {
    use crate::orchestrator::{HealthStatus, Orchestrator, OrchestratorAction};

    let mut actions = Vec::new();
    if let Ok(orch) = Orchestrator::load(&Orchestrator::default_path()) {
        for summary in orch.list_workloads() {
            if matches!(
                summary.health,
                HealthStatus::Unhealthy | HealthStatus::Degraded
            ) {
                actions.push(OrchestratorAction::Restart {
                    workload: summary.name.clone(),
                    runtime: summary.runtime,
                    reason: format!("health status {}", summary.health),
                });
            }
        }
    }

    let detector = DriftDetector::new();
    let mut drift_candidates = Vec::new();
    for ws in state.list() {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        let report = detector.detect(&spec, ws);
        if report.has_drift {
            drift_candidates.push(ws.name.clone());
        }
    }

    preview_orchestrator_actions(&actions, policy, &drift_candidates)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealerExecuteRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealerExecuteReport {
    pub dry_run: bool,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

pub async fn execute_healer(
    state: &StateStore,
    state_path: &Path,
    policy: &AutonomyPolicy,
    dry_run: bool,
) -> HealerExecuteReport {
    let preview = build_healer_preview(state, policy).await;
    if dry_run {
        return HealerExecuteReport {
            dry_run: true,
            executed: preview.would_execute,
            skipped: preview.would_skip,
        };
    }

    use crate::orchestrator::{HealthStatus, Orchestrator, OrchestratorAction};
    let mut actions = Vec::new();
    if let Ok(orch) = Orchestrator::load(&Orchestrator::default_path()) {
        for summary in orch.list_workloads() {
            if matches!(
                summary.health,
                HealthStatus::Unhealthy | HealthStatus::Degraded
            ) {
                actions.push(OrchestratorAction::Restart {
                    workload: summary.name.clone(),
                    runtime: summary.runtime,
                    reason: format!("health status {}", summary.health),
                });
            }
        }
    }

    let state_arc = Arc::new(RwLock::new(state.clone()));
    let result = execute_orchestrator_actions(
        &actions,
        policy,
        &state_arc,
        state_path,
        "api-healer-execute",
    )
    .await;

    HealerExecuteReport {
        dry_run: false,
        executed: result.executed,
        skipped: result.skipped,
    }
}

pub async fn execute_orchestrator_actions(
    actions: &[OrchestratorAction],
    policy: &AutonomyPolicy,
    state: &Arc<RwLock<StateStore>>,
    state_path: &Path,
    source: &str,
) -> HealerResult {
    let mut result = HealerResult::default();
    for action in actions {
        match action {
            OrchestratorAction::Restart {
                workload, reason, ..
            } => {
                if !policy.allows_restart() {
                    result
                        .skipped
                        .push(format!("restart {workload}: autonomy policy disabled"));
                    continue;
                }
                match restart_workload_internal(state, state_path, workload).await {
                    Ok(()) => {
                        audit_autonomous(source, workload, reason);
                        result
                            .executed
                            .push(format!("restarted {workload}: {reason}"));
                        crate::metrics::record_orchestrator_restart(workload, "auto");
                    }
                    Err(e) => {
                        result
                            .skipped
                            .push(format!("restart {workload} failed: {e}"));
                    }
                }
            }
            OrchestratorAction::Alert { workload, message } => {
                result.executed.push(format!("alert {workload}: {message}"));
            }
            OrchestratorAction::CircuitOpened { workload, reason } => {
                if !policy.allows_rollback() {
                    result.skipped.push(format!(
                        "circuit-opened {workload}: {reason} (auto-rollback disabled)"
                    ));
                    continue;
                }
                // Guard against rollback thrash to a bad snapshot: at most one
                // rollback per workload per cooldown window.
                let now = chrono::Utc::now().timestamp();
                if !rollback_cooldown_elapsed(workload, now) {
                    result
                        .skipped
                        .push(format!("rollback {workload}: within cooldown"));
                    continue;
                }
                match rollback_workload_internal(state, state_path, workload).await {
                    Ok(snapshot) => {
                        mark_rollback(workload, now);
                        let detail = format!(
                            "auto-rollback to {} (circuit opened: {reason})",
                            snapshot.display()
                        );
                        audit_autonomous(source, workload, &detail);
                        result
                            .executed
                            .push(format!("rolled back {workload}: {detail}"));
                    }
                    Err(e) => {
                        result
                            .skipped
                            .push(format!("rollback {workload} failed: {e}"));
                    }
                }
            }
            other => {
                result.skipped.push(format!("{other} (no auto executor)"));
            }
        }
    }

    if policy.allows_drift_reconcile() {
        reconcile_drift_fleet(state, state_path, source, &mut result).await;
    }

    result
}

async fn restart_workload_internal(
    state: &Arc<RwLock<StateStore>>,
    state_path: &Path,
    name: &str,
) -> anyhow::Result<()> {
    let (ws, rt) = {
        let store = state.read().await;
        let ws = store
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("workload not found: {name}"))?;
        let rt = create_runtime(&ws.runtime).await?;
        (ws, rt)
    };
    rt.stop(&ws.instance).await?;
    let spec = Workload::from_file(&ws.spec_path)?;
    let image = rt.build(&spec).await?;
    let instance = rt.run(&image, &spec).await?;
    let mut store = state.write().await;
    store.upsert(name.to_string(), ws.migrated(ws.runtime, instance));
    store.save(state_path)?;
    Ok(())
}

/// Per-workload timestamp (epoch seconds) of the last autonomous rollback, used
/// to bound rollback frequency so a persistently-failing workload isn't rolled
/// back to the same snapshot every time its circuit reopens.
fn rollback_ledger() -> &'static std::sync::Mutex<std::collections::HashMap<String, i64>> {
    static LEDGER: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, i64>>> =
        std::sync::OnceLock::new();
    LEDGER.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn rollback_cooldown_secs() -> i64 {
    std::env::var("AETHER_ROLLBACK_COOLDOWN_SECS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(600)
}

fn rollback_cooldown_elapsed(workload: &str, now: i64) -> bool {
    let cooldown = rollback_cooldown_secs();
    let ledger = rollback_ledger().lock().unwrap();
    match ledger.get(workload) {
        Some(&t) => now.saturating_sub(t) >= cooldown,
        None => true,
    }
}

fn mark_rollback(workload: &str, now: i64) {
    rollback_ledger()
        .lock()
        .unwrap()
        .insert(workload.to_string(), now);
}

/// Roll a workload back to its most recent snapshot: stop the current instance
/// (best-effort), then rebuild and redeploy from the snapshot's spec and restore
/// its state. Returns the snapshot path used. Errors when no snapshot exists.
async fn rollback_workload_internal(
    state: &Arc<RwLock<StateStore>>,
    state_path: &Path,
    name: &str,
) -> anyhow::Result<std::path::PathBuf> {
    use crate::backup::{Backup, SnapshotManager};

    let snap_mgr = SnapshotManager::new();
    let snapshot_path = snap_mgr
        .latest_snapshot(name)?
        .ok_or_else(|| anyhow::anyhow!("no snapshot available for '{name}'"))?;
    let backup = Backup::load(&snapshot_path)?;
    let snap_ws = backup
        .workloads
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("snapshot for '{name}' is empty"))?;

    // Stop the current (failing) instance, best-effort.
    let current = {
        let store = state.read().await;
        store.get(name).cloned()
    };
    if let Some(current) = current {
        if let Ok(rt) = create_runtime(&current.runtime).await {
            if let Err(e) = rt.stop(&current.instance).await {
                tracing::warn!("rollback {name}: stop of current instance failed: {e}");
            }
        }
    }

    // Redeploy from the snapshot's spec.
    let rt = create_runtime(&snap_ws.runtime).await?;
    let spec = Workload::from_file(&snap_ws.spec_path)?;
    let image = rt.build(&spec).await?;
    let instance = rt.run(&image, &spec).await?;

    let mut store = state.write().await;
    store.upsert(
        name.to_string(),
        WorkloadState {
            name: name.to_string(),
            runtime: snap_ws.runtime,
            instance,
            spec_path: snap_ws.spec_path.clone(),
            created_at: snap_ws.created_at.clone(),
            updated_at: crate::resources::now_rfc3339(),
            os_version: snap_ws.os_version.clone(),
            node_labels: snap_ws.node_labels.clone(),
            atlas_volume_ids: snap_ws.atlas_volume_ids.clone(),
            namespace: snap_ws.namespace.clone(),
            cluster_context: snap_ws.cluster_context.clone(),
            k8s_kind: snap_ws.k8s_kind.clone(),
        },
    );
    store.save(state_path)?;
    Ok(snapshot_path)
}

async fn reconcile_drift_fleet(
    state: &Arc<RwLock<StateStore>>,
    state_path: &Path,
    source: &str,
    result: &mut HealerResult,
) {
    let workloads: Vec<WorkloadState> = {
        let store = state.read().await;
        store.list().into_iter().cloned().collect()
    };
    let detector = DriftDetector::new();
    for ws in workloads {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        let report = detector.detect(&spec, &ws);
        if !report.has_drift {
            continue;
        }
        let mut store = state.write().await;
        match crate::drift::execute_reconciliation(&report, &mut store).await {
            Ok(results) => {
                let msg = results
                    .iter()
                    .map(|r| r.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                audit_autonomous(source, &ws.name, &msg);
                result
                    .executed
                    .push(format!("reconciled drift on {}: {}", ws.name, msg));
                let _ = store.save(state_path);
            }
            Err(e) => {
                result
                    .skipped
                    .push(format!("drift reconcile {} failed: {e}", ws.name));
            }
        }
    }
}

fn audit_autonomous(source: &str, workload: &str, detail: &str) {
    let path = AuditLog::default_path();
    if let Ok(mut log) = AuditLog::load(&path) {
        log.record(
            AuditAction::External,
            workload,
            Some(source),
            ActionResult::Success,
            "autonomous_action",
            Some(detail),
        );
        let _ = log.save(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_cooldown_gates_repeat_rollbacks() {
        // Unique workload name so the process-global ledger doesn't collide
        // with other tests running in parallel.
        let w = "healer-cooldown-test-workload-xyz";
        let now = 1_000_000i64;
        // Never rolled back → allowed.
        assert!(rollback_cooldown_elapsed(w, now));
        mark_rollback(w, now);
        // Within the default 600s window → blocked.
        assert!(!rollback_cooldown_elapsed(w, now + 100));
        // After the window → allowed again.
        assert!(rollback_cooldown_elapsed(w, now + 601));
    }
}
