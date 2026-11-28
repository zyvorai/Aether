// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Command Center briefing — aggregates intelligence into a narrative snapshot.

use crate::config::Config;
use crate::intelligence::evolution::EvolutionEngine;
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::policy::AutonomyPolicy;
use crate::intelligence::predict::FailurePredictor;
use crate::ops::observed::{Condition, Snapshot};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCenterBriefing {
    pub generated_at: String,
    pub greeting: String,
    /// `None` (JSON `null`) when no workload has a current health check.
    pub fleet_health_pct: Option<f64>,
    #[serde(default)]
    pub fleet: FleetHealth,
    pub issues: Vec<BriefingIssue>,
    pub potential_savings_usd: f64,
    pub migration_opportunities: u32,
    pub capacity_risks: Vec<CapacityRisk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingIssue {
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub workload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityRisk {
    pub resource: String,
    pub days_remaining: u32,
    pub summary: String,
}

fn workload_pairs(store: &StateStore) -> Vec<(Workload, WorkloadState)> {
    store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|s| (s, (*ws).clone()))
        })
        .collect()
}

fn greeting_for_hour(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good morning.",
        12..=16 => "Good afternoon.",
        17..=21 => "Good evening.",
        _ => "Good night.",
    }
}

/// What the recorded observations say about the managed workloads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetHealth {
    pub total: usize,
    pub running: usize,
    /// Checked recently and found in trouble.
    pub unhealthy: usize,
    /// Stopped on request; not part of the percentage.
    pub stopped: usize,
    /// No current check; not part of the percentage.
    pub unknown: usize,
}

impl FleetHealth {
    /// Share of the checked workloads that are running, or `None` when none
    /// has a current check.
    pub fn pct(&self) -> Option<f64> {
        let checked = self.running + self.unhealthy;
        (checked > 0).then(|| self.running as f64 * 100.0 / checked as f64)
    }
}

fn fleet_health(
    observed: &Snapshot,
    store: &StateStore,
    now: chrono::DateTime<chrono::Utc>,
) -> FleetHealth {
    let mut fleet = FleetHealth::default();
    for ws in store.list() {
        fleet.total += 1;
        match observed.status(ws, now).condition {
            Condition::Running => fleet.running += 1,
            Condition::Stopped => fleet.stopped += 1,
            Condition::Unknown => fleet.unknown += 1,
            Condition::Unhealthy
            | Condition::Pending
            | Condition::Failed
            | Condition::Down
            | Condition::NotReady => fleet.unhealthy += 1,
        }
    }
    fleet
}

/// One issue per workload that was observed in trouble, worst first.
fn current_issues(
    observed: &Snapshot,
    store: &StateStore,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<BriefingIssue> {
    let mut issues: Vec<_> = store
        .list()
        .iter()
        .filter_map(|ws| {
            let status = observed.status(ws, now);
            let severity = match status.condition {
                Condition::Failed | Condition::Down => "critical",
                Condition::Unhealthy | Condition::Pending | Condition::NotReady => "medium",
                Condition::Running | Condition::Stopped | Condition::Unknown => return None,
            };
            Some(BriefingIssue {
                severity: severity.into(),
                title: format!("{} is {}", ws.name, status.condition.label()),
                detail: status.detail,
                workload: Some(ws.name.clone()),
            })
        })
        .collect();
    issues.sort_by(|a, b| {
        (a.severity != "critical", &a.workload).cmp(&(b.severity != "critical", &b.workload))
    });
    issues
}

pub fn build_command_center_briefing(state_path: &Path) -> anyhow::Result<CommandCenterBriefing> {
    let store = StateStore::load(state_path)?;
    let pairs = workload_pairs(&store);

    let observed = Snapshot::load(&crate::ops::data_dir());
    let now = chrono::Utc::now();
    let fleet = fleet_health(&observed, &store, now);
    let predictions = FailurePredictor::predict_fleet(&pairs);
    let cost = FinOpsEngine::optimize_fleet(&pairs);
    let config = Config::load();
    let policy =
        AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let evolution = EvolutionEngine::status_for_fleet(&pairs, &policy);

    // What is wrong now comes before what might go wrong.
    let mut issues = current_issues(&observed, &store, now);
    for p in &predictions.predictions {
        if p.risk_level == "critical" || p.risk_level == "high" {
            let detail = p
                .predictions
                .first()
                .map(|s| s.reason.clone())
                .unwrap_or_else(|| format!("Risk score {:.0}%", p.risk_score * 100.0));
            issues.push(BriefingIssue {
                severity: p.risk_level.clone(),
                title: format!("{} — elevated failure risk", p.workload),
                detail,
                workload: Some(p.workload.clone()),
            });
        }
    }

    for rec in cost.recommendations.iter().take(3) {
        issues.push(BriefingIssue {
            severity: "info".into(),
            title: format!("Cost optimization — {}", rec.workload),
            detail: rec.reason.clone(),
            workload: Some(rec.workload.clone()),
        });
    }

    issues.truncate(8);

    let potential_savings_usd: f64 = cost
        .recommendations
        .iter()
        .map(|r| r.savings_monthly_usd)
        .sum();

    let migration_opportunities = evolution
        .workloads
        .iter()
        .filter(|e| e.recommended_runtime != e.current_runtime && e.improvement_pct > 5.0)
        .count() as u32;

    let mut capacity_risks = Vec::new();
    for p in predictions.predictions.iter().take(5) {
        for signal in &p.predictions {
            if signal.kind.contains("gpu") || signal.reason.to_lowercase().contains("gpu") {
                capacity_risks.push(CapacityRisk {
                    resource: "GPU".into(),
                    days_remaining: horizon_to_days(&signal.horizon),
                    summary: format!("{} — {}", p.workload, signal.reason),
                });
            }
        }
        if signal_is_capacity(&p.predictions) {
            capacity_risks.push(CapacityRisk {
                resource: "Workload capacity".into(),
                days_remaining: 12,
                summary: format!(
                    "{} predicted saturation risk ({})",
                    p.workload, p.risk_level
                ),
            });
        }
    }
    capacity_risks.truncate(4);

    let hour = chrono::Local::now().hour();
    Ok(CommandCenterBriefing {
        generated_at: crate::resources::now_rfc3339(),
        greeting: greeting_for_hour(hour).into(),
        fleet_health_pct: fleet.pct(),
        fleet,
        issues,
        potential_savings_usd,
        migration_opportunities,
        capacity_risks,
    })
}

fn horizon_to_days(horizon: &str) -> u32 {
    match horizon {
        "immediate" => 1,
        "4h" => 1,
        "24h" => 3,
        "ongoing" => 7,
        _ => 14,
    }
}

fn signal_is_capacity(signals: &[crate::intelligence::predict::FailureSignal]) -> bool {
    signals
        .iter()
        .any(|s| s.kind == "oom" || s.kind == "workload_failure" || s.probability >= 0.7)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Instance, RuntimeKind};

    #[test]
    fn briefing_empty_fleet_has_greeting() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        StateStore::new().save(&state_path).unwrap();

        let briefing = build_command_center_briefing(&state_path).unwrap();
        assert!(!briefing.greeting.is_empty());
        assert_eq!(briefing.migration_opportunities, 0);
    }

    #[test]
    fn briefing_reports_no_health_for_a_workload_nobody_checked() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let spec_path = dir.path().join("web.yaml");
        std::fs::write(
            &spec_path,
            include_str!("../../examples/labs/kubernetes/workload.yaml").replace("nginx", "web"),
        )
        .unwrap();

        let mut store = StateStore::new();
        store.upsert(
            "web".into(),
            WorkloadState {
                name: "web".into(),
                runtime: RuntimeKind::Kubernetes,
                instance: Instance {
                    id: "web-1".into(),
                    name: "web".into(),
                    runtime: RuntimeKind::Kubernetes,
                    image: "web:latest".into(),
                    created_at: "2024-01-01T00:00:00Z".into(),
                },
                spec_path: spec_path.clone(),
                created_at: "2024-01-01T00:00:00Z".into(),
                updated_at: "2024-01-01T00:00:00Z".into(),
                os_version: None,
                node_labels: vec![],
                atlas_volume_ids: Vec::new(),
                namespace: None,
                cluster_context: None,
                k8s_kind: None,
            },
        );
        store.save(&state_path).unwrap();

        let briefing = build_command_center_briefing(&state_path).unwrap();
        // The data directory is the real one here, which never saw this fleet.
        assert_ne!(briefing.fleet_health_pct, Some(100.0));
    }

    #[test]
    fn fleet_health_counts_only_what_was_observed() {
        use crate::ops::incidents::{self, tests::obs};
        use crate::runtime::InstanceState;
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = chrono::Utc::now();
        let mut store = StateStore::new();
        for name in ["api", "web", "worker"] {
            let instance =
                Instance::new(name.into(), name.into(), RuntimeKind::Podman, name.into());
            let mut ws = WorkloadState::new(
                name.into(),
                RuntimeKind::Podman,
                instance,
                "web.yaml".into(),
            );
            ws.updated_at = (now - chrono::Duration::hours(1)).to_rfc3339();
            store.upsert(name.into(), ws);
        }
        let nothing = fleet_health(&Snapshot::load(d), &store, now);
        assert_eq!((nothing.unknown, nothing.pct()), (3, None));

        incidents::record(d, &obs("api", true, &now.to_rfc3339())).unwrap();
        let mut down = obs("web", false, &now.to_rfc3339());
        down.state = InstanceState::Stopped;
        incidents::record(d, &down).unwrap();
        let snapshot = Snapshot::load(d);
        let fleet = fleet_health(&snapshot, &store, now);
        assert_eq!((fleet.running, fleet.unhealthy, fleet.unknown), (1, 1, 1));
        assert_eq!(fleet.pct(), Some(50.0));
        let issues = current_issues(&snapshot, &store, now);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].title, "web is stopped unexpectedly");
        assert_eq!(issues[0].severity, "critical");
    }
}
