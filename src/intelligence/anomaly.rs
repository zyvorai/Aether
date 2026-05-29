// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! PacketWolf anomaly signals for intelligent placement scoring.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClusterAnomalySummary {
    pub cluster: String,
    pub count: u32,
    pub max_severity: f64,
    pub namespaces: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AnomalyPlacementSignals {
    pub configured: bool,
    pub total_anomalies: u32,
    pub by_cluster: HashMap<String, ClusterAnomalySummary>,
}

impl AnomalyPlacementSignals {
    /// Penalty subtracted from placement score (0.0–30.0).
    pub fn cluster_penalty(&self, cluster: &str) -> f64 {
        self.by_cluster
            .get(cluster)
            .map(|s| {
                let base = s.count as f64 * 2.0;
                let sev = s.max_severity * 5.0;
                (base + sev).min(30.0)
            })
            .unwrap_or(0.0)
    }

    pub fn cluster_reasons(&self, cluster: &str) -> Vec<String> {
        self.by_cluster
            .get(cluster)
            .map(|s| {
                vec![format!(
                    "PacketWolf: {} anomal{} (max severity {:.1})",
                    s.count,
                    if s.count == 1 { "y" } else { "ies" },
                    s.max_severity
                )]
            })
            .unwrap_or_default()
    }
}

/// Load anomaly signals from PacketWolf when configured.
pub async fn load_placement_signals(clusters: &[crate::kubecluster::ClusterInfo]) -> AnomalyPlacementSignals {
    if !crate::ecosystem::packetwolf::config().configured {
        return AnomalyPlacementSignals::default();
    }
    let raw = match crate::ecosystem::packetwolf::anomalies(Some(100)).await {
        Ok(v) => v,
        Err(_) => return AnomalyPlacementSignals { configured: true, ..Default::default() },
    };
    parse_anomaly_signals(&raw, clusters)
}

pub fn parse_anomaly_signals(raw: &Value, clusters: &[crate::kubecluster::ClusterInfo]) -> AnomalyPlacementSignals {
    let items = extract_anomaly_items(raw);
    let cluster_names: Vec<String> = clusters.iter().map(|c| c.name.clone()).collect();
    let mut by_cluster: HashMap<String, ClusterAnomalySummary> = HashMap::new();

    for item in items {
        let cluster = infer_cluster(&item, &cluster_names);
        let Some(cluster) = cluster else { continue };
        let sev = item
            .get("severity")
            .or_else(|| item.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let ns = item
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let entry = by_cluster.entry(cluster.clone()).or_insert(ClusterAnomalySummary {
            cluster: cluster.clone(),
            count: 0,
            max_severity: 0.0,
            namespaces: Vec::new(),
        });
        entry.count += 1;
        entry.max_severity = entry.max_severity.max(sev);
        if !entry.namespaces.contains(&ns) {
            entry.namespaces.push(ns);
        }
    }

    let total: u32 = by_cluster.values().map(|s| s.count).sum();
    AnomalyPlacementSignals {
        configured: true,
        total_anomalies: total,
        by_cluster,
    }
}

fn extract_anomaly_items(raw: &Value) -> Vec<Value> {
    if let Some(arr) = raw.as_array() {
        return arr.clone();
    }
    for key in ["anomalies", "data", "items", "results"] {
        if let Some(arr) = raw.get(key).and_then(|v| v.as_array()) {
            return arr.clone();
        }
        if let Some(inner) = raw.pointer("/data/anomalies").and_then(|v| v.as_array()) {
            return inner.clone();
        }
    }
    Vec::new()
}

fn infer_cluster(item: &Value, cluster_names: &[String]) -> Option<String> {
    for key in ["cluster", "kube_context", "context", "site"] {
        if let Some(c) = item.get(key).and_then(|v| v.as_str()) {
            if cluster_names.iter().any(|n| n == c) {
                return Some(c.to_string());
            }
            if let Some(m) = cluster_names.iter().find(|n| n.contains(c) || c.contains(n.as_str())) {
                return Some(m.clone());
            }
        }
    }
    if cluster_names.len() == 1 {
        return Some(cluster_names[0].clone());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_anomaly_payload() {
        let raw = serde_json::json!({
            "anomalies": [
                {"cluster": "prod", "namespace": "app", "severity": 3.0},
                {"cluster": "prod", "namespace": "db", "severity": 1.0}
            ]
        });
        let clusters = vec![crate::kubecluster::ClusterInfo {
            name: "prod".into(),
            server: None,
            version: None,
            reachable: true,
        }];
        let sig = parse_anomaly_signals(&raw, &clusters);
        assert_eq!(sig.total_anomalies, 2);
        assert!(sig.cluster_penalty("prod") > 0.0);
    }
}
