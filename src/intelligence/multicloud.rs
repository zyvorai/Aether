// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Multi-cloud fleet posture — cluster reachability, placement, and anomalies.

use crate::fleet::federation::{federation_policies, FederationPolicy};
use crate::intelligence::anomaly::load_placement_signals;
use crate::kubecluster::list_clusters;
use crate::spec::Workload;
use crate::state::StateStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterPosture {
    pub cluster: String,
    pub reachable: bool,
    pub server: Option<String>,
    pub workload_count: u32,
    pub runtimes: Vec<String>,
    pub anomaly_count: u32,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCloudPostureReport {
    pub generated_at: String,
    pub federation_enabled: bool,
    pub federation_clusters: Vec<String>,
    pub clusters: Vec<ClusterPosture>,
    pub fleet_workloads: u32,
    pub reachable_clusters: u32,
    pub recommended_actions: Vec<String>,
}

pub async fn build_multicloud_posture(
    state_path: &Path,
) -> anyhow::Result<MultiCloudPostureReport> {
    let store = StateStore::load(state_path)?;
    let policy = federation_policies();
    let all_clusters = list_clusters().await.unwrap_or_default();
    let anomalies = load_placement_signals(&all_clusters).await;

    let mut runtime_by_cluster: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let mut fleet_workloads = 0u32;

    for ws in store.list() {
        fleet_workloads += 1;
        let Ok(_spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        let cluster_key = ws.runtime.to_string().to_lowercase();
        let bucket = runtime_by_cluster.entry(cluster_key).or_default();
        *bucket.entry(format!("{}", ws.runtime)).or_insert(0) += 1;
    }

    let mut clusters = Vec::new();
    for cluster in &all_clusters {
        let anomaly_count = anomalies
            .by_cluster
            .get(&cluster.name)
            .map(|s| s.count)
            .unwrap_or(0);
        let weight = policy.weights.get(&cluster.name).copied().unwrap_or(1.0);
        let reach_bonus = if cluster.reachable { 0.2 } else { -0.4 };
        let anomaly_penalty = (anomaly_count as f64 * 0.05).min(0.35);
        let score = (weight + reach_bonus - anomaly_penalty).clamp(0.0, 1.0);

        let runtimes: Vec<String> = runtime_by_cluster
            .get(&cluster.name.to_lowercase())
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();

        clusters.push(ClusterPosture {
            cluster: cluster.name.clone(),
            reachable: cluster.reachable,
            server: cluster.server.clone(),
            workload_count: runtimes
                .iter()
                .map(|r| {
                    runtime_by_cluster
                        .get(&cluster.name.to_lowercase())
                        .and_then(|m| m.get(r))
                        .copied()
                        .unwrap_or(0)
                })
                .sum(),
            runtimes,
            anomaly_count,
            score,
        });
    }

    clusters.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let reachable_clusters = clusters.iter().filter(|c| c.reachable).count() as u32;
    let recommended_actions =
        build_recommendations(&policy, &clusters, fleet_workloads, anomalies.configured);

    Ok(MultiCloudPostureReport {
        generated_at: crate::resources::now_rfc3339(),
        federation_enabled: !policy.clusters.is_empty(),
        federation_clusters: policy.clusters.clone(),
        clusters,
        fleet_workloads,
        reachable_clusters,
        recommended_actions,
    })
}

fn build_recommendations(
    policy: &FederationPolicy,
    clusters: &[ClusterPosture],
    fleet_workloads: u32,
    anomalies_configured: bool,
) -> Vec<String> {
    let mut recs = Vec::new();
    if fleet_workloads == 0 {
        recs.push("Deploy workloads to activate multi-cloud placement scoring.".into());
    }
    if policy.clusters.is_empty() {
        recs.push("Set AETHER_FEDERATION_CLUSTERS to enable federated GitOps targets.".into());
    }
    if clusters.iter().any(|c| !c.reachable) {
        recs.push("Some clusters are unreachable — verify kubeconfig contexts and network.".into());
    }
    if anomalies_configured && clusters.iter().any(|c| c.anomaly_count > 0) {
        recs.push(
            "PacketWolf anomalies detected — review Fleet placement tab before migrations.".into(),
        );
    }
    if recs.is_empty() {
        recs.push(
            "Multi-cloud posture healthy — placement engine can rank clusters autonomously.".into(),
        );
    }
    recs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn multicloud_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let report = build_multicloud_posture(&path).await.unwrap();
        assert_eq!(report.fleet_workloads, 0);
        assert!(!report.recommended_actions.is_empty());
    }
}
