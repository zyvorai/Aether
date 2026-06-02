// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Unified context snapshot for copilot and intelligence APIs.

use crate::cost;
use crate::drift::DriftDetector;
use crate::events::EventBus;
use crate::health::HealthHistory;
use crate::intelligence::store::IntelligenceStore;
use crate::kubecluster;
use crate::observability;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct ContextSnapshot {
    pub generated_at: String,
    pub workload_count: usize,
    pub workloads: Vec<WorkloadContextEntry>,
    pub drift_summary: DriftSummary,
    pub recent_events: Vec<EventSummary>,
    pub observability: observability::ObservabilitySummary,
    pub chargeback_preview: Vec<ChargebackRow>,
    pub cluster_count: usize,
    pub intelligence_outcomes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkloadContextEntry {
    pub name: String,
    pub runtime: String,
    pub state: String,
    pub ready: bool,
    pub uptime_pct: f64,
    pub restart_count: u32,
    pub has_drift: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftSummary {
    pub workloads_with_drift: usize,
    pub total_drifts: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventSummary {
    pub timestamp: String,
    pub severity: String,
    pub category: String,
    pub message: String,
    pub workload: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChargebackRow {
    pub workload: String,
    pub owner: String,
    pub monthly_usd: f64,
}

pub async fn build_context_snapshot(state_path: &Path) -> anyhow::Result<ContextSnapshot> {
    let store = StateStore::load(state_path)?;
    let health = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
    let detector = DriftDetector::new();
    let intel = IntelligenceStore::load(&IntelligenceStore::default_path()).unwrap_or_default();

    let mut workloads = Vec::new();
    let mut drift_workloads = 0usize;
    let mut total_drifts = 0usize;

    for ws in store.list() {
        let (has_drift, drift_items) = drift_for_workload(&detector, ws);
        if has_drift {
            drift_workloads += 1;
            total_drifts += drift_items;
        }
        workloads.push(WorkloadContextEntry {
            name: ws.name.clone(),
            runtime: format!("{}", ws.runtime),
            state: format!("{:?}", ws.instance.id),
            ready: health.uptime_percent(&ws.name) >= 99.0 || health.restart_count(&ws.name) == 0,
            uptime_pct: health.uptime_percent(&ws.name),
            restart_count: health.restart_count(&ws.name),
            has_drift,
        });
    }

    let events = EventBus::load(&EventBus::default_path()).unwrap_or_default();
    let recent_events: Vec<EventSummary> = events
        .last_n(20)
        .into_iter()
        .map(|e| EventSummary {
            timestamp: e.timestamp.clone(),
            severity: format!("{:?}", e.severity),
            category: format!("{:?}", e.category),
            message: format!("{}: {}", e.title, e.message),
            workload: e.workload.clone(),
        })
        .collect();

    let cluster_metrics = if let Ok(clusters) = kubecluster::list_clusters().await {
        if let Some(c) = clusters.iter().find(|c| c.reachable) {
            kubecluster::metrics_summary(&c.name, None).await.ok()
        } else {
            None
        }
    } else {
        None
    };
    let cilium = kubecluster::cilium::cilium_status(None, None).await.ok();
    let observability = observability::build_summary(cluster_metrics, cilium);
    let clusters = kubecluster::list_clusters().await.unwrap_or_default();

    let chargeback_preview = chargeback_rows(&store);

    Ok(ContextSnapshot {
        generated_at: crate::resources::now_rfc3339(),
        workload_count: workloads.len(),
        workloads,
        drift_summary: DriftSummary {
            workloads_with_drift: drift_workloads,
            total_drifts,
        },
        recent_events,
        observability,
        chargeback_preview,
        cluster_count: clusters.len(),
        intelligence_outcomes: intel.outcomes.len(),
    })
}

fn drift_for_workload(detector: &DriftDetector, ws: &WorkloadState) -> (bool, usize) {
    match Workload::from_file(&ws.spec_path) {
        Ok(spec) => {
            let report = detector.detect(&spec, ws);
            (report.has_drift, report.drifts.len())
        }
        Err(_) => (false, 0),
    }
}

fn chargeback_rows(store: &StateStore) -> Vec<ChargebackRow> {
    let mut rows = Vec::new();
    for ws in store.list() {
        if let Ok(spec) = Workload::from_file(&ws.spec_path) {
            if let Ok(estimates) = cost::estimate_all_providers(&spec) {
                if let Some(first) = estimates.first() {
                    rows.push(ChargebackRow {
                        workload: ws.name.clone(),
                        owner: spec.metadata.owner.clone(),
                        monthly_usd: first.total_monthly,
                    });
                }
            }
        }
    }
    rows.sort_by(|a, b| {
        b.monthly_usd
            .partial_cmp(&a.monthly_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.truncate(10);
    rows
}
