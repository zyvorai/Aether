// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Digital twin — what-if simulation over fleet capacity, cost, and risk.

use crate::cost;
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::predict::FailurePredictor;
use crate::runtime::RuntimeKind;
use crate::scheduler::Scheduler;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinSimulateRequest {
    #[serde(default)]
    pub workload: Option<String>,
    #[serde(default = "default_scale")]
    pub scale_factor: f64,
    #[serde(default)]
    pub target_runtime: Option<String>,
}

fn default_scale() -> f64 {
    1.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinSimulateReport {
    pub generated_at: String,
    pub scenario: String,
    pub baseline: TwinSnapshot,
    pub projected: TwinSnapshot,
    pub deltas: TwinDeltas,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinSnapshot {
    pub fleet_risk_score: f64,
    pub avg_cpu_utilization: f64,
    pub avg_memory_utilization: f64,
    pub estimated_monthly_cost_usd: f64,
    pub saturation_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinDeltas {
    pub risk_delta: f64,
    pub cpu_util_delta: f64,
    pub memory_util_delta: f64,
    pub cost_delta_usd: f64,
}

pub struct DigitalTwinEngine;

impl DigitalTwinEngine {
    pub fn simulate(
        pairs: &[(Workload, WorkloadState)],
        req: &TwinSimulateRequest,
    ) -> TwinSimulateReport {
        let scale = req.scale_factor.clamp(1.0, 5.0);
        let scoped: Vec<_> = if let Some(ref name) = req.workload {
            pairs
                .iter()
                .filter(|(s, _)| s.metadata.name == *name)
                .cloned()
                .collect()
        } else {
            pairs.to_vec()
        };
        let active = if scoped.is_empty() { pairs } else { &scoped };

        let predictions = FailurePredictor::predict_fleet(active);
        let finops = FinOpsEngine::optimize_fleet(active);
        let (cpu_util, mem_util) = avg_scheduler_utilization();
        let monthly_cost = estimate_fleet_monthly(active);

        let baseline = TwinSnapshot {
            fleet_risk_score: predictions.fleet_risk_score,
            avg_cpu_utilization: cpu_util,
            avg_memory_utilization: mem_util,
            estimated_monthly_cost_usd: monthly_cost,
            saturation_days: saturation_days(cpu_util, mem_util),
        };

        let runtime_multiplier = req
            .target_runtime
            .as_ref()
            .and_then(|t| t.parse::<RuntimeKind>().ok())
            .map(|rt| match rt {
                RuntimeKind::Metal3 => 1.15,
                RuntimeKind::KubeVirt => 1.08,
                RuntimeKind::Kubernetes => 1.0,
                RuntimeKind::Podman | RuntimeKind::Docker => 0.92,
            })
            .unwrap_or(1.0);

        let risk_boost = if scale > 2.0 { 1.25 } else { 1.0 };
        let projected_cpu = (cpu_util * scale * runtime_multiplier).min(0.99);
        let projected_mem = (mem_util * scale * runtime_multiplier * 0.95).min(0.99);
        let projected_cost = monthly_cost * scale * runtime_multiplier;
        let projected_risk =
            (predictions.fleet_risk_score * scale * 0.12 * risk_boost).min(1.0);

        let projected = TwinSnapshot {
            fleet_risk_score: projected_risk,
            avg_cpu_utilization: projected_cpu,
            avg_memory_utilization: projected_mem,
            estimated_monthly_cost_usd: projected_cost,
            saturation_days: saturation_days(projected_cpu, projected_mem),
        };

        let mut recommendations = Vec::new();
        if projected_cpu >= 0.85 {
            recommendations.push(
                "CPU saturation projected within two weeks — add nodes or right-size workloads."
                    .into(),
            );
        }
        if projected_mem >= 0.85 {
            recommendations
                .push("Memory pressure rising — review OOM-prone workloads and limits.".into());
        }
        if finops.recommendations.first().is_some() && scale <= 1.2 {
            recommendations.push(
                "FinOps suggests runtime shifts before scaling — check Cost Intelligence."
                    .into(),
            );
        }
        if projected_risk > baseline.fleet_risk_score + 0.15 {
            recommendations
                .push("Failure risk increases materially — run fleet root cause analysis.".into());
        }
        if recommendations.is_empty() {
            recommendations.push("Scenario within safe operating bounds.".into());
        }

        let scenario = if let Some(ref name) = req.workload {
            format!(
                "Scale {name} ×{scale:.1}{}",
                req.target_runtime
                    .as_ref()
                    .map(|r| format!(" → {r}"))
                    .unwrap_or_default()
            )
        } else {
            format!(
                "Fleet scale ×{scale:.1}{}",
                req.target_runtime
                    .as_ref()
                    .map(|r| format!(" → {r}"))
                    .unwrap_or_default()
            )
        };

        TwinSimulateReport {
            generated_at: crate::resources::now_rfc3339(),
            scenario,
            deltas: TwinDeltas {
                risk_delta: projected.fleet_risk_score - baseline.fleet_risk_score,
                cpu_util_delta: projected.avg_cpu_utilization - baseline.avg_cpu_utilization,
                memory_util_delta: projected.avg_memory_utilization
                    - baseline.avg_memory_utilization,
                cost_delta_usd: projected.estimated_monthly_cost_usd
                    - baseline.estimated_monthly_cost_usd,
            },
            baseline,
            projected,
            recommendations,
        }
    }
}

fn avg_scheduler_utilization() -> (f64, f64) {
    let path = Scheduler::default_path();
    let Ok(scheduler) = Scheduler::load(&path) else {
        return (0.35, 0.4);
    };
    let utils = scheduler.utilization_summary();
    if utils.is_empty() {
        return (0.35, 0.4);
    }
    let cpu = utils.iter().map(|u| u.cpu_utilization).sum::<f64>() / utils.len() as f64;
    let mem = utils.iter().map(|u| u.memory_utilization).sum::<f64>() / utils.len() as f64;
    (cpu, mem)
}

fn estimate_fleet_monthly(pairs: &[(Workload, WorkloadState)]) -> f64 {
    pairs
        .iter()
        .filter_map(|(spec, _)| {
            cost::estimate_all_providers(spec)
                .ok()?
                .into_iter()
                .find(|e| e.provider == cost::CloudProvider::AWS)
                .map(|e| e.total_monthly)
        })
        .sum()
}

fn saturation_days(cpu: f64, mem: f64) -> u32 {
    let peak = cpu.max(mem);
    if peak >= 0.9 {
        7
    } else if peak >= 0.75 {
        14
    } else if peak >= 0.6 {
        21
    } else {
        45
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Instance, RuntimeKind};

    #[test]
    fn simulate_increases_utilization_with_scale() {
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("web.yaml");
        std::fs::write(
            &spec_path,
            include_str!("../../examples/labs/kubernetes/workload.yaml").replace("nginx", "web"),
        )
        .unwrap();
        let spec = Workload::from_file(&spec_path).unwrap();
        let ws = WorkloadState {
            name: "web".into(),
            runtime: RuntimeKind::Kubernetes,
            instance: Instance {
                id: "web-1".into(),
                name: "web".into(),
                runtime: RuntimeKind::Kubernetes,
                image: "web:latest".into(),
                created_at: "2024-01-01T00:00:00Z".into(),
            },
            spec_path,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
            os_version: None,
            node_labels: vec![],
        };
        let pairs = vec![(spec, ws)];
        let report = DigitalTwinEngine::simulate(
            &pairs,
            &TwinSimulateRequest {
                workload: None,
                scale_factor: 2.0,
                target_runtime: None,
            },
        );
        assert!(
            report.projected.avg_cpu_utilization >= report.baseline.avg_cpu_utilization,
            "scale should not decrease CPU utilization"
        );
        assert!(report.deltas.cost_delta_usd >= 0.0);
        assert!(!report.recommendations.is_empty());
    }
}
