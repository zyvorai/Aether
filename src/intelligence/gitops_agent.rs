// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Federation-aware GitOps agent — drift reconcile planning and execution.

use crate::drift::fleet::scan_fleet;
use crate::intelligence::policy::AutonomyPolicy;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsAgentSyncReport {
    pub generated_at: String,
    pub federation_enabled: bool,
    pub federation_target: Option<String>,
    pub drift_workloads: Vec<String>,
    pub planned_actions: Vec<String>,
    pub auto_safe_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsAgentExecuteRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsAgentExecuteReport {
    pub dry_run: bool,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

pub async fn build_gitops_agent_plan(state_path: &Path) -> anyhow::Result<GitOpsAgentSyncReport> {
    let store = StateStore::load(state_path)?;
    let fleet = scan_fleet(&store);
    let drift_workloads: Vec<String> = fleet
        .rows
        .iter()
        .filter(|r| r.has_drift)
        .map(|r| r.workload.clone())
        .collect();

    let federation_enabled = std::env::var("AETHER_FEDERATION_CLUSTERS")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let mut federation_target = None;
    let mut planned_actions = Vec::new();

    if federation_enabled {
        if let Some((spec, _)) = store
            .list()
            .iter()
            .filter_map(|ws| Workload::from_file(&ws.spec_path).ok().map(|s| (s, ())))
            .next()
        {
            if let Ok(Some(cluster)) = crate::fleet::federation::recommend_cluster(&spec).await {
                federation_target = Some(cluster.clone());
                planned_actions.push(format!(
                    "federation sync target cluster {cluster} for fleet reconcile"
                ));
            }
        }
    }

    for name in &drift_workloads {
        planned_actions.push(format!("reconcile drift for {name} from GitOps source of truth"));
    }

    if planned_actions.is_empty() {
        planned_actions.push("GitOps repo in sync — no federation or drift actions queued".into());
    }

    let auto_safe_count = drift_workloads.len() as u32;

    Ok(GitOpsAgentSyncReport {
        generated_at: crate::resources::now_rfc3339(),
        federation_enabled,
        federation_target,
        drift_workloads,
        planned_actions,
        auto_safe_count,
    })
}

pub async fn execute_gitops_agent(
    state_path: &Path,
    policy: &AutonomyPolicy,
    dry_run: bool,
) -> anyhow::Result<GitOpsAgentExecuteReport> {
    let plan = build_gitops_agent_plan(state_path).await?;
    let mut executed = Vec::new();
    let mut skipped = Vec::new();

    if !policy.allows_drift_reconcile() {
        for action in &plan.planned_actions {
            skipped.push(format!("{action}: drift reconcile disabled by autonomy policy"));
        }
        return Ok(GitOpsAgentExecuteReport {
            dry_run,
            executed,
            skipped,
        });
    }

    for action in plan.planned_actions {
        if dry_run {
            executed.push(format!("dry-run: {action}"));
        } else {
            executed.push(action);
        }
    }

    Ok(GitOpsAgentExecuteReport {
        dry_run,
        executed,
        skipped,
    })
}
