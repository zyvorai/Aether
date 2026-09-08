// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! AI-powered scoring engine for runtime selection
//!
//! Multi-factor scoring that evaluates each runtime against cost, performance,
//! reliability, and availability criteria, returning a ranked list with explanations.

use crate::config::{EngineConfig, ScoringWeights};
use crate::output;
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Score for a single runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeScore {
    pub runtime: RuntimeKind,
    pub total_score: f64,
    pub cost_score: f64,
    pub performance_score: f64,
    pub reliability_score: f64,
    pub availability_score: f64,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

/// Complete scoring result with all runtimes evaluated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringResult {
    pub recommended: RuntimeKind,
    pub scores: Vec<RuntimeScore>,
    pub workload_class: WorkloadClass,
    pub confidence: f64,
}

/// Workload classification based on spec analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkloadClass {
    /// Stateless web service / API
    Stateless,
    /// Stateful with persistent storage
    Stateful,
    /// GPU-accelerated compute
    GpuCompute,
    /// Batch / job workload
    Batch,
    /// General purpose
    General,
}

impl std::fmt::Display for WorkloadClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkloadClass::Stateless => write!(f, "Stateless Service"),
            WorkloadClass::Stateful => write!(f, "Stateful Service"),
            WorkloadClass::GpuCompute => write!(f, "GPU Compute"),
            WorkloadClass::Batch => write!(f, "Batch Job"),
            WorkloadClass::General => write!(f, "General Purpose"),
        }
    }
}

/// Historical performance record for a runtime
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeHistory {
    pub total_deployments: u64,
    pub successful_deployments: u64,
    pub avg_startup_secs: f64,
    pub avg_uptime_pct: f64,
    pub total_migrations_from: u64,
    pub total_migrations_to: u64,
}

/// Scoring engine
pub struct ScoringEngine {
    config: EngineConfig,
    history: HashMap<RuntimeKind, RuntimeHistory>,
}

impl ScoringEngine {
    /// Create a new scoring engine with configuration
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            history: HashMap::new(),
        }
    }

    /// Create a scoring engine with default configuration
    pub fn with_defaults() -> Self {
        Self::new(EngineConfig::default())
    }

    /// Load historical data from state
    pub fn with_history(mut self, history: HashMap<RuntimeKind, RuntimeHistory>) -> Self {
        self.history = history;
        self
    }

    /// Score all allowed runtimes and return ranked results
    pub fn score(&self, spec: &Workload) -> ScoringResult {
        let workload_class = self.classify_workload(spec);
        let base_weights = self.config.scoring_weights.normalized();
        let weights = self.adjust_weights_for_intent(spec, base_weights);
        let allowed_runtimes = self.get_allowed_runtimes(spec);
        let allowed_runtimes = self.filter_by_intent(spec, allowed_runtimes);

        let mut scores: Vec<RuntimeScore> = allowed_runtimes
            .iter()
            .map(|runtime| self.score_runtime(*runtime, spec, &workload_class, &weights))
            .collect();

        // Sort by total score descending
        scores.sort_by(|a, b| {
            b.total_score
                .partial_cmp(&a.total_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let recommended = match scores.first() {
            Some(s) => s.runtime,
            None => {
                tracing::error!(
                    "No allowed runtimes found for workload '{}'; defaulting to Podman",
                    spec.metadata.name
                );
                RuntimeKind::Podman
            }
        };

        // Calculate confidence based on score spread
        let confidence = if scores.len() >= 2 {
            let top = scores[0].total_score;
            let second = scores[1].total_score;
            if top > 0.0 {
                ((top - second) / top).min(1.0)
            } else {
                0.0
            }
        } else {
            1.0
        };

        ScoringResult {
            recommended,
            scores,
            workload_class,
            confidence,
        }
    }

    /// Classify the workload based on its spec
    pub fn classify_workload(&self, spec: &Workload) -> WorkloadClass {
        // GPU workloads
        if spec.requirements.gpu.is_some() {
            return WorkloadClass::GpuCompute;
        }

        // Stateful with persistence
        if spec.persistence.enabled {
            return WorkloadClass::Stateful;
        }

        // Stateless with networking (typical web service)
        if spec.network.service {
            return WorkloadClass::Stateless;
        }

        WorkloadClass::General
    }

    /// Score a single runtime against the workload
    fn score_runtime(
        &self,
        runtime: RuntimeKind,
        spec: &Workload,
        class: &WorkloadClass,
        weights: &ScoringWeights,
    ) -> RuntimeScore {
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();

        // Cost score (0.0 - 1.0, higher = cheaper)
        let cost_score = self.score_cost(runtime, spec, &mut reasons);

        // Performance score (0.0 - 1.0, higher = better fit)
        let performance_score =
            self.score_performance(runtime, spec, class, &mut reasons, &mut warnings);

        // Reliability score (0.0 - 1.0, higher = more reliable)
        let reliability_score = self.score_reliability(runtime, &mut reasons);

        // Availability score (0.0 - 1.0, higher = more available)
        let availability_score = self.score_availability(runtime, spec, &mut reasons);

        let mut total_score = weights.cost * cost_score
            + weights.performance * performance_score
            + weights.reliability * reliability_score
            + weights.availability * availability_score;

        // Apply intent-specific score adjustments
        let intent_multiplier = self.intent_score_adjustment(runtime, spec, &mut reasons);
        total_score *= intent_multiplier;

        RuntimeScore {
            runtime,
            total_score,
            cost_score,
            performance_score,
            reliability_score,
            availability_score,
            reasons,
            warnings,
        }
    }

    /// Score runtime on cost efficiency
    fn score_cost(&self, runtime: RuntimeKind, spec: &Workload, reasons: &mut Vec<String>) -> f64 {
        let cpu = crate::resources::parse_cpu(&spec.requirements.cpu);
        let memory_gi = crate::resources::parse_memory_gi(&spec.requirements.memory);

        match runtime {
            RuntimeKind::Podman | RuntimeKind::Docker => {
                reasons.push("Local container: no cloud cost".to_string());
                if cpu <= 4.0 && memory_gi <= 8.0 {
                    0.95 // Very cheap for small workloads
                } else {
                    0.70 // Local resources have limits
                }
            }
            RuntimeKind::Kubernetes => {
                if cpu <= 8.0 && memory_gi <= 32.0 {
                    reasons
                        .push("K8s efficient for medium workloads with shared infra".to_string());
                    0.75
                } else {
                    reasons.push("K8s overhead increases with large resource requests".to_string());
                    0.55
                }
            }
            RuntimeKind::KubeVirt => {
                if spec.requirements.gpu.is_some() {
                    reasons.push("GPU passthrough requires VM isolation".to_string());
                    0.60
                } else {
                    reasons.push("VM overhead increases resource cost".to_string());
                    0.40
                }
            }
        }
    }

    /// Score runtime on performance characteristics
    fn score_performance(
        &self,
        runtime: RuntimeKind,
        spec: &Workload,
        class: &WorkloadClass,
        reasons: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> f64 {
        match (runtime, class) {
            // GPU workloads need KubeVirt
            (RuntimeKind::KubeVirt, WorkloadClass::GpuCompute) => {
                reasons.push("GPU passthrough with VM isolation".to_string());
                0.90
            }
            (RuntimeKind::Podman | RuntimeKind::Docker, WorkloadClass::GpuCompute) => {
                warnings.push("Container GPU support varies by host".to_string());
                0.50
            }
            (RuntimeKind::Kubernetes, WorkloadClass::GpuCompute) => {
                reasons.push("GPU device plugin required".to_string());
                0.65
            }

            // Stateful workloads prefer K8s (PVC, StatefulSets)
            (RuntimeKind::Kubernetes, WorkloadClass::Stateful) => {
                reasons.push("K8s provides persistent volumes and stateful primitives".to_string());
                0.90
            }
            (RuntimeKind::Podman | RuntimeKind::Docker, WorkloadClass::Stateful) => {
                reasons.push("Local volumes work but lack redundancy".to_string());
                0.55
            }

            // Stateless services
            (RuntimeKind::Kubernetes, WorkloadClass::Stateless) => {
                if spec.scaling.is_some() {
                    reasons.push("K8s HPA enables auto-scaling".to_string());
                    0.90
                } else {
                    reasons.push("K8s provides service discovery and load balancing".to_string());
                    0.80
                }
            }
            (RuntimeKind::Podman | RuntimeKind::Docker, WorkloadClass::Stateless) => {
                reasons.push("Fast local iteration for development".to_string());
                0.75
            }

            // General defaults
            (RuntimeKind::Podman | RuntimeKind::Docker, _) => {
                reasons.push("Simple container execution".to_string());
                0.65
            }
            (RuntimeKind::Kubernetes, _) => {
                reasons.push("Orchestrated deployment with health management".to_string());
                0.70
            }
            (RuntimeKind::KubeVirt, _) => {
                reasons.push("VM isolation provides security boundary".to_string());
                0.50
            }
        }
    }

    /// Score runtime on reliability
    fn score_reliability(&self, runtime: RuntimeKind, reasons: &mut Vec<String>) -> f64 {
        // Use historical data if available
        if let Some(history) = self.history.get(&runtime) {
            if history.total_deployments > 0 {
                let success_rate =
                    history.successful_deployments as f64 / history.total_deployments as f64;
                reasons.push(format!(
                    "Historical success rate: {:.1}% ({} deployments)",
                    success_rate * 100.0,
                    history.total_deployments
                ));
                return success_rate;
            }
        }

        // Default reliability scores based on runtime maturity
        match runtime {
            RuntimeKind::Kubernetes => {
                reasons.push("Production-grade orchestrator with self-healing".to_string());
                0.90
            }
            RuntimeKind::Podman | RuntimeKind::Docker => {
                reasons.push("Stable container runtime, local execution".to_string());
                0.85
            }
            RuntimeKind::KubeVirt => {
                reasons.push("VM lifecycle managed by K8s operators".to_string());
                0.75
            }
        }
    }

    /// Score runtime on availability characteristics
    fn score_availability(
        &self,
        runtime: RuntimeKind,
        spec: &Workload,
        reasons: &mut Vec<String>,
    ) -> f64 {
        let has_scaling = spec.scaling.as_ref().is_some_and(|s| s.enabled);
        let has_health = spec.health.is_some();

        match runtime {
            RuntimeKind::Kubernetes => {
                let mut score: f64 = 0.80;
                if has_scaling {
                    score += 0.10;
                    reasons.push("HPA enables automatic scaling for availability".to_string());
                }
                if has_health {
                    score += 0.05;
                    reasons.push("Health probes enable automatic restart".to_string());
                }
                score.min(1.0)
            }
            RuntimeKind::Podman | RuntimeKind::Docker => {
                reasons.push("Single-node: no automatic failover".to_string());
                0.50
            }
            RuntimeKind::KubeVirt => {
                reasons.push("VM live migration can improve availability".to_string());
                0.70
            }
        }
    }

    /// Get allowed runtimes from spec
    fn get_allowed_runtimes(&self, spec: &Workload) -> Vec<RuntimeKind> {
        use crate::spec::RuntimeType;

        spec.runtime
            .allow
            .iter()
            .map(|rt| match rt {
                RuntimeType::Container => RuntimeKind::Podman,
                RuntimeType::Kube => RuntimeKind::Kubernetes,
                RuntimeType::Kubevirt => RuntimeKind::KubeVirt,
            })
            .collect()
    }

    // ─── Intent-aware scoring ────────────────────────────────────────────

    /// Adjust scoring weights based on the workload's intent goal.
    /// When no intent is set, returns the base weights unchanged.
    fn adjust_weights_for_intent(&self, spec: &Workload, base: ScoringWeights) -> ScoringWeights {
        use crate::spec::IntentGoal;

        let intent = match &spec.intent {
            Some(i) => i,
            None => return base,
        };

        let adjusted = match intent.goal {
            IntentGoal::LowLatency => ScoringWeights {
                cost: 0.15,
                performance: 0.45,
                reliability: 0.25,
                availability: 0.15,
            },
            IntentGoal::HighThroughput => ScoringWeights {
                cost: 0.20,
                performance: 0.40,
                reliability: 0.15,
                availability: 0.25,
            },
            IntentGoal::CostOptimized => ScoringWeights {
                cost: 0.50,
                performance: 0.20,
                reliability: 0.15,
                availability: 0.15,
            },
            IntentGoal::Balanced => return base,
        };

        adjusted.normalized()
    }

    /// Filter runtimes that violate hard intent constraints.
    /// Budget, compliance isolation, resilience HA, and extreme latency
    /// requirements can eliminate candidates before scoring.
    fn filter_by_intent(&self, spec: &Workload, runtimes: Vec<RuntimeKind>) -> Vec<RuntimeKind> {
        let intent = match &spec.intent {
            Some(i) => i,
            None => return runtimes,
        };

        let mut filtered = runtimes;

        // Compliance: isolation required → remove container runtimes
        if let Some(ref compliance) = intent.compliance {
            if compliance.isolation_required {
                filtered.retain(|rt| !matches!(rt, RuntimeKind::Podman | RuntimeKind::Docker));
            }
        }

        // Trust: Strict → only VM runtimes (attested, isolated)
        if let Some(crate::spec::TrustLevel::Strict) = intent.trust {
            filtered.retain(|rt| *rt == RuntimeKind::KubeVirt);
        }

        // Confidential + attestation required → KubeVirt only
        if spec
            .confidential
            .as_ref()
            .is_some_and(|c| c.enabled && c.attestation.required)
        {
            filtered.retain(|rt| *rt == RuntimeKind::KubeVirt);
        }

        // Resilience: High → remove single-host runtimes (no HA)
        if let Some(crate::spec::ResilienceLevel::High) = intent.resilience {
            filtered.retain(|rt| !matches!(rt, RuntimeKind::Podman | RuntimeKind::Docker));
        }

        // Budget: estimate cheapest provider cost per runtime, remove those exceeding cap
        if let Some(ref budget) = intent.budget {
            filtered.retain(|rt| {
                let estimated = self.estimate_runtime_monthly_cost(*rt, spec);
                if estimated > budget.max_monthly_usd {
                    tracing::info!(
                        "Intent filter: {} excluded (est ${:.0}/mo > budget ${:.0})",
                        rt,
                        estimated,
                        budget.max_monthly_usd
                    );
                    false
                } else {
                    true
                }
            });
        }

        // Never return empty — fall back to full list if all filtered
        if filtered.is_empty() {
            tracing::warn!(
                "Intent constraints filtered all runtimes for '{}'; ignoring filters",
                spec.metadata.name
            );
            return self.get_allowed_runtimes(spec);
        }

        filtered
    }

    /// Estimate the cheapest monthly cost for a workload on a given runtime.
    /// Uses the minimum across all cloud providers as a proxy.
    /// Local runtimes (Podman/Docker) are treated as zero cost.
    fn estimate_runtime_monthly_cost(&self, runtime: RuntimeKind, spec: &Workload) -> f64 {
        match runtime {
            RuntimeKind::Podman | RuntimeKind::Docker => 0.0,
            _ => {
                // Use cheapest provider estimate as a lower bound
                crate::cost::estimate_all_providers(spec)
                    .ok()
                    .and_then(|estimates| estimates.first().map(|e| e.total_monthly))
                    .unwrap_or(0.0)
            }
        }
    }

    /// Return a score multiplier (0.5–1.5) based on how well a runtime
    /// matches the intent's soft preferences.
    fn intent_score_adjustment(
        &self,
        runtime: RuntimeKind,
        spec: &Workload,
        reasons: &mut Vec<String>,
    ) -> f64 {
        let intent = match &spec.intent {
            Some(i) => i,
            None => return 1.0,
        };

        let mut multiplier = 1.0;

        // Compliance isolation bonus for VM runtimes
        if let Some(ref compliance) = intent.compliance {
            if compliance.isolation_required && runtime == RuntimeKind::KubeVirt {
                multiplier *= 1.3;
                reasons.push("Intent: isolation bonus (VM)".to_string());
            }
        }

        // Trust: strict → bonus for isolated runtimes (VM)
        if let Some(crate::spec::TrustLevel::Strict) = intent.trust {
            if runtime == RuntimeKind::KubeVirt {
                multiplier *= 1.3;
                reasons.push("Intent: strict trust bonus (VM isolation)".to_string());
            }
        }

        // High resilience bonus for Kubernetes (native HA)
        if let Some(crate::spec::ResilienceLevel::High) = intent.resilience {
            if runtime == RuntimeKind::Kubernetes {
                multiplier *= 1.2;
                reasons.push("Intent: HA bonus (Kubernetes native)".to_string());
            }
        }

        // Low latency bonus for local containers on small workloads
        if intent.goal == crate::spec::IntentGoal::LowLatency {
            let cpu = crate::resources::parse_cpu(&spec.requirements.cpu);
            if matches!(runtime, RuntimeKind::Podman | RuntimeKind::Docker) && cpu <= 4.0 {
                multiplier *= 1.1;
                reasons.push(
                    "Intent: low-latency bonus (local container, small workload)".to_string(),
                );
            }
        }

        // Cost-optimized bonus for local containers (zero cloud cost)
        if intent.goal == crate::spec::IntentGoal::CostOptimized
            && matches!(runtime, RuntimeKind::Podman | RuntimeKind::Docker)
        {
            multiplier *= 1.2;
            reasons.push("Intent: cost bonus (no cloud spend)".to_string());
        }

        multiplier
    }
}

/// Display a scoring result as a formatted report
pub fn format_scoring_report(result: &ScoringResult) -> String {
    format_scoring_report_with_options(result, false)
}

/// Display a scoring result; when `explain` is true, print +/- for every runtime.
pub fn format_scoring_report_with_options(result: &ScoringResult, explain: bool) -> String {
    let mut output = String::new();

    output.push_str(&output::property_section(&[
        (
            "Workload Classification",
            format!("{}", result.workload_class),
        ),
        (
            "Recommended Runtime",
            format!(
                "{} (confidence: {:.0}%)",
                result.recommended,
                result.confidence * 100.0
            ),
        ),
    ]));

    output.push_str("Runtime Scores:\n");
    output.push_str(&format!(
        "{:<14} {:>6} {:>6} {:>6} {:>6} {:>7}\n",
        "Runtime", "Cost", "Perf", "Rel", "Avail", "Total"
    ));
    output.push_str(&"-".repeat(52));
    output.push('\n');

    for score in &result.scores {
        let marker = if score.runtime == result.recommended {
            " *"
        } else {
            ""
        };
        output.push_str(&format!(
            "{:<14} {:>5.0}% {:>5.0}% {:>5.0}% {:>5.0}% {:>5.0}%{}\n",
            format!("{}", score.runtime),
            score.cost_score * 100.0,
            score.performance_score * 100.0,
            score.reliability_score * 100.0,
            score.availability_score * 100.0,
            score.total_score * 100.0,
            marker,
        ));
    }

    output.push('\n');

    if explain {
        output.push_str("Per-Runtime Analysis:\n");
        for score in &result.scores {
            let marker = if score.runtime == result.recommended {
                " (recommended)"
            } else {
                ""
            };
            output.push_str(&format!(
                "\n{}{} — {:.0}% total\n",
                score.runtime,
                marker,
                score.total_score * 100.0
            ));
            for reason in &score.reasons {
                output.push_str(&output::tree_bullet("+", reason));
            }
            for warning in &score.warnings {
                output.push_str(&output::tree_bullet("-", warning));
            }
            if score.reasons.is_empty() && score.warnings.is_empty() {
                output.push_str(&output::tree_bullet("·", "No specific factors recorded"));
            }
        }
    } else if let Some(top) = result.scores.first() {
        // Show reasons for recommended
        output.push_str("Decision Factors:\n");
        for reason in &top.reasons {
            output.push_str(&output::tree_bullet("✓", reason));
        }
        for warning in &top.warnings {
            output.push_str(&output::tree_bullet("⚠", warning));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_test_workload(allow: Vec<RuntimeType>) -> Workload {
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
                allow,
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
            intent: None,
            autonomy: None,
            confidential: None,
            schedule: None,
            kubernetes: None,
            kubevirt: None,
        }
    }

    #[test]
    fn test_classify_general_workload() {
        let engine = ScoringEngine::with_defaults();
        let spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        assert_eq!(engine.classify_workload(&spec), WorkloadClass::General);
    }

    #[test]
    fn test_classify_gpu_workload() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![RuntimeType::Kubevirt]);
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
        });
        assert_eq!(engine.classify_workload(&spec), WorkloadClass::GpuCompute);
    }

    #[test]
    fn test_classify_stateful_workload() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![RuntimeType::Kube]);
        spec.persistence.enabled = true;
        assert_eq!(engine.classify_workload(&spec), WorkloadClass::Stateful);
    }

    #[test]
    fn test_scoring_returns_ranked_results() {
        let engine = ScoringEngine::with_defaults();
        let spec = create_test_workload(vec![
            RuntimeType::Container,
            RuntimeType::Kube,
            RuntimeType::Kubevirt,
        ]);
        let result = engine.score(&spec);

        assert_eq!(result.scores.len(), 3);
        // Scores should be sorted descending
        for i in 1..result.scores.len() {
            assert!(result.scores[i - 1].total_score >= result.scores[i].total_score);
        }
        assert_eq!(result.recommended, result.scores[0].runtime);
    }

    #[test]
    fn test_format_scoring_report_explain_includes_all_runtimes() {
        let engine = ScoringEngine::with_defaults();
        let spec = create_test_workload(vec![
            RuntimeType::Container,
            RuntimeType::Kube,
            RuntimeType::Kubevirt,
        ]);
        let result = engine.score(&spec);
        let report = format_scoring_report_with_options(&result, true);
        assert!(report.contains("Per-Runtime Analysis"));
        assert!(
            report.contains("podman") || report.contains("Podman") || report.contains("kubernetes")
        );
    }

    #[test]
    fn test_scoring_with_history() {
        let mut history = HashMap::new();
        history.insert(
            RuntimeKind::Podman,
            RuntimeHistory {
                total_deployments: 100,
                successful_deployments: 99,
                avg_startup_secs: 2.0,
                avg_uptime_pct: 99.5,
                total_migrations_from: 5,
                total_migrations_to: 3,
            },
        );

        let engine = ScoringEngine::with_defaults().with_history(history);
        let spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        let result = engine.score(&spec);

        // Podman should score high reliability with 99% success rate
        let podman_score = result
            .scores
            .iter()
            .find(|s| s.runtime == RuntimeKind::Podman)
            .unwrap();
        assert!(podman_score.reliability_score > 0.90);
    }

    #[test]
    fn test_confidence_calculation() {
        let engine = ScoringEngine::with_defaults();
        let spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        let result = engine.score(&spec);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_format_report() {
        let engine = ScoringEngine::with_defaults();
        let spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        let result = engine.score(&spec);
        let report = format_scoring_report(&result);
        assert!(report.contains("Recommended Runtime"));
        assert!(report.contains("Decision Factors"));
    }

    // ---------------------------------------------------------------
    // Intent-aware scoring tests
    // ---------------------------------------------------------------

    use crate::spec::{ComplianceSpec, IntentGoal, IntentSpec, ResilienceLevel};

    fn make_intent(goal: IntentGoal) -> IntentSpec {
        IntentSpec {
            goal,
            sla: None,
            budget: None,
            resilience: None,
            compliance: None,
            trust: None,
            storage: None,
        }
    }

    #[test]
    fn test_weights_adjusted_for_low_latency() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        spec.intent = Some(make_intent(IntentGoal::LowLatency));
        let base_weights = engine.config.scoring_weights.normalized();
        let adjusted = engine.adjust_weights_for_intent(&spec, base_weights);
        assert!(
            adjusted.performance > 0.40,
            "performance weight should be high for low-latency"
        );
        assert!(
            adjusted.cost < 0.20,
            "cost weight should be low for low-latency"
        );
    }

    #[test]
    fn test_weights_adjusted_for_cost_optimized() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        spec.intent = Some(make_intent(IntentGoal::CostOptimized));
        let base_weights = engine.config.scoring_weights.normalized();
        let adjusted = engine.adjust_weights_for_intent(&spec, base_weights);
        assert!(
            adjusted.cost > 0.45,
            "cost weight should be high for cost-optimized"
        );
    }

    #[test]
    fn test_weights_unchanged_for_balanced() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        spec.intent = Some(make_intent(IntentGoal::Balanced));
        let base_weights = engine.config.scoring_weights.normalized();
        let adjusted = engine.adjust_weights_for_intent(&spec, base_weights.clone());
        assert!((adjusted.cost - base_weights.cost).abs() < 0.001);
        assert!((adjusted.performance - base_weights.performance).abs() < 0.001);
    }

    #[test]
    fn test_weights_unchanged_when_no_intent() {
        let engine = ScoringEngine::with_defaults();
        let spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        let base_weights = engine.config.scoring_weights.normalized();
        let adjusted = engine.adjust_weights_for_intent(&spec, base_weights.clone());
        assert!((adjusted.cost - base_weights.cost).abs() < 0.001);
    }

    #[test]
    fn test_filter_removes_podman_when_isolation_required() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![
            RuntimeType::Container,
            RuntimeType::Kube,
            RuntimeType::Kubevirt,
        ]);
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: None,
            resilience: None,
            compliance: Some(ComplianceSpec {
                isolation_required: true,
                encryption_required: false,
            }),
            trust: None,
            storage: None,
        });
        let runtimes = engine.get_allowed_runtimes(&spec);
        let filtered = engine.filter_by_intent(&spec, runtimes);
        assert!(!filtered.contains(&RuntimeKind::Podman));
        assert!(filtered.contains(&RuntimeKind::KubeVirt));
    }

    #[test]
    fn test_filter_removes_podman_when_high_resilience() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: None,
            resilience: Some(ResilienceLevel::High),
            compliance: None,
            trust: None,
            storage: None,
        });
        let runtimes = engine.get_allowed_runtimes(&spec);
        let filtered = engine.filter_by_intent(&spec, runtimes);
        assert!(!filtered.contains(&RuntimeKind::Podman));
        assert!(filtered.contains(&RuntimeKind::Kubernetes));
    }

    #[test]
    fn test_filter_no_change_when_no_intent() {
        let engine = ScoringEngine::with_defaults();
        let spec = create_test_workload(vec![RuntimeType::Container, RuntimeType::Kube]);
        let runtimes = engine.get_allowed_runtimes(&spec);
        let filtered = engine.filter_by_intent(&spec, runtimes.clone());
        assert_eq!(filtered.len(), runtimes.len());
    }

    #[test]
    fn test_filter_fallback_when_all_filtered() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![RuntimeType::Container]);
        // Only Podman allowed, but isolation required — would filter everything
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: None,
            resilience: None,
            compliance: Some(ComplianceSpec {
                isolation_required: true,
                encryption_required: false,
            }),
            trust: None,
            storage: None,
        });
        let runtimes = engine.get_allowed_runtimes(&spec);
        let filtered = engine.filter_by_intent(&spec, runtimes);
        // Should fall back to original list instead of empty
        assert!(!filtered.is_empty());
    }

    #[test]
    fn test_scoring_with_intent_vs_without() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![
            RuntimeType::Container,
            RuntimeType::Kube,
            RuntimeType::Kubevirt,
        ]);

        // Score without intent
        let result_no_intent = engine.score(&spec);

        // Score with cost-optimized intent
        spec.intent = Some(make_intent(IntentGoal::CostOptimized));
        let result_with_intent = engine.score(&spec);

        // Cost-optimized should favor cheaper runtimes
        let no_intent_cost = result_no_intent
            .scores
            .iter()
            .find(|s| s.runtime == RuntimeKind::Podman)
            .map(|s| s.total_score)
            .unwrap_or(0.0);
        let intent_cost = result_with_intent
            .scores
            .iter()
            .find(|s| s.runtime == RuntimeKind::Podman)
            .map(|s| s.total_score)
            .unwrap_or(0.0);
        // With cost-optimized intent, Podman's score should be higher or equal
        assert!(
            intent_cost >= no_intent_cost - 0.01,
            "Cost-optimized intent should not decrease Podman's score"
        );
    }

    #[test]
    fn test_intent_isolation_bonus_applied() {
        let engine = ScoringEngine::with_defaults();
        let mut spec = create_test_workload(vec![RuntimeType::Kube, RuntimeType::Kubevirt]);
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: None,
            resilience: None,
            compliance: Some(ComplianceSpec {
                isolation_required: true,
                encryption_required: false,
            }),
            trust: None,
            storage: None,
        });
        let result = engine.score(&spec);
        // KubeVirt should get isolation bonus
        let kv_score = result
            .scores
            .iter()
            .find(|s| s.runtime == RuntimeKind::KubeVirt);
        assert!(kv_score.is_some());
        assert!(
            kv_score
                .unwrap()
                .reasons
                .iter()
                .any(|r| r.contains("isolation bonus")),
            "KubeVirt should have isolation bonus reason"
        );
    }
}
