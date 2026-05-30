// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Command Center next actions — prioritized operational queue.

use crate::intelligence::briefing::build_command_center_briefing;
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::healer::build_healer_preview;
use crate::intelligence::pipeline::build_autonomous_placement;
use crate::intelligence::policy::AutonomyPolicy;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub priority: u8,
    pub route: String,
    pub action_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextActionsReport {
    pub generated_at: String,
    pub actions: Vec<NextAction>,
}

pub async fn build_next_actions(state_path: &Path) -> anyhow::Result<NextActionsReport> {
    let store = StateStore::load(state_path)?;
    let briefing = build_command_center_briefing(state_path)?;
    let config = crate::config::Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let healer = build_healer_preview(&store, &policy).await;
    let pairs: Vec<(Workload, WorkloadState)> = store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|s| (s, (*ws).clone()))
        })
        .collect();
    let cost = FinOpsEngine::optimize_fleet(&pairs);
    let evolution = build_autonomous_placement(state_path)?;

    let mut actions = Vec::new();

    for issue in briefing.issues.iter().take(3) {
        if issue.severity == "info" {
            continue;
        }
        let route = if issue.workload.is_some() {
            "observability"
        } else {
            "observability"
        };
        actions.push(NextAction {
            id: format!("issue-{}", actions.len()),
            title: issue.title.clone(),
            detail: issue.detail.clone(),
            priority: severity_priority(&issue.severity),
            route: route.into(),
            action_type: "investigate".into(),
        });
    }

    for line in healer.would_execute.iter().take(3) {
        actions.push(NextAction {
            id: format!("heal-{}", actions.len()),
            title: "Self-healing ready".into(),
            detail: line.clone(),
            priority: 85,
            route: "observability".into(),
            action_type: "heal".into(),
        });
    }

    if briefing.potential_savings_usd > 0.0 {
        let top = cost.recommendations.first();
        actions.push(NextAction {
            id: "cost-save".into(),
            title: "Capture FinOps savings".into(),
            detail: top
                .map(|r| format!("{} — ${:.0}/mo", r.workload, r.savings_monthly_usd))
                .unwrap_or_else(|| format!("${:.0}/mo fleet potential", briefing.potential_savings_usd)),
            priority: 70,
            route: "cost".into(),
            action_type: "optimize".into(),
        });
    }

    if briefing.migration_opportunities > 0 {
        actions.push(NextAction {
            id: "migrate".into(),
            title: "Review migration opportunities".into(),
            detail: format!("{} workload(s) with runtime evolution candidates", briefing.migration_opportunities),
            priority: 65,
            route: "migrations".into(),
            action_type: "migrate".into(),
        });
    }

    for entry in evolution
        .workloads
        .iter()
        .filter(|e| e.auto_eligible && e.recommended_runtime != e.current_runtime)
        .take(2)
    {
        actions.push(NextAction {
            id: format!("place-{}", entry.workload),
            title: format!("Auto-place {}", entry.workload),
            detail: format!(
                "{} → {} (+{:.0}%)",
                entry.current_runtime, entry.recommended_runtime, entry.improvement_pct
            ),
            priority: 60,
            route: "migrations".into(),
            action_type: "place".into(),
        });
    }

    for risk in briefing.capacity_risks.iter().take(2) {
        actions.push(NextAction {
            id: format!("cap-{}", risk.resource),
            title: format!("Capacity — {}", risk.resource),
            detail: risk.summary.clone(),
            priority: 75,
            route: "observability".into(),
            action_type: "capacity".into(),
        });
    }

    if actions.is_empty() {
        actions.push(NextAction {
            id: "healthy".into(),
            title: "Fleet operating normally".into(),
            detail: "Run intent pipeline or digital twin to plan the next change.".into(),
            priority: 10,
            route: "ai".into(),
            action_type: "explore".into(),
        });
    }

    actions.sort_by(|a, b| b.priority.cmp(&a.priority));
    actions.truncate(8);

    Ok(NextActionsReport {
        generated_at: crate::resources::now_rfc3339(),
        actions,
    })
}

fn severity_priority(severity: &str) -> u8 {
    match severity.to_lowercase().as_str() {
        "critical" => 95,
        "high" => 90,
        "medium" => 75,
        "low" => 50,
        _ => 40,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn next_actions_empty_fleet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let report = build_next_actions(&path).await.unwrap();
        assert!(!report.actions.is_empty());
    }
}
