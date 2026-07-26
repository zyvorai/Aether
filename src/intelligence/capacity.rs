// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Predictive capacity scaling — HPA/VPA suggestions from failure signals.

use crate::intelligence::policy::AutonomyPolicy;
use crate::intelligence::predict::FailurePredictor;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleSuggestion {
    pub workload: String,
    pub resource: String,
    pub current: String,
    pub suggested: String,
    pub reason: String,
    pub kind: String,
    pub auto_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityScaleReport {
    pub generated_at: String,
    pub suggestions: Vec<ScaleSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityScaleExecuteRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityScaleExecuteReport {
    pub dry_run: bool,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn build_scale_suggestions(workloads: &[(Workload, WorkloadState)]) -> CapacityScaleReport {
    let predictions = FailurePredictor::predict_fleet(workloads);
    let mut suggestions = Vec::new();

    for (spec, ws) in workloads {
        let pred = predictions
            .predictions
            .iter()
            .find(|p| p.workload == spec.metadata.name);
        let Some(pred) = pred else { continue };

        for signal in &pred.predictions {
            if signal.kind == "oom" {
                let mem = spec.requirements.memory.clone();
                suggestions.push(ScaleSuggestion {
                    workload: spec.metadata.name.clone(),
                    resource: "memory".into(),
                    current: mem.clone(),
                    suggested: bump_memory(&mem),
                    reason: signal.reason.clone(),
                    kind: "vpa".into(),
                    auto_safe: pred.risk_level != "critical",
                });
            }
            if signal.kind == "instability" || signal.kind == "workload_failure" {
                let replicas = spec.scaling.as_ref().map(|s| s.min_replicas).unwrap_or(1);
                suggestions.push(ScaleSuggestion {
                    workload: spec.metadata.name.clone(),
                    resource: "replicas".into(),
                    current: replicas.to_string(),
                    suggested: (replicas + 1).to_string(),
                    reason: signal.reason.clone(),
                    kind: "hpa".into(),
                    auto_safe: ws.runtime == crate::runtime::RuntimeKind::Kubernetes,
                });
            }
        }

        if pred.risk_level == "critical"
            && suggestions.iter().all(|s| s.workload != spec.metadata.name)
        {
            let replicas = spec.scaling.as_ref().map(|s| s.min_replicas).unwrap_or(1);
            suggestions.push(ScaleSuggestion {
                workload: spec.metadata.name.clone(),
                resource: "replicas".into(),
                current: replicas.to_string(),
                suggested: (replicas + 1).to_string(),
                reason: "Critical failure risk — add headroom".into(),
                kind: "hpa".into(),
                auto_safe: false,
            });
        }
    }

    suggestions.sort_by(|a, b| a.workload.cmp(&b.workload));

    CapacityScaleReport {
        generated_at: crate::resources::now_rfc3339(),
        suggestions,
    }
}

pub fn execute_scale_suggestions(
    report: &CapacityScaleReport,
    policy: &AutonomyPolicy,
    dry_run: bool,
) -> CapacityScaleExecuteReport {
    let mut executed = Vec::new();
    let mut skipped = Vec::new();

    for suggestion in &report.suggestions {
        let action = format!(
            "{} {} {} → {} ({})",
            suggestion.workload,
            suggestion.kind,
            suggestion.current,
            suggestion.suggested,
            suggestion.resource
        );
        if !suggestion.auto_safe && !policy.allows_restart() {
            skipped.push(format!("{action}: requires autonomy.healing auto"));
            continue;
        }
        if dry_run {
            executed.push(format!("dry-run: {action}"));
        } else {
            skipped.push(format!(
                "{action} — not applied: capacity scale mutation not implemented"
            ));
        }
    }

    CapacityScaleExecuteReport {
        dry_run,
        executed,
        skipped,
    }
}

fn bump_memory(current: &str) -> String {
    let gi = crate::resources::parse_memory_gi(current);
    if gi >= 16.0 {
        format!("{}Gi", (gi * 1.25).ceil() as u32)
    } else if gi >= 4.0 {
        format!("{}Gi", (gi + 2.0) as u32)
    } else {
        format!("{}Gi", ((gi * 1.5).ceil() as u32).max(2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_memory_scales_up() {
        assert_eq!(bump_memory("4Gi"), "6Gi");
    }
}
