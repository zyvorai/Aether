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
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Default)]
pub struct HealerResult {
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
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
            OrchestratorAction::Restart { workload, reason, .. } => {
                if !policy.allows_restart() {
                    result
                        .skipped
                        .push(format!("restart {workload}: autonomy policy disabled"));
                    continue;
                }
                match restart_workload_internal(state, state_path, workload).await {
                    Ok(()) => {
                        audit_autonomous(source, workload, reason);
                        result.executed.push(format!("restarted {workload}: {reason}"));
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
                result
                    .executed
                    .push(format!("alert {workload}: {message}"));
            }
            other => {
                result
                    .skipped
                    .push(format!("{other} (no auto executor)"));
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
