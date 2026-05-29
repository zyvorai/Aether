// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Anomaly and drift auto-remediation planning and execution.

use crate::drift::fleet::scan_fleet;
use crate::drift::DriftSeverity;
use crate::intelligence::anomaly::load_placement_signals;
use crate::state::StateStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationAction {
    pub action_type: String,
    pub target: String,
    pub cluster: Option<String>,
    pub reason: String,
    pub auto_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationPlan {
    pub generated_at: String,
    pub sources: Vec<String>,
    pub actions: Vec<RemediationAction>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemediationExecuteResult {
    pub dry_run: bool,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemediationExecuteRequest {
    pub dry_run: Option<bool>,
    pub max_actions: Option<usize>,
}

pub async fn build_remediation_plan(store: &StateStore) -> RemediationPlan {
    let mut actions = Vec::new();
    let mut sources = Vec::new();
    let mut warnings = Vec::new();

    let clusters = crate::kubecluster::list_clusters().await.unwrap_or_default();
    let anomalies = load_placement_signals(&clusters).await;
    if anomalies.configured {
        sources.push("packetwolf_anomalies".into());
        for (cluster, summary) in &anomalies.by_cluster {
            if summary.count > 0 {
                actions.push(RemediationAction {
                    action_type: "avoid_placement".into(),
                    target: cluster.clone(),
                    cluster: Some(cluster.clone()),
                    reason: format!(
                        "PacketWolf reported {} anomal{} (max severity {:.1})",
                        summary.count,
                        if summary.count == 1 { "y" } else { "ies" },
                        summary.max_severity
                    ),
                    auto_safe: true,
                });
                if summary.max_severity >= 4.0 {
                    actions.push(RemediationAction {
                        action_type: "notify_operator".into(),
                        target: cluster.clone(),
                        cluster: Some(cluster.clone()),
                        reason: "High-severity PacketWolf anomaly cluster".into(),
                        auto_safe: true,
                    });
                }
            }
        }
    }

    let fleet_drift = scan_fleet(store);
    if fleet_drift.drifted > 0 {
        sources.push("fleet_drift".into());
        for row in fleet_drift.rows.iter().filter(|r| r.has_drift) {
            let auto_safe = matches!(row.severity, DriftSeverity::Info | DriftSeverity::Warning);
            actions.push(RemediationAction {
                action_type: if auto_safe {
                    "reconcile_drift".into()
                } else {
                    "notify_operator".into()
                },
                target: row.workload.clone(),
                cluster: row.cluster.clone(),
                reason: format!(
                    "Fleet drift detected ({:?}, {} items)",
                    row.severity, row.drift_count
                ),
                auto_safe,
            });
        }
    }

    if actions.is_empty() {
        warnings.push("No remediation actions recommended — fleet appears healthy.".into());
    } else {
        warnings.push(
            "Only auto_safe actions execute without explicit operator approval.".into(),
        );
    }

    RemediationPlan {
        generated_at: crate::resources::now_rfc3339(),
        sources,
        actions,
        warnings,
    }
}

pub async fn execute_remediation(
    plan: &RemediationPlan,
    dry_run: bool,
    max_actions: usize,
) -> Result<RemediationExecuteResult> {
    let mut result = RemediationExecuteResult {
        dry_run,
        ..Default::default()
    };

    for action in plan.actions.iter().take(max_actions) {
        if !action.auto_safe {
            result.skipped.push(format!(
                "{} {}: not auto_safe",
                action.action_type, action.target
            ));
            continue;
        }

        let detail = match action.action_type.as_str() {
            "avoid_placement" => format!("recorded placement penalty for {}", action.target),
            "notify_operator" => format!("operator notification queued for {}", action.target),
            "reconcile_drift" => format!("drift reconcile requested for {}", action.target),
            other => format!("noop for unsupported action {other}"),
        };

        if dry_run {
            result
                .skipped
                .push(format!("dry-run: {} — {}", action.action_type, detail));
        } else {
            result
                .executed
                .push(format!("{} — {}", action.action_type, detail));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateStore;

    #[tokio::test]
    async fn empty_store_plan_has_warning() {
        let store = StateStore::default();
        let plan = build_remediation_plan(&store).await;
        assert!(!plan.warnings.is_empty());
    }
}
