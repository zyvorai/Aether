// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! FinOps intelligence — utilization-aware cost optimization.

use crate::cost::{self, CloudProvider};
use crate::intelligence::profile::WorkloadBehaviorProfile;
use crate::intelligence::store::IntelligenceStore;
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizeReport {
    pub generated_at: String,
    pub total_potential_savings_pct: f64,
    pub recommendations: Vec<CostOptimizeRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizeRecommendation {
    pub workload: String,
    pub current_runtime: String,
    pub suggested_runtime: String,
    pub savings_pct: f64,
    pub savings_monthly_usd: f64,
    pub risk: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostApplyPatch {
    pub workload: String,
    pub field: String,
    pub before: String,
    pub after: String,
    pub savings_monthly_usd: f64,
    pub patch_yaml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostApplyRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostApplyReport {
    pub dry_run: bool,
    pub patches: Vec<CostApplyPatch>,
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
}

pub struct FinOpsEngine;

impl FinOpsEngine {
    pub fn optimize_fleet(workloads: &[(Workload, WorkloadState)]) -> CostOptimizeReport {
        let intel = IntelligenceStore::load(&IntelligenceStore::default_path()).unwrap_or_default();
        let mut recommendations = Vec::new();
        let mut total_savings = 0.0;
        let mut total_current = 0.0;

        for (spec, ws) in workloads {
            if let Some(rec) = Self::optimize_workload(spec, ws, &intel.behavior_profiles) {
                total_current += rec.savings_monthly_usd / (rec.savings_pct / 100.0).max(0.01);
                total_savings += rec.savings_monthly_usd;
                recommendations.push(rec);
            }
        }

        recommendations.sort_by(|a, b| {
            b.savings_monthly_usd
                .partial_cmp(&a.savings_monthly_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let pct = if total_current > 0.0 {
            (total_savings / total_current) * 100.0
        } else {
            0.0
        };

        CostOptimizeReport {
            generated_at: crate::resources::now_rfc3339(),
            total_potential_savings_pct: pct,
            recommendations,
        }
    }

    pub fn build_cost_patches(workloads: &[(Workload, WorkloadState)]) -> Vec<CostApplyPatch> {
        let report = Self::optimize_fleet(workloads);
        report
            .recommendations
            .iter()
            .map(|rec| CostApplyPatch {
                workload: rec.workload.clone(),
                field: "runtime".into(),
                before: rec.current_runtime.clone(),
                after: rec.suggested_runtime.clone(),
                savings_monthly_usd: rec.savings_monthly_usd,
                patch_yaml: format!(
                    "# FinOps right-size patch for {}\nmetadata:\n  name: {}\nspec:\n  runtime: {}\n# was: {}",
                    rec.workload, rec.workload, rec.suggested_runtime, rec.current_runtime
                ),
            })
            .collect()
    }

    pub fn apply_cost_patches(
        patches: &[CostApplyPatch],
        dry_run: bool,
        policy: &crate::intelligence::policy::AutonomyPolicy,
    ) -> CostApplyReport {
        let mut applied = Vec::new();
        let mut skipped = Vec::new();
        if !policy.allows_auto_evolve() {
            for patch in patches {
                skipped.push(format!(
                    "{} runtime patch: cost agent requires autonomy.evolution auto",
                    patch.workload
                ));
            }
            return CostApplyReport {
                dry_run,
                patches: patches.to_vec(),
                applied,
                skipped,
            };
        }
        for patch in patches {
            let line = format!(
                "right-size {} runtime {} → {} (${:.0}/mo)",
                patch.workload, patch.before, patch.after, patch.savings_monthly_usd
            );
            if dry_run {
                applied.push(format!("dry-run: {line}"));
            } else {
                applied.push(line);
            }
        }
        CostApplyReport {
            dry_run,
            patches: patches.to_vec(),
            applied,
            skipped,
        }
    }

    fn optimize_workload(
        spec: &Workload,
        ws: &WorkloadState,
        profiles: &std::collections::HashMap<String, WorkloadBehaviorProfile>,
    ) -> Option<CostOptimizeRecommendation> {
        let current_cost = cost::estimate_all_providers(spec).ok()?;
        let aws = current_cost
            .iter()
            .find(|e| e.provider == CloudProvider::AWS)?;
        let current_monthly = aws.total_monthly;
        if current_monthly <= 0.0 {
            return None;
        }

        let profile = profiles.get(&spec.metadata.name);
        let suggested = if profile.map(|p| p.gpu_contention).unwrap_or(false) {
            RuntimeKind::Metal3
        } else if profile.map(|p| p.cpu_bursty).unwrap_or(false) {
            RuntimeKind::KubeVirt
        } else if ws.runtime == RuntimeKind::KubeVirt || ws.runtime == RuntimeKind::Metal3 {
            RuntimeKind::Kubernetes
        } else {
            return None;
        };

        if suggested == ws.runtime {
            return None;
        }

        let savings_pct = match suggested {
            RuntimeKind::Kubernetes => 25.0,
            RuntimeKind::Podman | RuntimeKind::Docker => 40.0,
            RuntimeKind::KubeVirt => 15.0,
            RuntimeKind::Metal3 => -10.0,
        };

        if savings_pct <= 0.0 {
            return None;
        }

        Some(CostOptimizeRecommendation {
            workload: spec.metadata.name.clone(),
            current_runtime: format!("{}", ws.runtime),
            suggested_runtime: format!("{suggested}"),
            savings_pct,
            savings_monthly_usd: current_monthly * (savings_pct / 100.0),
            risk: if spec.persistence.enabled {
                "medium".into()
            } else {
                "low".into()
            },
            reason: format!(
                "Move from {} to {} for better cost-performance balance",
                ws.runtime, suggested
            ),
        })
    }
}
