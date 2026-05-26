// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Failure prediction from health history and metrics trends.

use crate::health::HealthHistory;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionReport {
    pub generated_at: String,
    pub fleet_risk_score: f64,
    pub predictions: Vec<WorkloadPrediction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadPrediction {
    pub workload: String,
    pub risk_score: f64,
    pub risk_level: String,
    pub predictions: Vec<FailureSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureSignal {
    pub kind: String,
    pub probability: f64,
    pub horizon: String,
    pub reason: String,
}

pub struct FailurePredictor;

impl FailurePredictor {
    pub fn predict_fleet(workloads: &[(Workload, WorkloadState)]) -> PredictionReport {
        let health = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
        let mut predictions = Vec::new();
        let mut total_risk = 0.0;

        for (spec, ws) in workloads {
            let p = Self::predict_workload(spec, ws, &health);
            total_risk += p.risk_score;
            predictions.push(p);
        }

        predictions.sort_by(|a, b| {
            b.risk_score
                .partial_cmp(&a.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let fleet_risk = if predictions.is_empty() {
            0.0
        } else {
            total_risk / predictions.len() as f64
        };

        PredictionReport {
            generated_at: crate::resources::now_rfc3339(),
            fleet_risk_score: fleet_risk,
            predictions,
        }
    }

    pub fn predict_workload(
        spec: &Workload,
        ws: &WorkloadState,
        health: &HealthHistory,
    ) -> WorkloadPrediction {
        let name = spec.metadata.name.clone();
        let uptime = health.uptime_percent(&name);
        let restarts = health.restart_count(&name);
        let mut signals = Vec::new();
        let mut risk: f64 = 0.0;

        if uptime > 0.0 && uptime < 99.0 {
            risk += 0.35;
            signals.push(FailureSignal {
                kind: "sla_breach".into(),
                probability: (100.0 - uptime) / 100.0,
                horizon: "ongoing".into(),
                reason: format!("Uptime {:.1}% below 99% SLA baseline", uptime),
            });
        }

        if restarts >= 3 {
            risk += 0.25;
            signals.push(FailureSignal {
                kind: "instability".into(),
                probability: (restarts as f64 / 10.0).min(0.95),
                horizon: "24h".into(),
                reason: format!("{restarts} restarts recorded in health history"),
            });
        }

        let mem = crate::resources::parse_memory_gi(&spec.requirements.memory);
        if mem >= 8.0 && health.restart_count(&name) >= 2 {
            risk += 0.3;
            signals.push(FailureSignal {
                kind: "oom".into(),
                probability: 0.55,
                horizon: "4h".into(),
                reason: "High memory request with not-ready instance".into(),
            });
        }

        if health.restart_count(&name) >= 5
            && matches!(ws.runtime, crate::runtime::RuntimeKind::Kubernetes)
        {
            risk += 0.5;
            signals.push(FailureSignal {
                kind: "workload_failure".into(),
                probability: 0.85,
                horizon: "immediate".into(),
                reason: "Instance in failed state".into(),
            });
        }

        risk = risk.min(1.0);
        let risk_level = if risk >= 0.7 {
            "critical"
        } else if risk >= 0.4 {
            "high"
        } else if risk >= 0.2 {
            "medium"
        } else {
            "low"
        }
        .to_string();

        WorkloadPrediction {
            workload: name,
            risk_score: risk,
            risk_level,
            predictions: signals,
        }
    }
}
