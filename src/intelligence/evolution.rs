// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Autonomous runtime evolution — continuous workload runtime optimization.

use crate::ai::scoring::ScoringEngine;
use crate::config::Config;
use crate::intelligence::policy::{AutonomyPolicy, AutonomyTier};
use crate::intelligence::profile::BehaviorProfiler;
use crate::intelligence::store::IntelligenceStore;
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionStatus {
    pub generated_at: String,
    pub workloads: Vec<EvolutionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEntry {
    pub workload: String,
    pub current_runtime: String,
    pub recommended_runtime: String,
    pub improvement_pct: f64,
    pub confidence: f64,
    pub auto_eligible: bool,
    pub trajectory: Vec<String>,
    pub reasons: Vec<String>,
}

pub struct EvolutionEngine;

impl EvolutionEngine {
    pub fn status_for_fleet(
        workloads: &[(Workload, WorkloadState)],
        policy: &AutonomyPolicy,
    ) -> EvolutionStatus {
        let config = Config::load();
        let intel = IntelligenceStore::load(&IntelligenceStore::default_path()).unwrap_or_default();
        let engine = ScoringEngine::new(config.engine).with_history(intel.runtime_history_map());

        let mut entries = Vec::new();
        for (spec, ws) in workloads {
            entries.push(Self::status_for_workload(spec, ws, &engine, policy));
        }

        entries.sort_by(|a, b| {
            b.improvement_pct
                .partial_cmp(&a.improvement_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        EvolutionStatus {
            generated_at: crate::resources::now_rfc3339(),
            workloads: entries,
        }
    }

    pub fn status_for_workload(
        spec: &Workload,
        ws: &WorkloadState,
        engine: &ScoringEngine,
        policy: &AutonomyPolicy,
    ) -> EvolutionEntry {
        let result = engine.score(spec);
        let current = ws.runtime;
        let recommended = result.recommended;
        let current_score = result
            .scores
            .iter()
            .find(|s| s.runtime == current)
            .map(|s| s.total_score)
            .unwrap_or(0.0);
        let recommended_score = result
            .scores
            .iter()
            .find(|s| s.runtime == recommended)
            .map(|s| s.total_score)
            .unwrap_or(0.0);

        let improvement = if current_score > 0.0 {
            ((recommended_score - current_score) / current_score.max(0.01)) * 100.0
        } else {
            recommended_score * 100.0
        };

        let profile = BehaviorProfiler::profile_workload(spec, ws);
        let trajectory = build_trajectory(&profile.classification, current, recommended);

        let auto_eligible = policy.auto_evolve != AutonomyTier::Recommend
            && recommended != current
            && improvement >= 10.0
            && result.confidence >= 0.3;

        let mut reasons = result
            .scores
            .iter()
            .find(|s| s.runtime == recommended)
            .map(|s| s.reasons.clone())
            .unwrap_or_default();
        reasons.push(format!(
            "Behavior profile: {} (uptime {:.1}%)",
            profile.classification, profile.uptime_pct
        ));

        EvolutionEntry {
            workload: spec.metadata.name.clone(),
            current_runtime: format!("{current}"),
            recommended_runtime: format!("{recommended}"),
            improvement_pct: improvement.max(0.0),
            confidence: result.confidence,
            auto_eligible,
            trajectory,
            reasons,
        }
    }
}

fn build_trajectory(
    classification: &str,
    current: RuntimeKind,
    recommended: RuntimeKind,
) -> Vec<String> {
    let ideal = ideal_path(classification);
    let mut path = vec![format!("{current}")];
    if !ideal.is_empty() && ideal[0] != format!("{current}") {
        path.extend(ideal);
    } else if current != recommended {
        path.push(format!("{recommended}"));
    }
    path
}

fn ideal_path(classification: &str) -> Vec<String> {
    match classification {
        c if c.contains("gpu") => vec!["podman".into(), "kubernetes".into(), "kubevirt".into()],
        c if c.contains("stateless") => vec!["podman".into(), "kubernetes".into()],
        c if c.contains("stateful") => vec!["kubernetes".into(), "kubevirt".into()],
        _ => vec!["podman".into(), "kubernetes".into()],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionExecuteRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    pub max_actions: Option<usize>,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionExecuteReport {
    pub dry_run: bool,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

pub async fn execute_evolution(
    state_path: &std::path::Path,
    policy: &AutonomyPolicy,
    dry_run: bool,
    max_actions: usize,
) -> anyhow::Result<EvolutionExecuteReport> {
    use crate::spec::Workload;
    use crate::state::StateStore;

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

    let status = EvolutionEngine::status_for_fleet(&pairs, policy);
    let mut executed = Vec::new();
    let mut skipped = Vec::new();

    for entry in status.workloads.iter().take(max_actions) {
        if !entry.auto_eligible {
            skipped.push(format!(
                "{} → {}: not auto-eligible (policy or confidence)",
                entry.workload, entry.recommended_runtime
            ));
            continue;
        }
        if !policy.allows_auto_evolve() {
            skipped.push(format!(
                "{} → {}: evolution autonomy is recommend-only",
                entry.workload, entry.recommended_runtime
            ));
            continue;
        }
        let line = format!(
            "evolve {} from {} to {} (+{:.0}%)",
            entry.workload, entry.current_runtime, entry.recommended_runtime, entry.improvement_pct
        );
        if dry_run {
            executed.push(format!("dry-run: {line}"));
        } else {
            skipped.push(format!(
                "{line} — not applied: runtime evolution mutation not implemented"
            ));
        }
    }

    Ok(EvolutionExecuteReport {
        dry_run,
        executed,
        skipped,
    })
}
