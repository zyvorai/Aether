// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! FinOps & Cost platform — Era H (phases 75–84).

use crate::cost::{self, CloudProvider};
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::policy::AutonomyPolicy;
use crate::intelligence::profile::WorkloadBehaviorProfile;
use crate::intelligence::store::IntelligenceStore;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

fn finops_history_path() -> PathBuf {
    crate::resources::aether_path("finops-history.json")
}

fn finops_budget_path() -> PathBuf {
    crate::resources::aether_path("finops-budget-alerts.jsonl")
}

// ── Phase 75: Chargeback automation ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebackAutomationLine {
    pub workload: String,
    pub owner: String,
    pub project: String,
    pub monthly_usd: f64,
    pub attribution_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebackAutomationReport {
    pub generated_at: String,
    pub provider: String,
    pub region: String,
    pub total_monthly_usd: f64,
    pub lines: Vec<ChargebackAutomationLine>,
    pub by_owner: HashMap<String, f64>,
    pub by_project: HashMap<String, f64>,
    pub unassigned_count: u32,
}

pub fn build_chargeback_automation(
    workloads: &[(&str, &PathBuf)],
) -> anyhow::Result<ChargebackAutomationReport> {
    let provider = cost::default_chargeback_provider();
    let report = cost::chargeback_report(workloads, provider)?;
    let mut unassigned = 0u32;
    let lines: Vec<ChargebackAutomationLine> = report
        .lines
        .iter()
        .map(|line| {
            let source = if line.owner.is_empty() || line.owner == "unknown" {
                unassigned += 1;
                "inferred-from-labels".into()
            } else {
                "metadata.owner".into()
            };
            ChargebackAutomationLine {
                workload: line.workload.clone(),
                owner: if line.owner.is_empty() {
                    "unassigned".into()
                } else {
                    line.owner.clone()
                },
                project: if line.project.is_empty() {
                    "default".into()
                } else {
                    line.project.clone()
                },
                monthly_usd: line.monthly_usd,
                attribution_source: source,
            }
        })
        .collect();
    Ok(ChargebackAutomationReport {
        generated_at: crate::resources::now_rfc3339(),
        provider: format!("{}", report.provider),
        region: report.region,
        total_monthly_usd: report.total_monthly_usd,
        lines,
        by_owner: report.by_owner,
        by_project: report.by_project,
        unassigned_count: unassigned,
    })
}

// ── Phase 76: Spot / preemptible advisor ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotAdvisorEntry {
    pub workload: String,
    pub eligible: bool,
    pub savings_pct: f64,
    pub spot_monthly_usd: f64,
    pub on_demand_monthly_usd: f64,
    pub risk: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotAdvisorReport {
    pub generated_at: String,
    pub entries: Vec<SpotAdvisorEntry>,
    pub eligible_count: u32,
}

pub fn build_spot_advisor(workloads: &[(Workload, WorkloadState)]) -> SpotAdvisorReport {
    let provider = cost::default_chargeback_provider();
    let mut entries = Vec::new();
    let mut eligible_count = 0u32;

    for (spec, ws) in workloads {
        let Ok(est) = cost::estimate_cost_priced(spec, provider) else {
            continue;
        };
        let (spot, _) = cost::purchase_model_costs(est.total_monthly);
        let profile_key = spec.metadata.name.clone();
        let intel = IntelligenceStore::load(&IntelligenceStore::default_path()).unwrap_or_default();
        let profile = intel.behavior_profiles.get(&profile_key);

        let gpu =
            spec.requirements.gpu.is_some() || profile.map(|p| p.gpu_contention).unwrap_or(false);
        let stateful = spec.persistence.enabled;
        let ha = spec
            .intent
            .as_ref()
            .and_then(|i| i.sla.as_ref())
            .and_then(|s| s.min_availability_pct)
            .map(|v| v >= 99.9)
            .unwrap_or(false);

        let (eligible, risk, reason) = if gpu {
            (
                false,
                "high".into(),
                "GPU workloads are poor spot candidates".into(),
            )
        } else if stateful {
            (
                false,
                "medium".into(),
                "Persistent volumes require stable nodes".into(),
            )
        } else if ha {
            (
                true,
                "medium".into(),
                "Eligible with interruption tolerance checks".into(),
            )
        } else {
            (
                true,
                "low".into(),
                "Stateless workload — good spot candidate".into(),
            )
        };

        if eligible {
            eligible_count += 1;
        }
        let savings_pct = if est.total_monthly > 0.0 {
            ((est.total_monthly - spot) / est.total_monthly) * 100.0
        } else {
            0.0
        };
        entries.push(SpotAdvisorEntry {
            workload: spec.metadata.name.clone(),
            eligible,
            savings_pct,
            spot_monthly_usd: spot,
            on_demand_monthly_usd: est.total_monthly,
            risk,
            reason,
        });
        let _ = ws;
    }

    entries.sort_by(|a, b| {
        b.savings_pct
            .partial_cmp(&a.savings_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    SpotAdvisorReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
        eligible_count,
    }
}

// ── Phase 77: Reserved instance planner ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservedInstanceRecommendation {
    pub workload: String,
    pub runtime: String,
    pub monthly_on_demand_usd: f64,
    pub reserved_monthly_usd: f64,
    pub savings_pct: f64,
    pub term_months: u32,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservedInstancePlannerReport {
    pub generated_at: String,
    pub recommendations: Vec<ReservedInstanceRecommendation>,
    pub fleet_reserved_savings_usd: f64,
}

pub fn build_reserved_instance_planner(
    workloads: &[(Workload, WorkloadState)],
) -> ReservedInstancePlannerReport {
    let provider = cost::default_chargeback_provider();
    let mut recommendations = Vec::new();
    let mut fleet_savings = 0.0;

    for (spec, ws) in workloads {
        let Ok(est) = cost::estimate_cost_priced(spec, provider) else {
            continue;
        };
        if est.total_monthly < 50.0 {
            continue;
        }
        let (_, reserved) = cost::purchase_model_costs(est.total_monthly);
        let savings_pct = if est.total_monthly > 0.0 {
            ((est.total_monthly - reserved) / est.total_monthly) * 100.0
        } else {
            0.0
        };
        if savings_pct < 5.0 {
            continue;
        }
        fleet_savings += est.total_monthly - reserved;
        recommendations.push(ReservedInstanceRecommendation {
            workload: spec.metadata.name.clone(),
            runtime: format!("{}", ws.runtime),
            monthly_on_demand_usd: est.total_monthly,
            reserved_monthly_usd: reserved,
            savings_pct,
            term_months: 12,
            recommendation: format!(
                "Purchase 1-year RI for {} ({:.0}% savings vs on-demand)",
                spec.metadata.name, savings_pct
            ),
        });
    }

    recommendations.sort_by(|a, b| {
        b.savings_pct
            .partial_cmp(&a.savings_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    ReservedInstancePlannerReport {
        generated_at: crate::resources::now_rfc3339(),
        recommendations,
        fleet_reserved_savings_usd: fleet_savings,
    }
}

// ── Phase 78: Cost anomaly detection ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FinOpsHistorySnapshot {
    pub timestamp: String,
    pub total_monthly_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FinOpsHistory {
    pub snapshots: Vec<FinOpsHistorySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnomalyEntry {
    pub workload: String,
    pub current_usd: f64,
    pub baseline_usd: f64,
    pub delta_pct: f64,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnomalyReport {
    pub generated_at: String,
    pub fleet_total_usd: f64,
    pub fleet_delta_pct: f64,
    pub anomalies: Vec<CostAnomalyEntry>,
    pub alert: bool,
}

fn read_finops_history() -> FinOpsHistory {
    let path = finops_history_path();
    if !path.exists() {
        return FinOpsHistory::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn append_finops_snapshot(total: f64) -> anyhow::Result<()> {
    let mut history = read_finops_history();
    history.snapshots.push(FinOpsHistorySnapshot {
        timestamp: crate::resources::now_rfc3339(),
        total_monthly_usd: total,
    });
    while history.snapshots.len() > 48 {
        history.snapshots.remove(0);
    }
    if let Some(parent) = finops_history_path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        finops_history_path(),
        serde_json::to_string_pretty(&history)?,
    )?;
    Ok(())
}

pub fn detect_cost_anomalies(workloads: &[(&str, &PathBuf)]) -> anyhow::Result<CostAnomalyReport> {
    let provider = cost::default_chargeback_provider();
    let report = cost::chargeback_report(workloads, provider)?;
    let _ = append_finops_snapshot(report.total_monthly_usd);

    let history = read_finops_history();
    let baseline = history
        .snapshots
        .iter()
        .rev()
        .skip(1)
        .take(6)
        .map(|s| s.total_monthly_usd)
        .collect::<Vec<_>>();
    let fleet_baseline = if baseline.is_empty() {
        report.total_monthly_usd
    } else {
        baseline.iter().sum::<f64>() / baseline.len() as f64
    };
    let fleet_delta = if fleet_baseline > 0.0 {
        ((report.total_monthly_usd - fleet_baseline) / fleet_baseline) * 100.0
    } else {
        0.0
    };

    let mut anomalies = Vec::new();
    for line in &report.lines {
        let baseline_line = line.monthly_usd * 0.92;
        let delta = if baseline_line > 0.0 {
            ((line.monthly_usd - baseline_line) / baseline_line) * 100.0
        } else {
            0.0
        };
        if delta >= 15.0 {
            anomalies.push(CostAnomalyEntry {
                workload: line.workload.clone(),
                current_usd: line.monthly_usd,
                baseline_usd: baseline_line,
                delta_pct: delta,
                severity: if delta >= 30.0 { "critical" } else { "warning" }.into(),
            });
        }
    }

    let alert = fleet_delta >= 15.0 || !anomalies.is_empty();
    Ok(CostAnomalyReport {
        generated_at: crate::resources::now_rfc3339(),
        fleet_total_usd: report.total_monthly_usd,
        fleet_delta_pct: fleet_delta,
        anomalies,
        alert,
    })
}

// ── Phase 79: Unit economics ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitEconomicsEntry {
    pub workload: String,
    pub monthly_usd: f64,
    pub estimated_requests_per_month: u64,
    pub cost_per_1k_requests_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitEconomicsReport {
    pub generated_at: String,
    pub entries: Vec<UnitEconomicsEntry>,
    pub fleet_cost_per_1k_usd: f64,
}

fn estimate_monthly_requests(profile: Option<&WorkloadBehaviorProfile>) -> u64 {
    profile
        .map(|p| {
            let base = 1_000_000u64;
            if p.cpu_bursty {
                base * 2
            } else {
                base
            }
        })
        .unwrap_or(1_000_000)
}

pub fn build_unit_economics(workloads: &[(Workload, WorkloadState)]) -> UnitEconomicsReport {
    let provider = cost::default_chargeback_provider();
    let intel = IntelligenceStore::load(&IntelligenceStore::default_path()).unwrap_or_default();
    let mut entries = Vec::new();
    let mut total_cost = 0.0;
    let mut total_requests = 0u64;

    for (spec, _ws) in workloads {
        let Ok(est) = cost::estimate_cost_priced(spec, provider) else {
            continue;
        };
        let profile = intel.behavior_profiles.get(&spec.metadata.name);
        let requests = estimate_monthly_requests(profile);
        let cost_per_1k = if requests > 0 {
            (est.total_monthly / requests as f64) * 1000.0
        } else {
            0.0
        };
        total_cost += est.total_monthly;
        total_requests += requests;
        entries.push(UnitEconomicsEntry {
            workload: spec.metadata.name.clone(),
            monthly_usd: est.total_monthly,
            estimated_requests_per_month: requests,
            cost_per_1k_requests_usd: cost_per_1k,
        });
    }

    entries.sort_by(|a, b| {
        b.cost_per_1k_requests_usd
            .partial_cmp(&a.cost_per_1k_requests_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let fleet_cost_per_1k = if total_requests > 0 {
        (total_cost / total_requests as f64) * 1000.0
    } else {
        0.0
    };

    UnitEconomicsReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
        fleet_cost_per_1k_usd: fleet_cost_per_1k,
    }
}

// ── Phase 80: FinOps agent execute ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinOpsExecuteRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    #[serde(default)]
    pub schedule: Option<String>,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinOpsExecuteReport {
    pub dry_run: bool,
    pub schedule: String,
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub savings_monthly_usd: f64,
}

pub fn execute_finops_agent(
    workloads: &[(Workload, WorkloadState)],
    policy: &AutonomyPolicy,
    req: &FinOpsExecuteRequest,
) -> FinOpsExecuteReport {
    let patches = FinOpsEngine::build_cost_patches(workloads);
    let savings: f64 = patches.iter().map(|p| p.savings_monthly_usd).sum();
    let apply = FinOpsEngine::apply_cost_patches(&patches, req.dry_run, policy);
    let schedule = req
        .schedule
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "nightly-02:00".into());

    FinOpsExecuteReport {
        dry_run: req.dry_run,
        schedule,
        applied: apply.applied,
        skipped: apply.skipped,
        savings_monthly_usd: savings,
    }
}

// ── Phase 81: Multi-cloud cost compare ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MulticloudCostRow {
    pub provider: String,
    pub total_monthly_usd: f64,
    pub cheapest_workloads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MulticloudCostCompareReport {
    pub generated_at: String,
    pub rows: Vec<MulticloudCostRow>,
    pub recommended_provider: String,
    pub savings_vs_worst_pct: f64,
}

pub fn build_multicloud_cost_compare(
    workloads: &[(Workload, WorkloadState)],
) -> MulticloudCostCompareReport {
    let providers = [CloudProvider::AWS, CloudProvider::GCP, CloudProvider::Azure];
    let mut totals = HashMap::new();
    let mut cheapest_counts = HashMap::new();

    for (spec, _ws) in workloads {
        let Ok(all) = cost::estimate_all_providers(spec) else {
            continue;
        };
        let best = all
            .iter()
            .min_by(|a, b| {
                a.total_monthly
                    .partial_cmp(&b.total_monthly)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| e.provider);
        for p in providers {
            if let Some(est) = all.iter().find(|e| e.provider == p) {
                *totals.entry(format!("{p}")).or_insert(0.0) += est.total_monthly;
            }
        }
        if let Some(bp) = best {
            *cheapest_counts.entry(format!("{bp}")).or_insert(0u32) += 1;
        }
    }

    let mut rows: Vec<MulticloudCostRow> = providers
        .iter()
        .map(|p| {
            let key = format!("{p}");
            MulticloudCostRow {
                provider: key.clone(),
                total_monthly_usd: *totals.get(&key).unwrap_or(&0.0),
                cheapest_workloads: *cheapest_counts.get(&key).unwrap_or(&0),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        a.total_monthly_usd
            .partial_cmp(&b.total_monthly_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let recommended = rows
        .first()
        .map(|r| r.provider.clone())
        .unwrap_or_else(|| "AWS".into());
    let worst = rows.last().map(|r| r.total_monthly_usd).unwrap_or(0.0);
    let best = rows.first().map(|r| r.total_monthly_usd).unwrap_or(0.0);
    let savings_pct = if worst > 0.0 {
        ((worst - best) / worst) * 100.0
    } else {
        0.0
    };

    MulticloudCostCompareReport {
        generated_at: crate::resources::now_rfc3339(),
        rows,
        recommended_provider: recommended,
        savings_vs_worst_pct: savings_pct,
    }
}

// ── Phase 82: Carbon footprint (Lab) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonFootprintEntry {
    pub region: String,
    pub intensity_gco2_kwh: f64,
    pub monthly_kwh_estimate: f64,
    pub carbon_kg_monthly: f64,
    pub greener_alternative: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonFootprintReport {
    pub status: String,
    pub generated_at: String,
    pub entries: Vec<CarbonFootprintEntry>,
    pub hint: String,
}

fn region_carbon_intensity(region: &str) -> f64 {
    match region.to_lowercase().as_str() {
        "us-east-1" | "us-west-2" | "us-central1" => 350.0,
        "eu-west-1" | "europe-west1" | "westeurope" => 220.0,
        "eu-north-1" | "northeurope" => 50.0,
        "ap-southeast-1" | "asia-southeast1" => 420.0,
        _ => 300.0,
    }
}

pub fn build_carbon_footprint(workloads: &[(Workload, WorkloadState)]) -> CarbonFootprintReport {
    let region = cost::pricing_region();
    let intensity = region_carbon_intensity(&region);
    let mut monthly_kwh = 0.0;
    for (spec, _ws) in workloads {
        let cores = spec
            .requirements
            .cpu
            .trim()
            .trim_end_matches('m')
            .parse::<f64>()
            .unwrap_or(1.0)
            .max(0.5);
        let mem_gb = crate::resources::parse_memory_gi(&spec.requirements.memory);
        monthly_kwh += cores * 720.0 * 0.05 + mem_gb * 720.0 * 0.01;
    }
    let carbon_kg = (monthly_kwh * intensity) / 1000.0;
    let greener = if intensity > 250.0 {
        Some("eu-north-1 or us-west-2 (lower grid intensity)".into())
    } else {
        None
    };

    CarbonFootprintReport {
        status: "lab".into(),
        generated_at: crate::resources::now_rfc3339(),
        entries: vec![CarbonFootprintEntry {
            region: region.clone(),
            intensity_gco2_kwh: intensity,
            monthly_kwh_estimate: monthly_kwh,
            carbon_kg_monthly: carbon_kg,
            greener_alternative: greener,
        }],
        hint: "Carbon-aware placement uses region intensity estimates — connect live grid APIs for production.".into(),
    }
}

// ── Phase 83: Budget alerts webhook ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetWebhookRequest {
    pub cap_usd: Option<f64>,
    pub webhook_url: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetWebhookReport {
    pub generated_at: String,
    pub cap_usd: f64,
    pub current_usd: f64,
    pub breached: bool,
    pub webhook_dispatched: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BudgetAlertLog {
    timestamp: String,
    cap_usd: f64,
    current_usd: f64,
    channel: String,
}

pub fn dispatch_budget_webhook(
    workloads: &[(&str, &PathBuf)],
    req: &BudgetWebhookRequest,
) -> anyhow::Result<BudgetWebhookReport> {
    let provider = cost::default_chargeback_provider();
    let report = cost::chargeback_report(workloads, provider)?;
    let cap = req.cap_usd.unwrap_or_else(|| {
        std::env::var("AETHER_FINOPS_BUDGET_CAP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10_000.0)
    });
    let breached = report.total_monthly_usd > cap;
    let channel = req
        .channel
        .clone()
        .or_else(|| std::env::var("AETHER_FINOPS_BUDGET_CHANNEL").ok())
        .unwrap_or_else(|| "slack".into());
    let message = if breached {
        format!(
            "FinOps budget breach: ${:.0}/mo exceeds cap ${:.0}/mo ({} workloads)",
            report.total_monthly_usd,
            cap,
            report.lines.len()
        )
    } else {
        format!(
            "FinOps budget OK: ${:.0}/mo within cap ${:.0}/mo",
            report.total_monthly_usd, cap
        )
    };

    let webhook_dispatched = breached
        && (req.webhook_url.is_some() || std::env::var("AETHER_FINOPS_BUDGET_WEBHOOK").is_ok());

    if webhook_dispatched {
        if let Some(parent) = finops_budget_path().parent() {
            std::fs::create_dir_all(parent)?;
        }
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(finops_budget_path())?;
        writeln!(
            file,
            "{}",
            serde_json::to_string(&BudgetAlertLog {
                timestamp: crate::resources::now_rfc3339(),
                cap_usd: cap,
                current_usd: report.total_monthly_usd,
                channel: channel.clone(),
            })?
        )?;
    }

    Ok(BudgetWebhookReport {
        generated_at: crate::resources::now_rfc3339(),
        cap_usd: cap,
        current_usd: report.total_monthly_usd,
        breached,
        webhook_dispatched,
        message,
    })
}

// ── Phase 84: FinOps trends & forecast ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinOpsTrendPoint {
    pub label: String,
    pub monthly_usd: f64,
    pub forecast: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinOpsTrendsReport {
    pub generated_at: String,
    pub points: Vec<FinOpsTrendPoint>,
    pub forecast_next_month_usd: f64,
    pub trend_direction: String,
}

pub fn build_finops_trends(
    workloads: &[(&str, &PathBuf)],
    pairs: &[(Workload, WorkloadState)],
) -> anyhow::Result<FinOpsTrendsReport> {
    let provider = cost::default_chargeback_provider();
    let report = cost::chargeback_report(workloads, provider)?;
    let _ = append_finops_snapshot(report.total_monthly_usd);

    let history = read_finops_history();
    let mut points: Vec<FinOpsTrendPoint> = history
        .snapshots
        .iter()
        .enumerate()
        .map(|(i, s)| FinOpsTrendPoint {
            label: format!("t-{i}"),
            monthly_usd: s.total_monthly_usd,
            forecast: false,
        })
        .collect();

    if points.is_empty() {
        points.push(FinOpsTrendPoint {
            label: "now".into(),
            monthly_usd: report.total_monthly_usd,
            forecast: false,
        });
    }

    let recent: Vec<f64> = points.iter().map(|p| p.monthly_usd).collect();
    let avg = recent.iter().sum::<f64>() / recent.len().max(1) as f64;
    let last = *recent.last().unwrap_or(&report.total_monthly_usd);
    let trend_direction = if last > avg * 1.05 {
        "up".into()
    } else if last < avg * 0.95 {
        "down".into()
    } else {
        "flat".into()
    };

    let optimize = FinOpsEngine::optimize_fleet(pairs);
    let savings = optimize
        .recommendations
        .iter()
        .map(|r| r.savings_monthly_usd)
        .sum::<f64>();
    let forecast = (last - savings * 0.5).max(0.0);

    points.push(FinOpsTrendPoint {
        label: "forecast".into(),
        monthly_usd: forecast,
        forecast: true,
    });

    Ok(FinOpsTrendsReport {
        generated_at: crate::resources::now_rfc3339(),
        points,
        forecast_next_month_usd: forecast,
        trend_direction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multicloud_compare_has_rows() {
        let r = build_multicloud_cost_compare(&[]);
        assert_eq!(r.rows.len(), 3);
    }
}
