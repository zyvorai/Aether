//! Intelligent migration advisor
//!
//! Recommends migration strategy based on workload characteristics,
//! provides adaptive canary analysis, and load-aware scheduling.

use crate::config::MigrationConfig;
use crate::migration::MigrationStrategy;
use crate::output;
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use serde::{Deserialize, Serialize};

/// Migration recommendation from the advisor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationAdvice {
    pub recommended_strategy: MigrationStrategy,
    pub estimated_downtime_secs: u64,
    pub risk_level: RiskLevel,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub suggested_timing: TimingAdvice,
    pub canary_config: CanaryConfig,
}

/// Risk level for migration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}

/// Timing advice for when to migrate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingAdvice {
    pub recommendation: String,
    pub preferred_window: String,
    pub avoid_times: Vec<String>,
}

/// Canary configuration derived from workload analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    /// Traffic percentages for each step
    pub steps: Vec<u32>,
    /// Seconds between steps
    pub step_interval_secs: u64,
    /// Error rate threshold to trigger rollback
    pub error_threshold: f64,
    /// Latency increase threshold (percentage) to trigger rollback
    pub latency_threshold_pct: f64,
    /// Minimum observation time per step in seconds
    pub min_observation_secs: u64,
}

/// Canary observation during migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryObservation {
    pub step: u32,
    pub traffic_pct: u32,
    pub error_rate: f64,
    pub latency_ms: f64,
    pub healthy: bool,
    pub timestamp: String,
}

/// Canary analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryAnalysis {
    pub should_continue: bool,
    pub should_rollback: bool,
    pub observations: Vec<CanaryObservation>,
    pub summary: String,
}

/// Migration advisor that recommends strategies
pub struct MigrationAdvisor {
    config: MigrationConfig,
}

impl MigrationAdvisor {
    pub fn new(config: MigrationConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(MigrationConfig::default())
    }

    /// Analyze workload and recommend migration strategy
    pub fn advise(
        &self,
        spec: &Workload,
        source: RuntimeKind,
        target: RuntimeKind,
    ) -> MigrationAdvice {
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        let is_stateful = spec.persistence.enabled;
        let has_health = spec.health.is_some();
        let has_scaling = spec.scaling.as_ref().is_some_and(|s| s.enabled);
        let has_ingress = spec.ingress.as_ref().is_some_and(|i| i.enabled);

        // Determine risk level
        let risk_level = self.assess_risk(spec, source, target, &mut warnings);

        // Recommend strategy based on workload characteristics
        let recommended_strategy = self.recommend_strategy(
            spec, source, target, &risk_level, &mut reasons,
        );

        // Estimate downtime
        let estimated_downtime_secs = self.estimate_downtime(&recommended_strategy, is_stateful);

        // Timing advice
        let suggested_timing = self.suggest_timing(has_ingress, has_scaling);

        // Canary configuration
        let canary_config = self.build_canary_config(&risk_level, has_health);

        if is_stateful {
            warnings.push("Stateful workload: ensure data migration plan is in place".to_string());
        }

        if !has_health {
            warnings.push("No health probes defined: validation will use basic connectivity checks".to_string());
        }

        MigrationAdvice {
            recommended_strategy,
            estimated_downtime_secs,
            risk_level,
            reasons,
            warnings,
            suggested_timing,
            canary_config,
        }
    }

    /// Assess migration risk
    fn assess_risk(
        &self,
        spec: &Workload,
        source: RuntimeKind,
        target: RuntimeKind,
        warnings: &mut Vec<String>,
    ) -> RiskLevel {
        let mut risk_score = 0u32;

        // Cross-category migration (e.g., container to bare metal) is higher risk
        let category_change = matches!(
            (source, target),
            (RuntimeKind::Podman, RuntimeKind::Metal3)
                | (RuntimeKind::Metal3, RuntimeKind::Podman)
                | (RuntimeKind::Podman, RuntimeKind::KubeVirt)
                | (RuntimeKind::KubeVirt, RuntimeKind::Podman)
        );
        if category_change {
            risk_score += 3;
            warnings.push("Cross-category migration: significant runtime differences".to_string());
        }

        // Stateful workloads are riskier to migrate
        if spec.persistence.enabled {
            risk_score += 2;
        }

        // GPU workloads require careful device mapping
        if spec.requirements.gpu.is_some() {
            risk_score += 2;
            warnings.push("GPU migration: verify device availability on target".to_string());
        }

        // No health probes means less validation capability
        if spec.health.is_none() {
            risk_score += 1;
        }

        let risk_score = risk_score.min(5);

        match risk_score {
            0..=1 => RiskLevel::Low,
            2..=3 => RiskLevel::Medium,
            // risk_score is clamped to 5 above, so 4..=5 covers all remaining cases
            _ => RiskLevel::High,
        }
    }

    /// Recommend migration strategy
    fn recommend_strategy(
        &self,
        spec: &Workload,
        _source: RuntimeKind,
        _target: RuntimeKind,
        risk_level: &RiskLevel,
        reasons: &mut Vec<String>,
    ) -> MigrationStrategy {
        let is_stateful = spec.persistence.enabled;
        let has_health = spec.health.is_some();
        let has_ingress = spec.ingress.as_ref().is_some_and(|i| i.enabled);

        match risk_level {
            RiskLevel::Critical | RiskLevel::High => {
                reasons.push("High risk: blue-green provides safest rollback path".to_string());
                MigrationStrategy::BlueGreen
            }
            RiskLevel::Medium => {
                if has_health && has_ingress {
                    reasons.push("Rolling migration with canary validation for medium-risk workload".to_string());
                    MigrationStrategy::Rolling
                } else {
                    reasons.push("Blue-green recommended: limited health observability".to_string());
                    MigrationStrategy::BlueGreen
                }
            }
            RiskLevel::Low => {
                if is_stateful {
                    reasons.push("Stateful but low risk: blue-green ensures data consistency".to_string());
                    MigrationStrategy::BlueGreen
                } else if has_health {
                    reasons.push("Low risk with health probes: rolling migration for zero downtime".to_string());
                    MigrationStrategy::Rolling
                } else {
                    reasons.push("Low risk, simple workload: immediate migration is sufficient".to_string());
                    MigrationStrategy::Immediate
                }
            }
        }
    }

    /// Estimate downtime for a migration strategy
    fn estimate_downtime(&self, strategy: &MigrationStrategy, is_stateful: bool) -> u64 {
        let base = match strategy {
            MigrationStrategy::Immediate => self.config.graceful_shutdown_secs + 30,
            MigrationStrategy::BlueGreen => 0, // Zero downtime in theory
            MigrationStrategy::Rolling => 0,   // Zero downtime in theory
            MigrationStrategy::Canary => 0,    // Zero downtime (canary runs alongside stable)
        };

        if is_stateful {
            base + 60 // Extra time for data sync
        } else {
            base
        }
    }

    /// Suggest timing for migration
    fn suggest_timing(&self, has_ingress: bool, has_scaling: bool) -> TimingAdvice {
        if has_ingress {
            TimingAdvice {
                recommendation: "Schedule during low-traffic window".to_string(),
                preferred_window: "Weekday 02:00-06:00 UTC (typical low traffic)".to_string(),
                avoid_times: vec![
                    "Peak business hours (09:00-17:00 local)".to_string(),
                    "End-of-month processing windows".to_string(),
                ],
            }
        } else if has_scaling {
            TimingAdvice {
                recommendation: "Migrate when current replica count is at minimum".to_string(),
                preferred_window: "Low-demand period based on scaling metrics".to_string(),
                avoid_times: vec!["During active scale-up events".to_string()],
            }
        } else {
            TimingAdvice {
                recommendation: "Internal service: can migrate during maintenance window".to_string(),
                preferred_window: "Any scheduled maintenance window".to_string(),
                avoid_times: vec!["During dependent service deployments".to_string()],
            }
        }
    }

    /// Validate that canary steps are in ascending order and the last step equals 100
    fn validate_canary_steps(steps: &[u32]) -> bool {
        if steps.is_empty() {
            return false;
        }
        if steps.last() != Some(&100) {
            return false;
        }
        steps.windows(2).all(|w| w[0] < w[1])
    }

    /// Build canary configuration based on risk
    fn build_canary_config(&self, risk_level: &RiskLevel, has_health: bool) -> CanaryConfig {
        let mut config = match risk_level {
            RiskLevel::Critical => CanaryConfig {
                steps: vec![5, 10, 25, 50, 75, 100],
                step_interval_secs: 120,
                error_threshold: 0.01,
                latency_threshold_pct: 10.0,
                min_observation_secs: 300,
            },
            RiskLevel::High => CanaryConfig {
                steps: vec![10, 25, 50, 75, 100],
                step_interval_secs: 60,
                error_threshold: 0.02,
                latency_threshold_pct: 20.0,
                min_observation_secs: 120,
            },
            RiskLevel::Medium => CanaryConfig {
                steps: self.config.rolling_steps.clone(),
                step_interval_secs: self.config.rolling_interval_secs,
                error_threshold: self.config.canary_error_threshold,
                latency_threshold_pct: 30.0,
                min_observation_secs: 60,
            },
            RiskLevel::Low => {
                if has_health {
                    CanaryConfig {
                        steps: vec![50, 100],
                        step_interval_secs: 15,
                        error_threshold: 0.10,
                        latency_threshold_pct: 50.0,
                        min_observation_secs: 30,
                    }
                } else {
                    CanaryConfig {
                        steps: vec![100],
                        step_interval_secs: 5,
                        error_threshold: 0.10,
                        latency_threshold_pct: 50.0,
                        min_observation_secs: 10,
                    }
                }
            }
        };

        if !Self::validate_canary_steps(&config.steps) {
            tracing::error!(
                "Invalid canary steps {:?}: must be ascending and end at 100; using default [25, 50, 75, 100]",
                config.steps
            );
            config.steps = vec![25, 50, 75, 100];
        }

        config
    }

    /// Analyze canary observations and decide whether to proceed
    pub fn analyze_canary(&self, observations: &[CanaryObservation], config: &CanaryConfig) -> CanaryAnalysis {
        if observations.is_empty() {
            return CanaryAnalysis {
                should_continue: true,
                should_rollback: false,
                observations: vec![],
                summary: "No observations yet".to_string(),
            };
        }

        // Safety: observations is guaranteed non-empty by the early return above
        let latest = &observations[observations.len() - 1];
        let error_rate = latest.error_rate;
        let should_rollback = error_rate > config.error_threshold;
        let should_continue = !should_rollback && latest.healthy;

        let avg_error_rate: f64 = observations.iter().map(|o| o.error_rate).sum::<f64>()
            / observations.len() as f64;

        let summary = if should_rollback {
            format!(
                "ROLLBACK: Error rate {:.2}% exceeds threshold {:.2}%",
                error_rate * 100.0,
                config.error_threshold * 100.0
            )
        } else if !latest.healthy {
            "PAUSE: Target instance unhealthy, awaiting recovery".to_string()
        } else {
            format!(
                "OK: Error rate {:.2}% (avg {:.2}%), latency {:.0}ms",
                error_rate * 100.0,
                avg_error_rate * 100.0,
                latest.latency_ms
            )
        };

        CanaryAnalysis {
            should_continue,
            should_rollback,
            observations: observations.to_vec(),
            summary,
        }
    }
}

/// Format migration advice as a report
pub fn format_migration_advice(advice: &MigrationAdvice) -> String {
    let mut output = String::new();

    output.push_str(&output::property_section(&[
        ("Strategy", format!("{:?}", advice.recommended_strategy)),
        ("Risk Level", format!("{}", advice.risk_level)),
        ("Estimated Downtime", format!("{}s", advice.estimated_downtime_secs)),
    ]));

    output.push_str("Reasons:\n");
    for reason in &advice.reasons {
        output.push_str(&output::tree_bullet("✓", reason));
    }

    if !advice.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in &advice.warnings {
            output.push_str(&output::tree_bullet("⚠", warning));
        }
    }

    output.push_str(&format!("\nTiming: {}\n", advice.suggested_timing.recommendation));
    output.push_str(&format!("  Window: {}\n", advice.suggested_timing.preferred_window));

    output.push_str(&format!(
        "\nCanary: {} steps, {}s intervals, {:.1}% error threshold\n",
        advice.canary_config.steps.len(),
        advice.canary_config.step_interval_secs,
        advice.canary_config.error_threshold * 100.0,
    ));

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_test_workload() -> Workload {
        Workload {
            api_version: "aether/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test-app".to_string(),
                owner: "test".to_string(),
                project: "demo".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/test".to_string(),
                build_args: HashMap::new(),
            ..Default::default()
            },
            requirements: ResourceRequirements {
                cpu: "2".to_string(),
                memory: "4Gi".to_string(),
                storage: "20Gi".to_string(),
                gpu: None,
                cpu_request: None,
                memory_request: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Container, RuntimeType::Kube],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
            intent: None,
            schedule: None,
        kubernetes: None,
        }
    }

    #[test]
    fn test_simple_migration_advice() {
        let advisor = MigrationAdvisor::with_defaults();
        let spec = create_test_workload();
        let advice = advisor.advise(&spec, RuntimeKind::Podman, RuntimeKind::Kubernetes);

        assert_eq!(advice.risk_level, RiskLevel::Low);
        assert!(!advice.reasons.is_empty());
    }

    #[test]
    fn test_stateful_migration_is_higher_risk() {
        let advisor = MigrationAdvisor::with_defaults();
        let mut spec = create_test_workload();
        spec.persistence.enabled = true;

        let advice = advisor.advise(&spec, RuntimeKind::Podman, RuntimeKind::Kubernetes);
        assert!(matches!(advice.risk_level, RiskLevel::Medium | RiskLevel::High));
    }

    #[test]
    fn test_cross_category_migration_high_risk() {
        let advisor = MigrationAdvisor::with_defaults();
        let spec = create_test_workload();
        let advice = advisor.advise(&spec, RuntimeKind::Podman, RuntimeKind::Metal3);

        assert!(matches!(advice.risk_level, RiskLevel::Medium | RiskLevel::High));
    }

    #[test]
    fn test_canary_analysis_healthy() {
        let advisor = MigrationAdvisor::with_defaults();
        let config = CanaryConfig {
            steps: vec![25, 50, 75, 100],
            step_interval_secs: 5,
            error_threshold: 0.05,
            latency_threshold_pct: 30.0,
            min_observation_secs: 10,
        };

        let observations = vec![CanaryObservation {
            step: 1,
            traffic_pct: 25,
            error_rate: 0.01,
            latency_ms: 50.0,
            healthy: true,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        }];

        let analysis = advisor.analyze_canary(&observations, &config);
        assert!(analysis.should_continue);
        assert!(!analysis.should_rollback);
    }

    #[test]
    fn test_canary_analysis_rollback() {
        let advisor = MigrationAdvisor::with_defaults();
        let config = CanaryConfig {
            steps: vec![25, 50, 75, 100],
            step_interval_secs: 5,
            error_threshold: 0.05,
            latency_threshold_pct: 30.0,
            min_observation_secs: 10,
        };

        let observations = vec![CanaryObservation {
            step: 1,
            traffic_pct: 25,
            error_rate: 0.15,
            latency_ms: 500.0,
            healthy: false,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        }];

        let analysis = advisor.analyze_canary(&observations, &config);
        assert!(!analysis.should_continue);
        assert!(analysis.should_rollback);
    }

    #[test]
    fn test_format_advice() {
        let advisor = MigrationAdvisor::with_defaults();
        let spec = create_test_workload();
        let advice = advisor.advise(&spec, RuntimeKind::Podman, RuntimeKind::Kubernetes);
        let report = format_migration_advice(&advice);
        assert!(report.contains("Strategy"));
        assert!(report.contains("Risk Level"));
    }
}
