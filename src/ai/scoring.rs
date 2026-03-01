//! AI-powered scoring engine for runtime selection
//!
//! Multi-factor scoring that evaluates each runtime against cost, performance,
//! reliability, and availability criteria, returning a ranked list with explanations.

use crate::config::{EngineConfig, ScoringWeights};
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
    /// High-resource bare metal workload
    BareMetal,
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
            WorkloadClass::BareMetal => write!(f, "Bare Metal"),
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
        let weights = self.config.scoring_weights.normalized();
        let allowed_runtimes = self.get_allowed_runtimes(spec);

        let mut scores: Vec<RuntimeScore> = allowed_runtimes
            .iter()
            .map(|runtime| self.score_runtime(*runtime, spec, &workload_class, &weights))
            .collect();

        // Sort by total score descending
        scores.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap_or(std::cmp::Ordering::Equal));

        let recommended = scores.first().map(|s| s.runtime).unwrap_or(RuntimeKind::Podman);

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

        // High-resource bare metal
        let cpu = self.parse_cpu(&spec.requirements.cpu);
        let memory_gi = self.parse_memory_gi(&spec.requirements.memory);
        if cpu > self.config.metal3_cpu_threshold || memory_gi > self.config.metal3_memory_threshold_gi {
            return WorkloadClass::BareMetal;
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
        let performance_score = self.score_performance(runtime, spec, class, &mut reasons, &mut warnings);

        // Reliability score (0.0 - 1.0, higher = more reliable)
        let reliability_score = self.score_reliability(runtime, &mut reasons);

        // Availability score (0.0 - 1.0, higher = more available)
        let availability_score = self.score_availability(runtime, spec, &mut reasons);

        let total_score = weights.cost * cost_score
            + weights.performance * performance_score
            + weights.reliability * reliability_score
            + weights.availability * availability_score;

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
        let cpu = self.parse_cpu(&spec.requirements.cpu);
        let memory_gi = self.parse_memory_gi(&spec.requirements.memory);

        match runtime {
            RuntimeKind::Podman => {
                reasons.push("Local container: no cloud cost".to_string());
                if cpu <= 4.0 && memory_gi <= 8.0 {
                    0.95 // Very cheap for small workloads
                } else {
                    0.70 // Local resources have limits
                }
            }
            RuntimeKind::Kubernetes => {
                if cpu <= 8.0 && memory_gi <= 32.0 {
                    reasons.push("K8s efficient for medium workloads with shared infra".to_string());
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
            RuntimeKind::Metal3 => {
                if cpu > 16.0 || memory_gi > 64.0 {
                    reasons.push("Dedicated hardware efficient at scale".to_string());
                    0.70
                } else {
                    reasons.push("Bare metal over-provisioned for small workloads".to_string());
                    0.20
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
            // GPU workloads need KubeVirt or Metal3
            (RuntimeKind::KubeVirt, WorkloadClass::GpuCompute) => {
                reasons.push("GPU passthrough with VM isolation".to_string());
                0.90
            }
            (RuntimeKind::Metal3, WorkloadClass::GpuCompute) => {
                reasons.push("Direct GPU access on bare metal".to_string());
                0.95
            }
            (RuntimeKind::Podman, WorkloadClass::GpuCompute) => {
                warnings.push("Container GPU support varies by host".to_string());
                0.50
            }
            (RuntimeKind::Kubernetes, WorkloadClass::GpuCompute) => {
                reasons.push("GPU device plugin required".to_string());
                0.65
            }

            // Bare metal workloads
            (RuntimeKind::Metal3, WorkloadClass::BareMetal) => {
                reasons.push("Native hardware performance, no virtualization overhead".to_string());
                0.95
            }
            (RuntimeKind::KubeVirt, WorkloadClass::BareMetal) => {
                reasons.push("VM can provide near-native performance".to_string());
                0.65
            }
            (_, WorkloadClass::BareMetal) => {
                warnings.push("Resource limits may be constrained".to_string());
                0.35
            }

            // Stateful workloads prefer K8s (PVC, StatefulSets)
            (RuntimeKind::Kubernetes, WorkloadClass::Stateful) => {
                reasons.push("K8s provides persistent volumes and stateful primitives".to_string());
                0.90
            }
            (RuntimeKind::Podman, WorkloadClass::Stateful) => {
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
            (RuntimeKind::Podman, WorkloadClass::Stateless) => {
                reasons.push("Fast local iteration for development".to_string());
                0.75
            }

            // General defaults
            (RuntimeKind::Podman, _) => {
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
            (RuntimeKind::Metal3, _) => {
                reasons.push("Dedicated hardware resources".to_string());
                0.45
            }
        }
    }

    /// Score runtime on reliability
    fn score_reliability(&self, runtime: RuntimeKind, reasons: &mut Vec<String>) -> f64 {
        // Use historical data if available
        if let Some(history) = self.history.get(&runtime) {
            if history.total_deployments > 0 {
                let success_rate = history.successful_deployments as f64
                    / history.total_deployments as f64;
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
            RuntimeKind::Podman => {
                reasons.push("Stable container runtime, local execution".to_string());
                0.85
            }
            RuntimeKind::KubeVirt => {
                reasons.push("VM lifecycle managed by K8s operators".to_string());
                0.75
            }
            RuntimeKind::Metal3 => {
                reasons.push("Hardware provisioning has longer failure recovery".to_string());
                0.65
            }
        }
    }

    /// Score runtime on availability characteristics
    fn score_availability(&self, runtime: RuntimeKind, spec: &Workload, reasons: &mut Vec<String>) -> f64 {
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
            RuntimeKind::Podman => {
                reasons.push("Single-node: no automatic failover".to_string());
                0.50
            }
            RuntimeKind::KubeVirt => {
                reasons.push("VM live migration can improve availability".to_string());
                0.70
            }
            RuntimeKind::Metal3 => {
                reasons.push("Hardware failures require physical intervention".to_string());
                0.55
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
                RuntimeType::Metal => RuntimeKind::Metal3,
            })
            .collect()
    }

    fn parse_cpu(&self, cpu: &str) -> f64 {
        crate::resources::parse_cpu(cpu)
    }

    fn parse_memory_gi(&self, memory: &str) -> f64 {
        crate::resources::parse_memory_gi(memory)
    }
}

/// Display a scoring result as a formatted report
pub fn format_scoring_report(result: &ScoringResult) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Workload Classification: {}\n",
        result.workload_class
    ));
    output.push_str(&format!(
        "Recommended Runtime: {} (confidence: {:.0}%)\n\n",
        result.recommended, result.confidence * 100.0
    ));

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

    // Show reasons for recommended
    if let Some(top) = result.scores.first() {
        output.push_str("Decision Factors:\n");
        for reason in &top.reasons {
            output.push_str(&format!("  + {}\n", reason));
        }
        for warning in &top.warnings {
            output.push_str(&format!("  ! {}\n", warning));
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
            api_version: "orchestr8/v1".to_string(),
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
            },
            requirements: ResourceRequirements {
                cpu: "2".to_string(),
                memory: "4Gi".to_string(),
                storage: "20Gi".to_string(),
                gpu: None,
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
        let podman_score = result.scores.iter().find(|s| s.runtime == RuntimeKind::Podman).unwrap();
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
}
