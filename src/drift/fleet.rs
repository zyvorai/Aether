// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Fleet-wide drift aggregation across Aether-managed workloads.

use super::{DriftDetector, DriftReport, DriftSeverity};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetDriftRow {
    pub workload: String,
    pub cluster: Option<String>,
    pub namespace: Option<String>,
    pub runtime: String,
    pub has_drift: bool,
    pub severity: DriftSeverity,
    pub drift_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetDriftSummary {
    pub total_workloads: usize,
    pub drifted: usize,
    pub critical: usize,
    pub warning: usize,
    pub rows: Vec<FleetDriftRow>,
}

pub fn scan_fleet(store: &StateStore) -> FleetDriftSummary {
    let detector = DriftDetector::new();
    let mut pairs: Vec<(Workload, WorkloadState)> = Vec::new();

    for ws in store.workloads.values() {
        if let Ok(spec) = Workload::from_file(&ws.spec_path) {
            pairs.push((spec, ws.clone()));
        }
    }

    let reports = detector.detect_all(&pairs);
    build_summary(&reports, &pairs)
}

fn build_summary(reports: &[DriftReport], pairs: &[(Workload, WorkloadState)]) -> FleetDriftSummary {
    let mut rows = Vec::new();
    let mut drifted = 0usize;
    let mut critical = 0usize;
    let mut warning = 0usize;

    for (report, (spec, ws)) in reports.iter().zip(pairs.iter()) {
        if report.has_drift {
            drifted += 1;
        }
        match report.severity {
            DriftSeverity::Critical => critical += 1,
            DriftSeverity::Warning => warning += 1,
            DriftSeverity::Info => {}
        }
        let (cluster, namespace) = k8s_meta(spec);
        rows.push(FleetDriftRow {
            workload: report.workload_name.clone(),
            cluster,
            namespace,
            runtime: format!("{}", ws.runtime),
            has_drift: report.has_drift,
            severity: report.severity.clone(),
            drift_count: report.drifts.len(),
        });
    }

    rows.sort_by(|a, b| {
        b.has_drift
            .cmp(&a.has_drift)
            .then_with(|| b.severity.cmp(&a.severity))
            .then_with(|| a.workload.cmp(&b.workload))
    });

    FleetDriftSummary {
        total_workloads: rows.len(),
        drifted,
        critical,
        warning,
        rows,
    }
}

fn k8s_meta(_spec: &Workload) -> (Option<String>, Option<String>) {
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Instance, RuntimeKind};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn empty_store_summary() {
        let store = StateStore::default();
        let s = scan_fleet(&store);
        assert_eq!(s.total_workloads, 0);
    }

    #[test]
    fn summary_counts_rows() {
        let mut store = StateStore::default();
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("w.yaml");
        std::fs::write(&spec_path, include_str!("../../examples/demo-webserver.yaml")).unwrap();
        store.workloads.insert(
            "demo".into(),
            WorkloadState::new(
                "demo".into(),
                RuntimeKind::Podman,
                Instance::new(
                    "id1".into(),
                    "demo".into(),
                    RuntimeKind::Podman,
                    "img:latest".into(),
                ),
                spec_path,
            ),
        );
        let s = scan_fleet(&store);
        assert_eq!(s.total_workloads, 1);
    }
}
