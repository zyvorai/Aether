// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Unified autonomous agent registry status.

use crate::intelligence::evolution::EvolutionEngine;
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::healer::build_healer_preview;
use crate::intelligence::policy::AutonomyPolicy;
use crate::intelligence::predict::FailurePredictor;
use crate::intelligence::remediation;
use crate::intelligence::security::SecurityEngine;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusEntry {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
    pub route: String,
    pub pending_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistryReport {
    pub generated_at: String,
    pub agents: Vec<AgentStatusEntry>,
}

pub async fn build_agent_registry(state_path: &Path) -> anyhow::Result<AgentRegistryReport> {
    let store = StateStore::load(state_path)?;
    let pairs: Vec<(Workload, WorkloadState)> = store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|s| (s, (*ws).clone()))
        })
        .collect();

    let config = crate::config::Config::load();
    let policy =
        AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let healer = build_healer_preview(&store, &policy).await;
    let cost = FinOpsEngine::optimize_fleet(&pairs);
    let threats = SecurityEngine::scan_fleet(&pairs);
    let predictions = FailurePredictor::predict_fleet(&pairs);
    let evolution = EvolutionEngine::status_for_fleet(&pairs, &policy);
    let remediation = remediation::build_remediation_plan(&store).await;

    let at_risk = predictions
        .predictions
        .iter()
        .filter(|p| p.risk_level == "high" || p.risk_level == "critical")
        .count() as u32;
    let migrate_candidates = evolution
        .workloads
        .iter()
        .filter(|e| e.recommended_runtime != e.current_runtime)
        .count() as u32;

    let agents = vec![
        entry(
            "sre",
            "Zyra SRE",
            healer.would_execute.len() as u32,
            "observability",
        ),
        entry("architect", "Zyra Architect", migrate_candidates, "ai"),
        entry(
            "devops",
            "Zyra DevOps",
            remediation.actions.len() as u32,
            "gitops",
        ),
        entry("kubernetes", "Zyra Kubernetes", at_risk, "clusters"),
        entry(
            "security",
            "Zyra Security",
            threats.threats.len() as u32,
            "security",
        ),
        entry(
            "cost",
            "Zyra Cost Optimizer",
            cost.recommendations.len() as u32,
            "cost",
        ),
        entry(
            "observability",
            "Zyra Observability",
            at_risk,
            "observability",
        ),
        entry("ai_engineer", "Zyra AI Engineer", 0, "confidential"),
        entry("database", "Zyra Database Expert", 0, "workloads"),
    ];

    Ok(AgentRegistryReport {
        generated_at: crate::resources::now_rfc3339(),
        agents,
    })
}

fn entry(id: &str, label: &str, pending: u32, route: &str) -> AgentStatusEntry {
    let status = if pending > 0 {
        if id == "security" || id == "capacity" {
            "alert"
        } else {
            "active"
        }
    } else {
        "idle"
    };
    AgentStatusEntry {
        id: id.into(),
        label: label.into(),
        status: status.into(),
        detail: format!("{pending} pending"),
        route: route.into(),
        pending_count: pending,
    }
}
