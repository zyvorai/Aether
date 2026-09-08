// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Federation placement — rank clusters with intelligence + anomaly signals.

use crate::engine::Engine;
use crate::intelligence::anomaly::{load_placement_signals, AnomalyPlacementSignals};
use crate::intelligence::placement::GlobalPlacementEngine;
use crate::kubecluster::{list_clusters, ClusterInfo};
use crate::spec::Workload;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPolicy {
    pub clusters: Vec<String>,
    pub weights: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPlanRequest {
    pub workload_yaml: Option<String>,
    pub workload_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterPlacementScore {
    pub cluster: String,
    pub score: f64,
    pub reachable: bool,
    pub server: Option<String>,
    pub runtime_hint: String,
    pub reasons: Vec<String>,
    #[serde(default)]
    pub anomaly_count: u32,
    #[serde(default)]
    pub anomaly_penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPlan {
    pub workload: String,
    pub recommended_runtime: String,
    pub recommended_cluster: Option<String>,
    pub anomaly_signals_configured: bool,
    pub total_anomalies: u32,
    pub clusters: Vec<ClusterPlacementScore>,
}

pub fn federation_policies() -> FederationPolicy {
    let clusters: Vec<String> = std::env::var("AETHER_FEDERATION_CLUSTERS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut weights = HashMap::new();
    if let Ok(raw) = std::env::var("AETHER_FEDERATION_WEIGHTS") {
        for part in raw.split(',') {
            if let Some((k, v)) = part.split_once('=') {
                if let Ok(w) = v.trim().parse::<f64>() {
                    weights.insert(k.trim().to_string(), w);
                }
            }
        }
    }
    FederationPolicy { clusters, weights }
}

pub async fn plan_placement(spec: &Workload) -> Result<FederationPlan> {
    let engine = Engine::new();
    let runtime = engine.decide(spec)?;
    let runtime_hint = format!("{runtime:?}");

    let policy = federation_policies();
    let all_clusters = list_clusters().await.unwrap_or_default();
    let anomalies = load_placement_signals(&all_clusters).await;

    let candidates: Vec<ClusterInfo> = if policy.clusters.is_empty() {
        all_clusters
    } else {
        all_clusters
            .into_iter()
            .filter(|c| policy.clusters.iter().any(|p| p == &c.name))
            .collect()
    };

    let intel_recs = GlobalPlacementEngine::recommend(spec, &candidates);
    let mut clusters = score_clusters(&candidates, &policy, &runtime_hint, &anomalies, &intel_recs);

    clusters.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let recommended_cluster = clusters.first().map(|c| c.cluster.clone());

    Ok(FederationPlan {
        workload: spec.metadata.name.clone(),
        recommended_runtime: runtime_hint,
        recommended_cluster,
        anomaly_signals_configured: anomalies.configured,
        total_anomalies: anomalies.total_anomalies,
        clusters,
    })
}

/// Top cluster for GitOps auto-target when kube_context is omitted.
pub async fn recommend_cluster(spec: &Workload) -> Result<Option<String>> {
    Ok(plan_placement(spec).await?.recommended_cluster)
}

fn score_clusters(
    candidates: &[ClusterInfo],
    policy: &FederationPolicy,
    runtime_hint: &str,
    anomalies: &AnomalyPlacementSignals,
    intel_recs: &[crate::intelligence::placement::PlacementRecommendation],
) -> Vec<ClusterPlacementScore> {
    let mut out = Vec::new();
    for c in candidates {
        let mut score = if c.reachable { 50.0 } else { 5.0 };
        let mut reasons = Vec::new();
        if c.reachable {
            reasons.push("cluster reachable".into());
        } else {
            reasons.push("cluster unreachable".into());
        }
        if let Some(w) = policy.weights.get(&c.name) {
            score += w * 10.0;
            reasons.push(format!("federation weight {w}"));
        }
        if let Some(rec) = intel_recs
            .iter()
            .filter(|r| r.cluster.as_deref() == Some(c.name.as_str()))
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            score += rec.score * 40.0;
            reasons.push(format!("intelligence score {:.2}", rec.score));
            reasons.extend(rec.reasons.iter().take(2).cloned());
        }
        let penalty = anomalies.cluster_penalty(&c.name);
        if penalty > 0.0 {
            score = (score - penalty).max(0.0);
            reasons.extend(anomalies.cluster_reasons(&c.name));
        }
        let anomaly_count = anomalies
            .by_cluster
            .get(&c.name)
            .map(|s| s.count)
            .unwrap_or(0);
        out.push(ClusterPlacementScore {
            cluster: c.name.clone(),
            score,
            reachable: c.reachable,
            server: c.server.clone(),
            runtime_hint: runtime_hint.to_string(),
            reasons,
            anomaly_count,
            anomaly_penalty: penalty,
        });
    }
    out
}

pub fn parse_workload_yaml(yaml: &str) -> Result<Workload> {
    let spec: Workload = serde_yaml::from_str(yaml)?;
    spec.validate()?;
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policies_from_env() {
        std::env::set_var("AETHER_FEDERATION_CLUSTERS", "a,b");
        std::env::set_var("AETHER_FEDERATION_WEIGHTS", "a=2,b=1");
        let p = federation_policies();
        assert_eq!(p.clusters, vec!["a", "b"]);
        assert_eq!(p.weights.get("a"), Some(&2.0));
        std::env::remove_var("AETHER_FEDERATION_CLUSTERS");
        std::env::remove_var("AETHER_FEDERATION_WEIGHTS");
    }
}
