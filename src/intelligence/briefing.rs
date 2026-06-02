// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Command Center briefing — aggregates intelligence into a narrative snapshot.

use crate::config::Config;
use crate::health::HealthHistory;
use crate::intelligence::evolution::EvolutionEngine;
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::policy::AutonomyPolicy;
use crate::intelligence::predict::FailurePredictor;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCenterBriefing {
    pub generated_at: String,
    pub greeting: String,
    pub fleet_health_pct: f64,
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

pub fn build_command_center_briefing(state_path: &Path) -> anyhow::Result<CommandCenterBriefing> {
    let store = StateStore::load(state_path)?;
    let pairs = workload_pairs(&store);

    let health = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
    let ready = pairs
        .iter()
        .filter(|(_, ws)| {
            health.uptime_percent(&ws.name) >= 99.0 || health.restart_count(&ws.name) == 0
        })
        .count();
    let total = pairs.len().max(1);
    let fleet_health_pct = ((ready as f64 / total as f64) * 100.0).min(100.0);

    let predictions = FailurePredictor::predict_fleet(&pairs);
    let cost = FinOpsEngine::optimize_fleet(&pairs);
    let config = Config::load();
    let policy =
        AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let evolution = EvolutionEngine::status_for_fleet(&pairs, &policy);

    let mut issues = Vec::new();
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
        fleet_health_pct,
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
    use std::path::PathBuf;

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
    fn briefing_computes_health_from_workloads_without_restarts() {
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
            },
        );
        store.save(&state_path).unwrap();

        let briefing = build_command_center_briefing(&state_path).unwrap();
        assert!(
            briefing.fleet_health_pct >= 99.0,
            "expected healthy fleet, got {}",
            briefing.fleet_health_pct
        );
    }
}
