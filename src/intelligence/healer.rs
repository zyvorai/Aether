// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
