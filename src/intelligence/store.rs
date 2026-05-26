// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Persistent intelligence store for outcomes, runtime history, and behavior profiles.

use crate::ai::affinity::{AffinityEngine, DeploymentOutcome, WorkloadClass};
use crate::ai::scoring::RuntimeHistory;
use crate::intelligence::profile::WorkloadBehaviorProfile;
use crate::runtime::RuntimeKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntelligenceStore {
    pub outcomes: Vec<DeploymentOutcome>,
    pub runtime_history: HashMap<String, RuntimeHistory>,
    pub behavior_profiles: HashMap<String, WorkloadBehaviorProfile>,
}

impl IntelligenceStore {
    const MAX_OUTCOMES: usize = 10_000;

    pub fn default_path() -> PathBuf {
        crate::resources::aether_path("intelligence.json")
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        crate::resources::json_load(path)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        crate::resources::json_save(self, path)
    }

    pub fn record_outcome(&mut self, outcome: DeploymentOutcome) {
        let runtime_key = format!("{}", outcome.runtime);
        let hist = self
            .runtime_history
            .entry(runtime_key)
            .or_default();
        hist.total_deployments += 1;
        if outcome.success {
            hist.successful_deployments += 1;
        }
        if let Some(uptime) = outcome.uptime_pct {
            let n = hist.total_deployments as f64;
            hist.avg_uptime_pct =
                ((hist.avg_uptime_pct * (n - 1.0)) + uptime) / n.max(1.0);
        }

        self.outcomes.push(outcome);
        if self.outcomes.len() > Self::MAX_OUTCOMES {
            let drain = self.outcomes.len() - Self::MAX_OUTCOMES;
            self.outcomes.drain(..drain);
        }
    }

    pub fn sync_affinity(&self) -> anyhow::Result<()> {
        let path = AffinityEngine::default_path();
        let mut engine = AffinityEngine::load(&path).unwrap_or_default();
        for outcome in &self.outcomes {
            engine.record(outcome.clone());
        }
        engine.save(&path)
    }

    pub fn runtime_history_map(&self) -> HashMap<RuntimeKind, RuntimeHistory> {
        let mut map = HashMap::new();
        for rt in [
            RuntimeKind::Podman,
            RuntimeKind::Docker,
            RuntimeKind::Kubernetes,
            RuntimeKind::KubeVirt,
            RuntimeKind::Metal3,
        ] {
            let key = format!("{rt}");
            if let Some(h) = self.runtime_history.get(&key) {
                map.insert(rt, h.clone());
            }
        }
        map
    }

    pub fn upsert_behavior_profile(&mut self, profile: WorkloadBehaviorProfile) {
        self.behavior_profiles
            .insert(profile.workload_name.clone(), profile);
    }
}

/// Map scoring workload class to affinity workload class.
pub fn affinity_class_from_spec(workload: &crate::spec::Workload) -> WorkloadClass {
    use crate::ai::scoring::ScoringEngine;
    let scoring_class = ScoringEngine::with_defaults().classify_workload(workload);
    match scoring_class {
        crate::ai::scoring::WorkloadClass::Stateless => WorkloadClass::WebService,
        crate::ai::scoring::WorkloadClass::Stateful => WorkloadClass::Database,
        crate::ai::scoring::WorkloadClass::GpuCompute => WorkloadClass::MlTraining,
        crate::ai::scoring::WorkloadClass::BareMetal => WorkloadClass::Worker,
        crate::ai::scoring::WorkloadClass::Batch => WorkloadClass::BatchJob,
        crate::ai::scoring::WorkloadClass::General => WorkloadClass::Microservice,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::affinity::DeploymentOutcome;

    #[test]
    fn test_record_outcome_updates_history() {
        let mut store = IntelligenceStore::default();
        store.record_outcome(DeploymentOutcome {
            workload_name: "app".into(),
            workload_class: WorkloadClass::WebService,
            runtime: RuntimeKind::Kubernetes,
            success: true,
            uptime_pct: Some(99.5),
            avg_latency_ms: None,
            error_rate_pct: None,
            restarts: 0,
            cost_per_day: None,
            timestamp: "2026-01-01T00:00:00Z".into(),
            failure_reason: None,
        });
        let hist = store.runtime_history.get("kubernetes").unwrap();
        assert_eq!(hist.total_deployments, 1);
        assert_eq!(hist.successful_deployments, 1);
    }
}
