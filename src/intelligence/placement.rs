// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Global multi-cluster placement intelligence.

use crate::ai::scoring::ScoringEngine;
use crate::config::Config;
use crate::intelligence::store::IntelligenceStore;
use crate::kubecluster::ClusterInfo;
use crate::ragnarok::scheduling;
use crate::ragnarok::tee::probe_host_tee;
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementRecommendation {
    pub cluster: Option<String>,
    pub runtime: String,
    pub score: f64,
    pub latency_score: f64,
    pub cost_score: f64,
    pub gpu_available: bool,
    pub reasons: Vec<String>,
}

pub struct GlobalPlacementEngine;

impl GlobalPlacementEngine {
    pub fn recommend(
        workload: &Workload,
        clusters: &[ClusterInfo],
    ) -> Vec<PlacementRecommendation> {
        let config = Config::load();
        let intel = IntelligenceStore::load(&IntelligenceStore::default_path()).unwrap_or_default();
        let engine = ScoringEngine::new(config.engine).with_history(intel.runtime_history_map());
        let scoring = engine.score(workload);
        let host_tee = probe_host_tee();
        let (conf_bonus, conf_reasons) = scheduling::placement_bonus(workload, &host_tee);
        let conf_runtime = if workload.confidential.as_ref().is_some_and(|c| c.enabled) {
            Some(scheduling::recommended_runtime(workload))
        } else {
            None
        };

        let mut out = Vec::new();
        let reachable_clusters: Vec<&ClusterInfo> = clusters
            .iter()
            .filter(|c| c.reachable)
            .collect();

        if reachable_clusters.is_empty() {
            for rs in &scoring.scores {
                out.push(PlacementRecommendation {
                    cluster: None,
                    runtime: format!("{}", rs.runtime),
                    score: rs.total_score,
                    latency_score: rs.performance_score,
                    cost_score: rs.cost_score,
                    gpu_available: rs.runtime == RuntimeKind::KubeVirt
                        || rs.runtime == RuntimeKind::Metal3,
                    reasons: rs.reasons.clone(),
                });
            }
            return out;
        }

        for cluster in reachable_clusters {
            for rs in &scoring.scores {
                let cluster_bonus = if cluster.reachable { 0.05 } else { 0.0 };
                let gpu_bonus = if workload.requirements.gpu.is_some()
                    && (rs.runtime == RuntimeKind::KubeVirt || rs.runtime == RuntimeKind::Metal3)
                {
                    0.1
                } else {
                    0.0
                };
                let mut score = (rs.total_score + cluster_bonus + gpu_bonus).min(1.0);
                let mut reasons = rs.reasons.clone();
                if conf_runtime == Some(rs.runtime) {
                    score = (score + conf_bonus).min(1.0);
                    reasons.extend(conf_reasons.clone());
                }
                reasons.push(format!("Cluster {} ({:?})", cluster.name, cluster.server));
                out.push(PlacementRecommendation {
                    cluster: Some(cluster.name.clone()),
                    runtime: format!("{}", rs.runtime),
                    score,
                    latency_score: rs.performance_score,
                    cost_score: rs.cost_score,
                    gpu_available: gpu_bonus > 0.0,
                    reasons,
                });
            }
        }

        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out.truncate(12);
        out
    }
}
