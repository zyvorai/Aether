//! SLA engine
//!
//! Define uptime and performance targets for workloads, track compliance,
//! and generate SLA reports.

use crate::output;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SLA target definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaTarget {
    pub workload: String,
    pub uptime_target_pct: f64,
    pub max_latency_ms: Option<f64>,
    pub max_error_rate_pct: Option<f64>,
    pub max_restarts_per_day: Option<u32>,
}

impl SlaTarget {
    /// Create a standard SLA target (99.9% uptime)
    pub fn standard(workload: &str) -> Self {
        Self {
            workload: workload.to_string(),
            uptime_target_pct: 99.9,
            max_latency_ms: Some(500.0),
            max_error_rate_pct: Some(1.0),
            max_restarts_per_day: Some(3),
        }
    }

    /// Create a high-availability SLA target (99.99% uptime)
    pub fn high_availability(workload: &str) -> Self {
        Self {
            workload: workload.to_string(),
            uptime_target_pct: 99.99,
            max_latency_ms: Some(200.0),
            max_error_rate_pct: Some(0.1),
            max_restarts_per_day: Some(1),
        }
    }

    /// Create a best-effort SLA target (99% uptime)
    pub fn best_effort(workload: &str) -> Self {
        Self {
            workload: workload.to_string(),
            uptime_target_pct: 99.0,
            max_latency_ms: Some(2000.0),
            max_error_rate_pct: Some(5.0),
            max_restarts_per_day: Some(10),
        }
    }
}

/// Observed metrics for SLA evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaObservation {
    pub uptime_pct: f64,
    pub avg_latency_ms: f64,
    pub error_rate_pct: f64,
    pub restarts: u32,
    pub observation_period: String,
}

/// SLA compliance status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceStatus {
    /// Meeting all SLA targets
    Compliant,
    /// At risk of violating SLA
    AtRisk,
    /// Currently violating SLA
    Violated,
}

impl std::fmt::Display for ComplianceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceStatus::Compliant => write!(f, "Compliant"),
            ComplianceStatus::AtRisk => write!(f, "At Risk"),
            ComplianceStatus::Violated => write!(f, "VIOLATED"),
        }
    }
}

/// SLA evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaReport {
    pub workload: String,
    pub status: ComplianceStatus,
    pub checks: Vec<SlaCheck>,
    pub remaining_error_budget: Option<ErrorBudget>,
    pub recommendations: Vec<String>,
}

/// Individual SLA check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaCheck {
    pub metric: String,
    pub target: String,
    pub actual: String,
    pub passed: bool,
    pub margin_pct: f64,
}

/// Error budget tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBudget {
    /// Total allowed downtime in minutes per month
    pub total_minutes: f64,
    /// Downtime consumed so far
    pub consumed_minutes: f64,
    /// Remaining budget in minutes
    pub remaining_minutes: f64,
    /// Percentage of budget consumed
    pub consumed_pct: f64,
    /// Projected budget exhaustion (days from now, None if on track)
    pub projected_exhaustion_days: Option<f64>,
}

/// SLA engine
#[derive(Default)]
pub struct SlaEngine {
    targets: HashMap<String, SlaTarget>,
}

impl SlaEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an SLA target
    pub fn add_target(&mut self, target: SlaTarget) {
        self.targets.insert(target.workload.clone(), target);
    }

    /// Get target for a workload
    pub fn get_target(&self, workload: &str) -> Option<&SlaTarget> {
        self.targets.get(workload)
    }

    /// List all targets
    pub fn list_targets(&self) -> Vec<&SlaTarget> {
        self.targets.values().collect()
    }

    /// Evaluate SLA compliance
    pub fn evaluate(
        &self,
        workload: &str,
        observation: &SlaObservation,
    ) -> Option<SlaReport> {
        let target = self.targets.get(workload)?;
        let mut checks = Vec::new();
        let mut all_passed = true;
        let mut any_at_risk = false;

        // Uptime check
        let uptime_passed = observation.uptime_pct >= target.uptime_target_pct;
        let uptime_margin = observation.uptime_pct - target.uptime_target_pct;
        if !uptime_passed {
            all_passed = false;
        } else if uptime_margin < 0.1 {
            any_at_risk = true;
        }
        checks.push(SlaCheck {
            metric: "Uptime".to_string(),
            target: format!("{:.2}%", target.uptime_target_pct),
            actual: format!("{:.2}%", observation.uptime_pct),
            passed: uptime_passed,
            margin_pct: uptime_margin,
        });

        // Latency check
        if let Some(max_latency) = target.max_latency_ms {
            let latency_passed = observation.avg_latency_ms <= max_latency;
            let margin = ((max_latency - observation.avg_latency_ms) / max_latency) * 100.0;
            if !latency_passed {
                all_passed = false;
            } else if margin < 10.0 {
                any_at_risk = true;
            }
            checks.push(SlaCheck {
                metric: "Latency".to_string(),
                target: format!("{:.0}ms", max_latency),
                actual: format!("{:.0}ms", observation.avg_latency_ms),
                passed: latency_passed,
                margin_pct: margin,
            });
        }

        // Error rate check
        if let Some(max_error_rate) = target.max_error_rate_pct {
            let error_passed = observation.error_rate_pct <= max_error_rate;
            let margin = if max_error_rate > 0.0 {
                ((max_error_rate - observation.error_rate_pct) / max_error_rate) * 100.0
            } else if observation.error_rate_pct == 0.0 {
                100.0 // both zero — perfect
            } else {
                0.0 // target is zero but actual > 0 — no margin remaining
            };
            if !error_passed {
                all_passed = false;
            } else if margin < 20.0 {
                any_at_risk = true;
            }
            checks.push(SlaCheck {
                metric: "Error Rate".to_string(),
                target: format!("{:.2}%", max_error_rate),
                actual: format!("{:.2}%", observation.error_rate_pct),
                passed: error_passed,
                margin_pct: margin,
            });
        }

        // Restart check
        if let Some(max_restarts) = target.max_restarts_per_day {
            let restart_passed = observation.restarts <= max_restarts;
            let margin = if max_restarts > 0 {
                ((max_restarts as f64 - observation.restarts as f64) / max_restarts as f64) * 100.0
            } else if observation.restarts == 0 {
                100.0 // both zero — perfect
            } else {
                0.0 // target is zero but actual > 0 — no margin remaining
            };
            if !restart_passed {
                all_passed = false;
            }
            checks.push(SlaCheck {
                metric: "Restarts/Day".to_string(),
                target: format!("{}", max_restarts),
                actual: format!("{}", observation.restarts),
                passed: restart_passed,
                margin_pct: margin,
            });
        }

        let status = if !all_passed {
            ComplianceStatus::Violated
        } else if any_at_risk {
            ComplianceStatus::AtRisk
        } else {
            ComplianceStatus::Compliant
        };

        // Error budget
        let remaining_error_budget = self.calculate_error_budget(target, observation);

        // Recommendations
        let recommendations = self.generate_recommendations(&checks, &status);

        Some(SlaReport {
            workload: workload.to_string(),
            status,
            checks,
            remaining_error_budget: Some(remaining_error_budget),
            recommendations,
        })
    }

    fn calculate_error_budget(
        &self,
        target: &SlaTarget,
        observation: &SlaObservation,
    ) -> ErrorBudget {
        // Calculate monthly error budget in minutes
        let minutes_per_month = 30.0 * 24.0 * 60.0; // 43200
        let allowed_downtime_pct = 100.0 - target.uptime_target_pct;
        let total_budget_minutes = minutes_per_month * (allowed_downtime_pct / 100.0);

        let actual_downtime_pct = 100.0 - observation.uptime_pct;
        let consumed_minutes = minutes_per_month * (actual_downtime_pct / 100.0);

        let remaining = (total_budget_minutes - consumed_minutes).max(0.0);
        let consumed_pct = if total_budget_minutes > 0.0 {
            (consumed_minutes / total_budget_minutes) * 100.0
        } else {
            0.0
        };

        // Project when budget will be exhausted
        let projected_exhaustion = if consumed_minutes > f64::EPSILON && remaining > 0.0 {
            // Extrapolate based on current burn rate (simplified: assume observation is for 1 day)
            let daily_burn = consumed_minutes;
            Some(remaining / daily_burn)
        } else if remaining <= 0.0 {
            Some(0.0)
        } else {
            None
        };

        ErrorBudget {
            total_minutes: total_budget_minutes,
            consumed_minutes,
            remaining_minutes: remaining,
            consumed_pct,
            projected_exhaustion_days: projected_exhaustion,
        }
    }

    fn generate_recommendations(
        &self,
        checks: &[SlaCheck],
        status: &ComplianceStatus,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        for check in checks {
            if !check.passed {
                match check.metric.as_str() {
                    "Uptime" => {
                        recs.push("Enable health probes and auto-restart for higher uptime".to_string());
                        recs.push("Consider adding replica scaling for redundancy".to_string());
                    }
                    "Latency" => {
                        recs.push("Profile application for performance bottlenecks".to_string());
                        recs.push("Consider adding caching layer or CDN".to_string());
                    }
                    "Error Rate" => {
                        recs.push("Analyze error logs to identify root causes".to_string());
                        recs.push("Implement circuit breaker patterns".to_string());
                    }
                    "Restarts/Day" => {
                        recs.push("Investigate OOM kills or crash loops".to_string());
                        recs.push("Review resource limits and liveness probe timing".to_string());
                    }
                    _ => {}
                }
            } else if check.margin_pct < 15.0 {
                recs.push(format!(
                    "{} is within {:.0}% of SLA limit - monitor closely",
                    check.metric, check.margin_pct
                ));
            }
        }

        if *status == ComplianceStatus::Compliant && recs.is_empty() {
            recs.push("All SLA targets met. No action required.".to_string());
        }

        recs
    }
}

/// Build an SLA observation from health history data
pub fn observation_from_health(
    history: &crate::health::HealthHistory,
    workload: &str,
) -> SlaObservation {
    let uptime = history.uptime_percent(workload);
    let restarts = history.restart_count(workload);
    SlaObservation {
        uptime_pct: uptime,
        avg_latency_ms: 100.0, // Default when no latency data available
        error_rate_pct: if uptime > 0.0 { 100.0 - uptime } else { 0.0 },
        restarts,
        observation_period: "auto".to_string(),
    }
}

/// Format SLA report
pub fn format_sla_report(report: &SlaReport) -> String {
    let mut output = String::new();

    output.push_str(&output::property_section(&[
        ("SLA Report", report.workload.clone()),
        ("Status", format!("{}", report.status)),
    ]));

    output.push_str(&format!(
        "{:<14} {:>10} {:>10} {:>8} {:>8}\n",
        "Metric", "Target", "Actual", "Status", "Margin"
    ));
    output.push_str(&"-".repeat(56));
    output.push('\n');

    for check in &report.checks {
        output.push_str(&format!(
            "{:<14} {:>10} {:>10} {:>8} {:>7.0}%\n",
            check.metric,
            check.target,
            check.actual,
            if check.passed { "PASS" } else { "FAIL" },
            check.margin_pct,
        ));
    }

    if let Some(budget) = &report.remaining_error_budget {
        let mut budget_pairs = vec![
            ("Error Budget", format!("{:.1} min remaining of {:.1} min ({:.0}% consumed)", budget.remaining_minutes, budget.total_minutes, budget.consumed_pct)),
        ];
        if let Some(days) = budget.projected_exhaustion_days {
            if days <= 0.0 {
                budget_pairs.push(("Projected exhaustion", "EXHAUSTED".to_string()));
            } else {
                budget_pairs.push(("Projected exhaustion", format!("{:.0} days", days)));
            }
        }
        output.push_str(&output::property_section(&budget_pairs));
    }

    if !report.recommendations.is_empty() {
        output.push_str("\nRecommendations:\n");
        for rec in &report.recommendations {
            output.push_str(&output::tree_bullet("💡", rec));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sla_compliant() {
        let mut engine = SlaEngine::new();
        engine.add_target(SlaTarget::best_effort("web-app")); // 99.0% target

        let observation = SlaObservation {
            uptime_pct: 99.5,       // 0.5% margin above 99.0% target
            avg_latency_ms: 500.0,  // well under 2000ms limit
            error_rate_pct: 1.0,    // well under 5.0% limit
            restarts: 2,            // under 10 limit
            observation_period: "24h".to_string(),
        };

        let report = engine.evaluate("web-app", &observation).unwrap();
        assert_eq!(report.status, ComplianceStatus::Compliant);
        assert!(report.checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_sla_violated() {
        let mut engine = SlaEngine::new();
        engine.add_target(SlaTarget::standard("web-app"));

        let observation = SlaObservation {
            uptime_pct: 98.0,
            avg_latency_ms: 800.0,
            error_rate_pct: 5.0,
            restarts: 10,
            observation_period: "24h".to_string(),
        };

        let report = engine.evaluate("web-app", &observation).unwrap();
        assert_eq!(report.status, ComplianceStatus::Violated);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_sla_at_risk() {
        let mut engine = SlaEngine::new();
        engine.add_target(SlaTarget::standard("web-app"));

        let observation = SlaObservation {
            uptime_pct: 99.92, // Just above 99.9 target
            avg_latency_ms: 100.0,
            error_rate_pct: 0.1,
            restarts: 0,
            observation_period: "24h".to_string(),
        };

        let report = engine.evaluate("web-app", &observation).unwrap();
        assert_eq!(report.status, ComplianceStatus::AtRisk);
    }

    #[test]
    fn test_error_budget() {
        let mut engine = SlaEngine::new();
        engine.add_target(SlaTarget::standard("web-app")); // 99.9% = 43.2 min/month

        let observation = SlaObservation {
            uptime_pct: 99.95,
            avg_latency_ms: 100.0,
            error_rate_pct: 0.1,
            restarts: 0,
            observation_period: "24h".to_string(),
        };

        let report = engine.evaluate("web-app", &observation).unwrap();
        let budget = report.remaining_error_budget.unwrap();
        assert!(budget.total_minutes > 0.0);
        assert!(budget.remaining_minutes > 0.0);
    }

    #[test]
    fn test_high_availability_target() {
        let target = SlaTarget::high_availability("critical-app");
        assert_eq!(target.uptime_target_pct, 99.99);
        assert!(target.max_latency_ms.unwrap() < 300.0);
    }

    #[test]
    fn test_format_report() {
        let mut engine = SlaEngine::new();
        engine.add_target(SlaTarget::standard("web-app"));
        let obs = SlaObservation {
            uptime_pct: 99.95,
            avg_latency_ms: 100.0,
            error_rate_pct: 0.1,
            restarts: 0,
            observation_period: "24h".to_string(),
        };
        let report = engine.evaluate("web-app", &obs).unwrap();
        let formatted = format_sla_report(&report);
        assert!(formatted.contains("SLA Report"));
        assert!(formatted.contains("Error Budget"));
    }
}
