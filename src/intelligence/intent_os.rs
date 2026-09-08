// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Intent & Infrastructure platform — NL intent, violations, deploy, templates, versioning.

use crate::cost::{self, CloudProvider};
use crate::drift::fleet::scan_fleet;
use crate::health::HealthHistory;
use crate::intelligence::pipeline::{
    build_intent_pipeline, build_spec_from_goals, intent_block_yaml, IntentPipelineRequest,
};
use crate::policy::PolicyEngine;
use crate::spec::{ComplianceSpec, IntentGoal, IntentSpec, ResilienceLevel, Workload};
use crate::state::StateStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Phase 15: NL intent ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NlIntentRequest {
    pub text: String,
    pub workload_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NlIntentReport {
    pub generated_at: String,
    pub parsed_goal: String,
    pub intent_yaml: String,
    pub spec_yaml: String,
    pub confidence: f64,
    pub keywords: Vec<String>,
}

pub fn parse_nl_intent(req: &NlIntentRequest) -> anyhow::Result<NlIntentReport> {
    let text = req.text.to_lowercase();
    let mut keywords = Vec::new();
    let goal = if text.contains("cost") || text.contains("cheap") || text.contains("budget") {
        keywords.push("cost".into());
        IntentGoal::CostOptimized
    } else if text.contains("latency") || text.contains("fast") || text.contains("performance") {
        keywords.push("latency".into());
        IntentGoal::LowLatency
    } else if text.contains("throughput") || text.contains("scale") {
        keywords.push("throughput".into());
        IntentGoal::HighThroughput
    } else {
        keywords.push("balanced".into());
        IntentGoal::Balanced
    };

    let mut intent = IntentSpec {
        goal,
        sla: None,
        budget: None,
        resilience: None,
        compliance: None,
        trust: None,
        storage: None,
    };

    if text.contains("99.9") || text.contains("availability") || text.contains("uptime") {
        keywords.push("sla".into());
        intent.sla = Some(crate::spec::IntentSla {
            max_latency_ms: if text.contains("latency") || text.contains("ms") {
                Some(100)
            } else {
                None
            },
            min_availability_pct: Some(99.9),
        });
    }
    if text.contains("$") || text.contains("budget") || text.contains("month") {
        keywords.push("budget".into());
        let max = extract_budget_usd(&text).unwrap_or(500.0);
        intent.budget = Some(crate::spec::IntentBudget {
            max_monthly_usd: max,
        });
    }
    if text.contains("encrypt") || text.contains("compliance") || text.contains("isolated") {
        keywords.push("compliance".into());
        intent.compliance = Some(ComplianceSpec {
            isolation_required: text.contains("isolat"),
            encryption_required: text.contains("encrypt"),
        });
    }
    if text.contains("ha") || text.contains("high availability") {
        intent.resilience = Some(ResilienceLevel::High);
    }

    let name = req
        .workload_name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "intent-app".into());

    let pipeline_req = IntentPipelineRequest {
        yaml: None,
        goals: vec![goal_label(&intent.goal).into()],
        workload_name: Some(name.clone()),
    };
    let mut spec = build_spec_from_goals(&pipeline_req)?;
    spec.intent = Some(intent);
    spec.validate()?;

    let intent_yaml = serde_yaml::to_string(spec.intent.as_ref().unwrap())?;
    let spec_yaml = serde_yaml::to_string(&spec)?;
    let confidence = if keywords.len() >= 2 { 0.85 } else { 0.65 };

    Ok(NlIntentReport {
        generated_at: crate::resources::now_rfc3339(),
        parsed_goal: goal_label(&spec.intent.as_ref().unwrap().goal).into(),
        intent_yaml,
        spec_yaml,
        confidence,
        keywords,
    })
}

fn extract_budget_usd(text: &str) -> Option<f64> {
    for word in text.split_whitespace() {
        let cleaned = word
            .trim_start_matches('$')
            .trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.');
        if let Ok(v) = cleaned.parse::<f64>() {
            if v > 0.0 {
                return Some(v);
            }
        }
    }
    None
}

fn goal_label(goal: &IntentGoal) -> &'static str {
    match goal {
        IntentGoal::LowLatency => "low-latency",
        IntentGoal::HighThroughput => "high-throughput",
        IntentGoal::CostOptimized => "cost-optimized",
        IntentGoal::Balanced => "balanced",
    }
}

// ── Phase 16: Pipeline deploy ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDeployRequest {
    #[serde(flatten)]
    pub pipeline: IntentPipelineRequest,
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDeployReport {
    pub dry_run: bool,
    pub workload_name: String,
    pub validated: bool,
    pub policy_passed: bool,
    pub compliance_passed: bool,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub spec_path: Option<String>,
    pub steps: Vec<String>,
}

pub async fn deploy_intent_pipeline(
    req: &IntentDeployRequest,
) -> anyhow::Result<IntentDeployReport> {
    let report = build_intent_pipeline(&req.pipeline).await?;
    let spec: Workload = serde_yaml::from_str(&report.spec_yaml)?;
    spec.validate()?;

    let policy = PolicyEngine::production().evaluate(&spec);
    let compliance = check_compliance_gate(&spec);
    let mut steps = vec![
        "intent pipeline generated spec".into(),
        "spec validation passed".into(),
    ];

    if policy.passed {
        steps.push("policy check passed".into());
    } else {
        let msgs: Vec<String> = policy
            .violations
            .iter()
            .map(|v| v.message.clone())
            .collect();
        steps.push(format!("policy blocked: {}", msgs.join("; ")));
    }
    if compliance.allowed {
        steps.push("compliance gate passed".into());
    } else {
        steps.push(format!(
            "compliance blocked: {}",
            compliance.violations.join("; ")
        ));
    }

    let blocked = !policy.passed || !compliance.allowed;
    let block_reason = if !policy.passed {
        Some(
            policy
                .violations
                .iter()
                .map(|v| v.message.clone())
                .collect::<Vec<_>>()
                .join("; "),
        )
    } else if !compliance.allowed {
        Some(compliance.violations.join("; "))
    } else {
        None
    };

    let mut spec_path = None;
    if !req.dry_run && !blocked {
        let path = save_spec_for_deploy(&spec)?;
        steps.push(format!("spec written to {}", path.display()));
        record_intent_version(&spec.metadata.name, &spec)?;
        spec_path = Some(path.display().to_string());
    } else if req.dry_run {
        steps.push("dry-run: spec not written".into());
    }

    Ok(IntentDeployReport {
        dry_run: req.dry_run,
        workload_name: spec.metadata.name.clone(),
        validated: true,
        policy_passed: policy.passed,
        compliance_passed: compliance.allowed,
        blocked,
        block_reason,
        spec_path,
        steps,
    })
}

fn save_spec_for_deploy(spec: &Workload) -> anyhow::Result<PathBuf> {
    let dir = crate::resources::aether_path("intent-specs");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.yaml", spec.metadata.name));
    let tmp = path.with_extension("yaml.tmp");
    let yaml = serde_yaml::to_string(spec)?;
    std::fs::write(&tmp, yaml.as_bytes())?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

// ── Phase 17: Violations ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentViolationEntry {
    pub workload: String,
    pub violation_type: String,
    pub severity: String,
    pub detail: String,
    pub current_value: String,
    pub intent_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentViolationsReport {
    pub generated_at: String,
    pub violations: Vec<IntentViolationEntry>,
    pub reconciliation_status: String,
}

pub fn scan_intent_violations(state_path: &Path) -> anyhow::Result<IntentViolationsReport> {
    let store = StateStore::load(state_path)?;
    let health = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
    let mut violations = Vec::new();

    for ws in store.list() {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        let Some(intent) = spec.intent.as_ref() else {
            continue;
        };

        if let Some(ref sla) = intent.sla {
            if let Some(min_avail) = sla.min_availability_pct {
                let uptime = health.uptime_percent(&ws.name);
                if uptime > 0.0 && uptime < min_avail {
                    violations.push(IntentViolationEntry {
                        workload: ws.name.clone(),
                        violation_type: "sla_availability".into(),
                        severity: "high".into(),
                        detail: "Uptime below intent minimum".into(),
                        current_value: format!("{uptime:.1}%"),
                        intent_target: format!("{min_avail:.1}%"),
                    });
                }
            }
        }

        if let Some(ref budget) = intent.budget {
            if let Ok(estimates) = cost::estimate_all_providers(&spec) {
                if let Some(aws) = estimates.iter().find(|e| e.provider == CloudProvider::AWS) {
                    if aws.total_monthly > budget.max_monthly_usd {
                        violations.push(IntentViolationEntry {
                            workload: ws.name.clone(),
                            violation_type: "budget".into(),
                            severity: "medium".into(),
                            detail: "Estimated monthly cost exceeds intent budget".into(),
                            current_value: format!("${:.0}/mo", aws.total_monthly),
                            intent_target: format!("${:.0}/mo max", budget.max_monthly_usd),
                        });
                    }
                }
            }
        }

        let compliance = check_compliance_gate(&spec);
        if !compliance.allowed {
            violations.push(IntentViolationEntry {
                workload: ws.name.clone(),
                violation_type: "compliance".into(),
                severity: "critical".into(),
                detail: compliance.violations.join("; "),
                current_value: "non-compliant".into(),
                intent_target: "compliance spec".into(),
            });
        }
    }

    let status = if violations.is_empty() {
        "in_sync"
    } else {
        "violations_detected"
    };

    Ok(IntentViolationsReport {
        generated_at: crate::resources::now_rfc3339(),
        violations,
        reconciliation_status: status.into(),
    })
}

// ── Phase 18: SLA breaches for Command Center ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentSlaBreach {
    pub workload: String,
    pub metric: String,
    pub current: String,
    pub target: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentSlaBreachesReport {
    pub generated_at: String,
    pub breaches: Vec<IntentSlaBreach>,
}

pub fn build_intent_sla_breaches(state_path: &Path) -> anyhow::Result<IntentSlaBreachesReport> {
    let report = scan_intent_violations(state_path)?;
    let breaches = report
        .violations
        .iter()
        .filter(|v| v.violation_type.starts_with("sla"))
        .map(|v| IntentSlaBreach {
            workload: v.workload.clone(),
            metric: v.violation_type.clone(),
            current: v.current_value.clone(),
            target: v.intent_target.clone(),
            severity: v.severity.clone(),
        })
        .collect();
    Ok(IntentSlaBreachesReport {
        generated_at: crate::resources::now_rfc3339(),
        breaches,
    })
}

// ── Phase 19: Budget enforcement ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetEnforceRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetEnforceAction {
    pub workload: String,
    pub estimated_usd: f64,
    pub budget_usd: f64,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetEnforceReport {
    pub dry_run: bool,
    pub actions: Vec<BudgetEnforceAction>,
}

pub fn enforce_intent_budget(
    state_path: &Path,
    dry_run: bool,
) -> anyhow::Result<BudgetEnforceReport> {
    let store = StateStore::load(state_path)?;
    let mut actions = Vec::new();

    for ws in store.list() {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        let Some(ref intent) = spec.intent else {
            continue;
        };
        let Some(ref budget) = intent.budget else {
            continue;
        };
        let Ok(estimates) = cost::estimate_all_providers(&spec) else {
            continue;
        };
        let Some(aws) = estimates.iter().find(|e| e.provider == CloudProvider::AWS) else {
            continue;
        };
        if aws.total_monthly <= budget.max_monthly_usd {
            continue;
        }
        let action = if dry_run {
            format!(
                "dry-run: right-size or migrate {} (${:.0} > ${:.0})",
                ws.name, aws.total_monthly, budget.max_monthly_usd
            )
        } else {
            format!(
                "flagged {} for FinOps review (${:.0} > ${:.0})",
                ws.name, aws.total_monthly, budget.max_monthly_usd
            )
        };
        actions.push(BudgetEnforceAction {
            workload: ws.name.clone(),
            estimated_usd: aws.total_monthly,
            budget_usd: budget.max_monthly_usd,
            action,
        });
    }

    Ok(BudgetEnforceReport { dry_run, actions })
}

// ── Phase 20: Compliance gates ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceGateReport {
    pub allowed: bool,
    pub violations: Vec<String>,
    pub recommendations: Vec<String>,
}

pub fn check_compliance_gate(spec: &Workload) -> ComplianceGateReport {
    let mut violations = Vec::new();
    let mut recommendations = Vec::new();

    if let Some(ref intent) = spec.intent {
        if let Some(ref compliance) = intent.compliance {
            if compliance.encryption_required
                && spec.confidential.as_ref().is_none_or(|c| !c.enabled)
            {
                violations.push("encryption_required but confidential block not enabled".into());
                recommendations.push("Enable confidential.encryption or confidential block".into());
            }
            if compliance.isolation_required {
                recommendations.push("Prefer kubevirt runtime for isolation_required intent".into());
            }
        }

        if spec.confidential.as_ref().is_some_and(|c| c.enabled) {
            // Sovereign evaluation requires Ragnarok (separate product).
        }
    }

    ComplianceGateReport {
        allowed: violations.is_empty(),
        violations,
        recommendations,
    }
}

// ── Phase 21: Template library ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentTemplate {
    pub id: String,
    pub goal: String,
    pub title: String,
    pub description: String,
    pub spec_yaml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentTemplateLibrary {
    pub generated_at: String,
    pub templates: Vec<IntentTemplate>,
}

pub fn list_intent_templates() -> IntentTemplateLibrary {
    let templates = vec![
        template(
            "cost-api",
            "cost-optimized",
            "Cost-optimized API",
            "Stateless API tuned for FinOps",
        ),
        template(
            "latency-edge",
            "low-latency",
            "Low-latency edge",
            "Performance-first with tight SLA",
        ),
        template(
            "ha-data",
            "balanced",
            "HA data tier",
            "Standard resilience with encryption",
        ),
        template(
            "throughput-worker",
            "high-throughput",
            "Throughput worker",
            "Batch/queue worker profile",
        ),
    ];
    IntentTemplateLibrary {
        generated_at: crate::resources::now_rfc3339(),
        templates,
    }
}

fn template(id: &str, goal: &str, title: &str, description: &str) -> IntentTemplate {
    let spec_yaml = format!(
        r#"apiVersion: aether/v1
kind: Workload
metadata:
  name: {id}
  owner: platform
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 500m
  memory: 512Mi
  storage: 1Gi
intent:
  goal: {goal}
runtime:
  preferred: auto
  allow:
    - kube
    - container
"#
    );
    IntentTemplate {
        id: id.into(),
        goal: goal.into(),
        title: title.into(),
        description: description.into(),
        spec_yaml,
    }
}

// ── Phase 22: Multi-workload bundles ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentBundleRequest {
    pub bundle_name: String,
    pub goal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentBundleWorkload {
    pub role: String,
    pub workload_name: String,
    pub spec_yaml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentBundleReport {
    pub generated_at: String,
    pub bundle_name: String,
    pub shared_intent_yaml: String,
    pub workloads: Vec<IntentBundleWorkload>,
}

pub fn build_intent_bundle(req: &IntentBundleRequest) -> anyhow::Result<IntentBundleReport> {
    let goal = req.goal.as_deref().unwrap_or("balanced");
    let base = req.bundle_name.trim();
    let name = if base.is_empty() { "stack" } else { base };

    let shared_intent =
        format!("goal: {goal}\nsla:\n  minAvailabilityPct: 99.9\nbudget:\n  maxMonthlyUsd: 2000");

    let roles = [
        ("app", format!("{name}-app"), "1", "1Gi"),
        ("db", format!("{name}-db"), "2", "4Gi"),
        ("cache", format!("{name}-cache"), "500m", "512Mi"),
    ];

    let workloads = roles
        .iter()
        .map(|(role, wl_name, cpu, mem)| {
            let spec_yaml = format!(
                r#"apiVersion: aether/v1
kind: Workload
metadata:
  name: {wl_name}
  owner: platform
  project: {name}
requirements:
  cpu: {cpu}
  memory: {mem}
  storage: 10Gi
intent:
  goal: {goal}
runtime:
  preferred: auto
"#
            );
            IntentBundleWorkload {
                role: (*role).into(),
                workload_name: wl_name.clone(),
                spec_yaml,
            }
        })
        .collect();

    Ok(IntentBundleReport {
        generated_at: crate::resources::now_rfc3339(),
        bundle_name: name.into(),
        shared_intent_yaml: shared_intent,
        workloads,
    })
}

// ── Phase 23: GitOps intent diff ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentGitOpsDiffEntry {
    pub workload: String,
    pub has_drift: bool,
    pub live_intent_yaml: String,
    pub gitops_intent_yaml: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentGitOpsDiffReport {
    pub generated_at: String,
    pub entries: Vec<IntentGitOpsDiffEntry>,
}

pub fn build_intent_gitops_diff(state_path: &Path) -> anyhow::Result<IntentGitOpsDiffReport> {
    let store = StateStore::load(state_path)?;
    let fleet = scan_fleet(&store);
    let versions = load_version_store()?;
    let mut entries = Vec::new();

    for ws in store.list() {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        if spec.intent.is_none() {
            continue;
        }
        let live = intent_block_yaml(&spec);
        let drifted = fleet
            .rows
            .iter()
            .any(|r| r.workload == ws.name && r.has_drift);
        let gitops_yaml = versions
            .get(&ws.name)
            .and_then(|hist| hist.first())
            .map(|v| v.intent_yaml.clone());
        let summary = if drifted {
            "Fleet drift detected — intent block may differ from GitOps source".into()
        } else if gitops_yaml.as_ref() == Some(&live) {
            "Intent in sync with last recorded version".into()
        } else {
            "Intent changed since last snapshot".into()
        };
        entries.push(IntentGitOpsDiffEntry {
            workload: ws.name.clone(),
            has_drift: drifted,
            live_intent_yaml: live,
            gitops_intent_yaml: gitops_yaml,
            summary,
        });
    }

    Ok(IntentGitOpsDiffReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
    })
}

// ── Phase 24: Intent versioning ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentVersionEntry {
    pub version_id: String,
    pub recorded_at: String,
    pub intent_yaml: String,
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentVersionHistory {
    pub workload: String,
    pub versions: Vec<IntentVersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRollbackRequest {
    pub version_id: String,
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRollbackReport {
    pub dry_run: bool,
    pub workload: String,
    pub restored_intent_yaml: String,
    pub message: String,
}

type VersionStore = HashMap<String, Vec<IntentVersionEntry>>;

fn versions_path() -> PathBuf {
    crate::resources::aether_path("intent-versions.json")
}

fn load_version_store() -> anyhow::Result<VersionStore> {
    crate::resources::json_load(&versions_path()).or_else(|_| Ok(HashMap::new()))
}

fn save_version_store(store: &VersionStore) -> anyhow::Result<()> {
    crate::resources::json_save(store, &versions_path())
}

pub fn record_intent_version(workload: &str, spec: &Workload) -> anyhow::Result<()> {
    let Some(intent) = spec.intent.as_ref() else {
        return Ok(());
    };
    let mut store = load_version_store()?;
    let entry = IntentVersionEntry {
        version_id: format!("v{}", store.get(workload).map(|v| v.len()).unwrap_or(0) + 1),
        recorded_at: crate::resources::now_rfc3339(),
        intent_yaml: serde_yaml::to_string(intent)?,
        goal: goal_label(&intent.goal).into(),
    };
    store.entry(workload.into()).or_default().insert(0, entry);
    if let Some(list) = store.get_mut(workload) {
        list.truncate(10);
    }
    save_version_store(&store)
}

pub fn list_intent_versions(workload: &str) -> anyhow::Result<IntentVersionHistory> {
    let store = load_version_store()?;
    Ok(IntentVersionHistory {
        workload: workload.into(),
        versions: store.get(workload).cloned().unwrap_or_default(),
    })
}

pub fn rollback_intent_version(
    state_path: &Path,
    workload: &str,
    req: &IntentRollbackRequest,
) -> anyhow::Result<IntentRollbackReport> {
    let store = load_version_store()?;
    let Some(versions) = store.get(workload) else {
        anyhow::bail!("no intent versions for {workload}");
    };
    let Some(version) = versions.iter().find(|v| v.version_id == req.version_id) else {
        anyhow::bail!("version {} not found", req.version_id);
    };

    if req.dry_run {
        return Ok(IntentRollbackReport {
            dry_run: true,
            workload: workload.into(),
            restored_intent_yaml: version.intent_yaml.clone(),
            message: format!("dry-run: would restore {}", req.version_id),
        });
    }

    let state = StateStore::load(state_path)?;
    let Some(ws) = state.get(workload) else {
        anyhow::bail!("workload {workload} not in state");
    };
    let mut spec = Workload::from_file(&ws.spec_path)?;
    spec.intent = Some(serde_yaml::from_str(&version.intent_yaml)?);
    spec.validate()?;
    let yaml = serde_yaml::to_string(&spec)?;
    let tmp = ws.spec_path.with_extension("yaml.tmp");
    std::fs::write(&tmp, yaml.as_bytes())?;
    std::fs::rename(&tmp, &ws.spec_path)?;

    Ok(IntentRollbackReport {
        dry_run: false,
        workload: workload.into(),
        restored_intent_yaml: version.intent_yaml.clone(),
        message: format!("restored intent {} for {workload}", req.version_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nl_parses_cost_intent() {
        let report = parse_nl_intent(&NlIntentRequest {
            text: "I need a cost optimized API under $300 per month".into(),
            workload_name: Some("api".into()),
        })
        .unwrap();
        assert!(report.intent_yaml.contains("cost") || report.parsed_goal.contains("cost"));
    }

    #[test]
    fn templates_non_empty() {
        assert!(!list_intent_templates().templates.is_empty());
    }

    #[test]
    fn bundle_produces_three_workloads() {
        let bundle = build_intent_bundle(&IntentBundleRequest {
            bundle_name: "shop".into(),
            goal: Some("balanced".into()),
        })
        .unwrap();
        assert_eq!(bundle.workloads.len(), 3);
    }
}
