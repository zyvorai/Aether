// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Runtime affinity learning
//!
//! Learns from deployment history which runtimes work best for
//! different workload profiles. Builds a compatibility matrix
//! and reputation scores over time.

use crate::output;
use crate::runtime::RuntimeKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A deployment outcome record for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentOutcome {
    pub workload_name: String,
    pub workload_class: WorkloadClass,
    pub runtime: RuntimeKind,
    pub success: bool,
    pub uptime_pct: Option<f64>,
    pub avg_latency_ms: Option<f64>,
    pub error_rate_pct: Option<f64>,
    pub restarts: u32,
    pub cost_per_day: Option<f64>,
    pub timestamp: String,
    pub failure_reason: Option<String>,
}

/// Workload classification for affinity grouping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkloadClass {
    WebService,
    ApiBackend,
    Database,
    Cache,
    BatchJob,
    MlTraining,
    Worker,
    Microservice,
    Unknown,
}

impl std::fmt::Display for WorkloadClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkloadClass::WebService => write!(f, "Web Service"),
            WorkloadClass::ApiBackend => write!(f, "API Backend"),
            WorkloadClass::Database => write!(f, "Database"),
            WorkloadClass::Cache => write!(f, "Cache"),
            WorkloadClass::BatchJob => write!(f, "Batch Job"),
            WorkloadClass::MlTraining => write!(f, "ML Training"),
            WorkloadClass::Worker => write!(f, "Worker"),
            WorkloadClass::Microservice => write!(f, "Microservice"),
            WorkloadClass::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::str::FromStr for WorkloadClass {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "web-service" => Ok(WorkloadClass::WebService),
            "api-backend" => Ok(WorkloadClass::ApiBackend),
            "database" | "db" => Ok(WorkloadClass::Database),
            "cache" => Ok(WorkloadClass::Cache),
            "batch-job" | "batch" => Ok(WorkloadClass::BatchJob),
            "ml-training" | "ml" => Ok(WorkloadClass::MlTraining),
            "worker" => Ok(WorkloadClass::Worker),
            "microservice" => Ok(WorkloadClass::Microservice),
            _ => Err(anyhow::anyhow!(
                "Unknown workload class: '{}'. Valid: web-service, api-backend, database, cache, batch-job, ml-training, worker, microservice",
                s
            )),
        }
    }
}

/// Runtime affinity score for a workload class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityScore {
    pub runtime: RuntimeKind,
    pub workload_class: WorkloadClass,
    pub success_rate: f64,
    pub avg_uptime: f64,
    pub avg_latency_ms: f64,
    pub avg_error_rate: f64,
    pub avg_cost_per_day: f64,
    pub total_deployments: usize,
    pub composite_score: f64,
    pub confidence: f64,
}

/// Compatibility matrix entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityEntry {
    pub compatible: bool,
    pub score: f64,
    pub deployments: usize,
    pub known_issues: Vec<String>,
}

/// Affinity learning engine
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AffinityEngine {
    outcomes: Vec<DeploymentOutcome>,
    /// Known incompatibilities (learned from failures)
    incompatibilities: Vec<Incompatibility>,
}

/// A learned incompatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incompatibility {
    pub workload_class: WorkloadClass,
    pub runtime: RuntimeKind,
    pub reason: String,
    pub failure_count: usize,
    pub last_seen: String,
}

crate::impl_json_store!(AffinityEngine, "affinity.json");

impl AffinityEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Maximum number of deployment outcomes to retain
    const MAX_OUTCOMES: usize = 10_000;

    /// Record a deployment outcome
    pub fn record(&mut self, outcome: DeploymentOutcome) {
        // Track incompatibilities from failures
        if !outcome.success {
            self.record_failure(&outcome);
        }
        self.outcomes.push(outcome);
        // Prune oldest outcomes if over limit
        if self.outcomes.len() > Self::MAX_OUTCOMES {
            let drain = self.outcomes.len() - Self::MAX_OUTCOMES;
            self.outcomes.drain(..drain);
        }
    }

    /// Get the best runtime recommendation for a workload class
    pub fn recommend(&self, class: &WorkloadClass) -> Vec<AffinityScore> {
        let runtimes = [
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            RuntimeKind::KubeVirt,
        ];

        let mut scores: Vec<AffinityScore> = runtimes
            .iter()
            .map(|rt| self.compute_affinity(class, rt))
            .collect();

        scores.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scores
    }

    /// Get the full compatibility matrix
    pub fn compatibility_matrix(
        &self,
    ) -> HashMap<(WorkloadClass, RuntimeKind), CompatibilityEntry> {
        let classes = [
            WorkloadClass::WebService,
            WorkloadClass::ApiBackend,
            WorkloadClass::Database,
            WorkloadClass::Cache,
            WorkloadClass::BatchJob,
            WorkloadClass::MlTraining,
            WorkloadClass::Worker,
            WorkloadClass::Microservice,
        ];
        let runtimes = [
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            RuntimeKind::KubeVirt,
        ];

        let mut matrix = HashMap::new();
        for class in &classes {
            for rt in &runtimes {
                let score = self.compute_affinity(class, rt);
                let known_issues: Vec<String> = self
                    .incompatibilities
                    .iter()
                    .filter(|i| i.workload_class == *class && i.runtime == *rt)
                    .map(|i| i.reason.clone())
                    .collect();

                matrix.insert(
                    (class.clone(), *rt),
                    CompatibilityEntry {
                        compatible: score.success_rate > 0.5 || score.total_deployments == 0,
                        score: score.composite_score,
                        deployments: score.total_deployments,
                        known_issues,
                    },
                );
            }
        }
        matrix
    }

    /// Get known incompatibilities
    pub fn incompatibilities(&self) -> &[Incompatibility] {
        &self.incompatibilities
    }

    /// Get learning statistics
    pub fn stats(&self) -> LearningStats {
        let total_outcomes = self.outcomes.len();
        let successes = self.outcomes.iter().filter(|o| o.success).count();
        let failures = total_outcomes - successes;

        let mut by_runtime: HashMap<String, usize> = HashMap::new();
        let mut by_class: HashMap<String, usize> = HashMap::new();

        for outcome in &self.outcomes {
            *by_runtime
                .entry(format!("{}", outcome.runtime))
                .or_insert(0) += 1;
            *by_class
                .entry(format!("{}", outcome.workload_class))
                .or_insert(0) += 1;
        }

        LearningStats {
            total_outcomes,
            successes,
            failures,
            incompatibilities: self.incompatibilities.len(),
            by_runtime,
            by_class,
        }
    }

    // --- Private ---

    fn compute_affinity(&self, class: &WorkloadClass, runtime: &RuntimeKind) -> AffinityScore {
        let relevant: Vec<&DeploymentOutcome> = self
            .outcomes
            .iter()
            .filter(|o| o.workload_class == *class && o.runtime == *runtime)
            .collect();

        let total = relevant.len();

        if total == 0 {
            // No data: return neutral score with base heuristics
            return AffinityScore {
                runtime: *runtime,
                workload_class: class.clone(),
                success_rate: self.heuristic_success_rate(class, runtime),
                avg_uptime: 99.0,
                avg_latency_ms: 100.0,
                avg_error_rate: 0.5,
                avg_cost_per_day: self.heuristic_cost(class, runtime),
                total_deployments: 0,
                composite_score: self.heuristic_score(class, runtime),
                confidence: 0.0,
            };
        }

        let successes = relevant.iter().filter(|o| o.success).count();
        let success_rate = successes as f64 / total as f64;

        let uptime_count = relevant.iter().filter(|o| o.uptime_pct.is_some()).count();
        let avg_uptime = if uptime_count > 0 {
            relevant.iter().filter_map(|o| o.uptime_pct).sum::<f64>() / uptime_count as f64
        } else {
            0.0 // No data — return 0 instead of heuristic value
        };

        let latency_count = relevant
            .iter()
            .filter(|o| o.avg_latency_ms.is_some())
            .count();
        let avg_latency = if latency_count > 0 {
            relevant
                .iter()
                .filter_map(|o| o.avg_latency_ms)
                .sum::<f64>()
                / latency_count as f64
        } else {
            0.0
        };

        let error_count = relevant
            .iter()
            .filter(|o| o.error_rate_pct.is_some())
            .count();
        let avg_error = if error_count > 0 {
            relevant
                .iter()
                .filter_map(|o| o.error_rate_pct)
                .sum::<f64>()
                / error_count as f64
        } else {
            0.0
        };

        let avg_cost = relevant.iter().filter_map(|o| o.cost_per_day).sum::<f64>()
            / relevant
                .iter()
                .filter(|o| o.cost_per_day.is_some())
                .count()
                .max(1) as f64;

        // Composite score: weighted combination
        let composite = success_rate * 0.4
            + (avg_uptime / 100.0) * 0.25
            + (1.0 - (avg_error / 100.0).min(1.0)) * 0.2
            + (1.0 - (avg_latency / 1000.0).min(1.0)) * 0.15;

        // Confidence based on sample size (max at ~20 deployments)
        let confidence = (total as f64 / 20.0).min(1.0);

        AffinityScore {
            runtime: *runtime,
            workload_class: class.clone(),
            success_rate,
            avg_uptime,
            avg_latency_ms: avg_latency,
            avg_error_rate: avg_error,
            avg_cost_per_day: avg_cost,
            total_deployments: total,
            composite_score: composite,
            confidence,
        }
    }

    fn record_failure(&mut self, outcome: &DeploymentOutcome) {
        let reason = outcome
            .failure_reason
            .clone()
            .unwrap_or_else(|| "Unknown failure".to_string());

        if let Some(existing) = self
            .incompatibilities
            .iter_mut()
            .find(|i| i.workload_class == outcome.workload_class && i.runtime == outcome.runtime)
        {
            existing.failure_count += 1;
            existing.last_seen = outcome.timestamp.clone();
            if !existing.reason.contains(&reason) {
                existing.reason = format!("{}; {}", existing.reason, reason);
                if existing.reason.len() > 500 {
                    // Truncate at a char boundary, leaving room for "..."
                    let mut end = 497.min(existing.reason.len());
                    while end > 0 && !existing.reason.is_char_boundary(end) {
                        end -= 1;
                    }
                    existing.reason.truncate(end);
                    existing.reason.push_str("...");
                }
            }
        } else {
            self.incompatibilities.push(Incompatibility {
                workload_class: outcome.workload_class.clone(),
                runtime: outcome.runtime,
                reason,
                failure_count: 1,
                last_seen: outcome.timestamp.clone(),
            });
        }
    }

    fn heuristic_success_rate(&self, class: &WorkloadClass, runtime: &RuntimeKind) -> f64 {
        match (class, runtime) {
            (WorkloadClass::WebService, RuntimeKind::Kubernetes) => 0.95,
            (WorkloadClass::WebService, RuntimeKind::Podman) => 0.90,
            (WorkloadClass::Database, RuntimeKind::Kubernetes) => 0.90,
            (WorkloadClass::Database, RuntimeKind::KubeVirt) => 0.80,
            (WorkloadClass::MlTraining, RuntimeKind::KubeVirt) => 0.80,
            (WorkloadClass::BatchJob, RuntimeKind::Podman) => 0.95,
            (WorkloadClass::Cache, RuntimeKind::Kubernetes) => 0.95,
            _ => 0.75,
        }
    }

    fn heuristic_cost(&self, _class: &WorkloadClass, runtime: &RuntimeKind) -> f64 {
        match runtime {
            RuntimeKind::Podman | RuntimeKind::Docker => 5.0,
            RuntimeKind::Kubernetes => 15.0,
            RuntimeKind::KubeVirt => 25.0,
        }
    }

    fn heuristic_score(&self, class: &WorkloadClass, runtime: &RuntimeKind) -> f64 {
        self.heuristic_success_rate(class, runtime) * 0.8 + 0.1
    }
}

/// Learning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStats {
    pub total_outcomes: usize,
    pub successes: usize,
    pub failures: usize,
    pub incompatibilities: usize,
    pub by_runtime: HashMap<String, usize>,
    pub by_class: HashMap<String, usize>,
}

/// Format affinity recommendation
pub fn format_affinity_report(class: &WorkloadClass, scores: &[AffinityScore]) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n  Runtime Affinity for {}\n\n", class));

    let rows: Vec<Vec<String>> = scores
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let indicator = if i == 0 { "★" } else { " " };
            let details = if s.total_deployments > 0 {
                format!(
                    "{:.0}% success | {:.1}% uptime | {:.0}ms latency",
                    s.success_rate * 100.0,
                    s.avg_uptime,
                    s.avg_latency_ms,
                )
            } else {
                "heuristic (no history)".to_string()
            };
            vec![
                format!("{} {}", indicator, s.runtime),
                format!("{:.0}%", s.composite_score * 100.0),
                format!("{:.0}%", s.confidence * 100.0),
                s.total_deployments.to_string(),
                details,
            ]
        })
        .collect();

    out.push_str(&output::table(
        &["Runtime", "Score", "Confidence", "Deploys", "Details"],
        rows,
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_outcome(
        class: WorkloadClass,
        runtime: RuntimeKind,
        success: bool,
        uptime: f64,
    ) -> DeploymentOutcome {
        DeploymentOutcome {
            workload_name: "test".to_string(),
            workload_class: class,
            runtime,
            success,
            uptime_pct: Some(uptime),
            avg_latency_ms: Some(50.0),
            error_rate_pct: Some(0.5),
            restarts: 0,
            cost_per_day: Some(10.0),
            timestamp: crate::resources::now_rfc3339(),
            failure_reason: if success {
                None
            } else {
                Some("test failure".to_string())
            },
        }
    }

    #[test]
    fn test_record_and_recommend() {
        let mut engine = AffinityEngine::new();

        // Record several Kubernetes successes for web services
        for _ in 0..5 {
            engine.record(make_outcome(
                WorkloadClass::WebService,
                RuntimeKind::Kubernetes,
                true,
                99.9,
            ));
        }
        // Record a Podman failure
        engine.record(make_outcome(
            WorkloadClass::WebService,
            RuntimeKind::Podman,
            false,
            95.0,
        ));

        let recs = engine.recommend(&WorkloadClass::WebService);
        assert!(!recs.is_empty());
        // Kubernetes should score higher
        assert_eq!(recs[0].runtime, RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_incompatibility_tracking() {
        let mut engine = AffinityEngine::new();

        engine.record(make_outcome(
            WorkloadClass::Database,
            RuntimeKind::Podman,
            false,
            0.0,
        ));
        engine.record(make_outcome(
            WorkloadClass::Database,
            RuntimeKind::Podman,
            false,
            0.0,
        ));

        let incompat = engine.incompatibilities();
        assert_eq!(incompat.len(), 1);
        assert_eq!(incompat[0].failure_count, 2);
    }

    #[test]
    fn test_heuristic_when_no_data() {
        let engine = AffinityEngine::new();
        let recs = engine.recommend(&WorkloadClass::WebService);

        // Should return scores based on heuristics
        assert_eq!(recs.len(), 3);
        assert!(recs.iter().all(|r| r.confidence == 0.0));
    }

    #[test]
    fn test_compatibility_matrix() {
        let engine = AffinityEngine::new();
        let matrix = engine.compatibility_matrix();
        // 8 classes x 3 runtimes = 24 entries
        assert_eq!(matrix.len(), 24);
    }

    #[test]
    fn test_learning_stats() {
        let mut engine = AffinityEngine::new();
        engine.record(make_outcome(
            WorkloadClass::WebService,
            RuntimeKind::Kubernetes,
            true,
            99.9,
        ));
        engine.record(make_outcome(
            WorkloadClass::Database,
            RuntimeKind::Podman,
            false,
            0.0,
        ));

        let stats = engine.stats();
        assert_eq!(stats.total_outcomes, 2);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 1);
    }

    #[test]
    fn test_format_report() {
        let engine = AffinityEngine::new();
        let recs = engine.recommend(&WorkloadClass::WebService);
        let output = format_affinity_report(&WorkloadClass::WebService, &recs);
        assert!(output.contains("Web Service"));
    }

    #[test]
    fn test_workload_class_from_str() {
        assert_eq!(
            "web-service".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::WebService
        );
        assert_eq!(
            "api-backend".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::ApiBackend
        );
        assert_eq!(
            "database".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::Database
        );
        assert_eq!(
            "db".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::Database
        );
        assert_eq!(
            "cache".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::Cache
        );
        assert_eq!(
            "batch-job".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::BatchJob
        );
        assert_eq!(
            "ml-training".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::MlTraining
        );
        assert_eq!(
            "worker".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::Worker
        );
        assert_eq!(
            "microservice".parse::<WorkloadClass>().unwrap(),
            WorkloadClass::Microservice
        );
        assert!("unknown-class".parse::<WorkloadClass>().is_err());
    }
}
