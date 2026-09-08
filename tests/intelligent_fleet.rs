// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

use aether::intelligence::anomaly::parse_anomaly_signals;
use aether::migration::volume::{plan_volume_replication, VolumeReplicationRequest};

#[test]
fn anomaly_penalty_reduces_score_signal() {
    let raw = serde_json::json!({"anomalies": [{"cluster": "prod", "severity": 4.0}]});
    let clusters = vec![aether::kubecluster::ClusterInfo {
        name: "prod".into(),
        server: None,
        version: None,
        reachable: true,
    }];
    let sig = parse_anomaly_signals(&raw, &clusters);
    assert!(sig.cluster_penalty("prod") > 0.0);
}

#[test]
fn volume_replication_plan_steps() {
    let plan = plan_volume_replication(
        &VolumeReplicationRequest {
            source_cluster: "a".into(),
            target_cluster: "b".into(),
            namespace: "default".into(),
            pvc_name: "data".into(),
            workload_name: None,
        },
        None,
    )
    .unwrap();
    assert_eq!(plan.steps.len(), 5);
    assert!(!plan.warnings.is_empty());
}
