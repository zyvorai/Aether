//! Workload profiler
//!
//! Classifies workloads, tracks resource usage patterns, detects waste,
//! and provides right-sizing recommendations.

use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use serde::{Deserialize, Serialize};

/// Workload profile based on analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadProfile {
    pub name: String,
    pub classification: WorkloadType,
    pub resource_analysis: ResourceAnalysis,
    pub recommendations: Vec<Recommendation>,
    pub optimization_score: f64,
}

/// Workload type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkloadType {
    /// Stateless web service or API
    WebService,
    /// Stateful application with persistence
    Database,
    /// Background worker or queue consumer
    Worker,
    /// GPU-accelerated compute
    GpuCompute,
    /// Batch processing job
    BatchJob,
    /// Microservice in a service mesh
    Microservice,
    /// Development/testing workload
    Development,
}

impl std::fmt::Display for WorkloadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkloadType::WebService => write!(f, "Web Service"),
            WorkloadType::Database => write!(f, "Database"),
            WorkloadType::Worker => write!(f, "Worker"),
            WorkloadType::GpuCompute => write!(f, "GPU Compute"),
            WorkloadType::BatchJob => write!(f, "Batch Job"),
            WorkloadType::Microservice => write!(f, "Microservice"),
            WorkloadType::Development => write!(f, "Development"),
        }
    }
}

/// Resource usage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnalysis {
    pub cpu_requested: f64,
    pub cpu_estimated_usage: f64,
    pub cpu_efficiency: f64,
    pub memory_requested_gi: f64,
    pub memory_estimated_usage_gi: f64,
    pub memory_efficiency: f64,
    pub storage_requested_gi: f64,
    pub storage_estimated_usage_gi: f64,
    pub storage_efficiency: f64,
    pub overall_efficiency: f64,
    pub waste_detected: bool,
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub category: RecommendationCategory,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub estimated_savings_pct: f64,
    pub action: String,
}

/// Recommendation category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationCategory {
    RightSizing,
    RuntimeSelection,
    CostOptimization,
    Performance,
    Reliability,
    Security,
}

impl std::fmt::Display for RecommendationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecommendationCategory::RightSizing => write!(f, "Right-Sizing"),
            RecommendationCategory::RuntimeSelection => write!(f, "Runtime"),
            RecommendationCategory::CostOptimization => write!(f, "Cost"),
            RecommendationCategory::Performance => write!(f, "Performance"),
            RecommendationCategory::Reliability => write!(f, "Reliability"),
            RecommendationCategory::Security => write!(f, "Security"),
        }
    }
}

/// Recommendation priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Critical => write!(f, "CRITICAL"),
            Priority::High => write!(f, "HIGH"),
            Priority::Medium => write!(f, "MEDIUM"),
            Priority::Low => write!(f, "LOW"),
        }
    }
}

/// Workload profiler
pub struct Profiler {
    waste_threshold: f64,
}

impl Profiler {
    pub fn new(waste_threshold: f64) -> Self {
        Self { waste_threshold }
    }

    pub fn with_defaults() -> Self {
        Self::new(0.50)
    }

    /// Profile a workload and generate recommendations
    pub fn profile(&self, spec: &Workload, runtime: Option<RuntimeKind>) -> WorkloadProfile {
        let classification = self.classify(spec);
        let resource_analysis = self.analyze_resources(spec, &classification);
        let mut recommendations = Vec::new();

        // Right-sizing recommendations
        self.check_right_sizing(spec, &resource_analysis, &mut recommendations);

        // Runtime recommendations
        self.check_runtime(spec, runtime, &classification, &mut recommendations);

        // Reliability recommendations
        self.check_reliability(spec, &mut recommendations);

        // Performance recommendations
        self.check_performance(spec, &classification, &mut recommendations);

        // Security recommendations
        self.check_security(spec, &mut recommendations);

        // Sort by priority
        recommendations.sort_by(|a, b| a.priority.cmp(&b.priority));

        let optimization_score = self.calculate_optimization_score(&resource_analysis, &recommendations);

        WorkloadProfile {
            name: spec.metadata.name.clone(),
            classification,
            resource_analysis,
            recommendations,
            optimization_score,
        }
    }

    /// Classify workload type from its spec
    fn classify(&self, spec: &Workload) -> WorkloadType {
        // GPU workloads
        if spec.requirements.gpu.is_some() {
            return WorkloadType::GpuCompute;
        }

        // Database-like: persistent, low port count
        if spec.persistence.enabled && spec.network.ports.len() <= 2 {
            return WorkloadType::Database;
        }

        // Web service: has ingress, service, and health probes
        let has_ingress = spec.ingress.as_ref().is_some_and(|i| i.enabled);
        if has_ingress && spec.network.service && spec.health.is_some() {
            return WorkloadType::WebService;
        }

        // Microservice: has service but no ingress (internal)
        if spec.network.service && !has_ingress && spec.health.is_some() {
            return WorkloadType::Microservice;
        }

        // Worker: no service, no ingress
        if !spec.network.service && !has_ingress {
            return WorkloadType::Worker;
        }

        // Development: small resources, no health checks, no scaling
        let cpu = crate::resources::parse_cpu(&spec.requirements.cpu);
        if cpu <= 1.0 && spec.health.is_none() && spec.scaling.is_none() {
            return WorkloadType::Development;
        }

        WorkloadType::WebService
    }

    /// Analyze resource allocation and estimate efficiency
    fn analyze_resources(&self, spec: &Workload, classification: &WorkloadType) -> ResourceAnalysis {
        let cpu = crate::resources::parse_cpu(&spec.requirements.cpu);
        let memory_gi = crate::resources::parse_memory_gi(&spec.requirements.memory);
        let storage_gi = crate::resources::parse_memory_gi(&spec.requirements.storage);

        // Estimate typical utilization based on workload type
        let (cpu_util, mem_util, storage_util) = match classification {
            WorkloadType::WebService => (0.40, 0.60, 0.30),
            WorkloadType::Database => (0.30, 0.70, 0.60),
            WorkloadType::Worker => (0.60, 0.50, 0.20),
            WorkloadType::GpuCompute => (0.70, 0.60, 0.40),
            WorkloadType::BatchJob => (0.80, 0.50, 0.30),
            WorkloadType::Microservice => (0.35, 0.55, 0.25),
            WorkloadType::Development => (0.15, 0.30, 0.10),
        };

        let cpu_estimated = cpu * cpu_util;
        let memory_estimated = memory_gi * mem_util;
        let storage_estimated = storage_gi * storage_util;

        let overall_efficiency = (cpu_util + mem_util + storage_util) / 3.0;

        ResourceAnalysis {
            cpu_requested: cpu,
            cpu_estimated_usage: cpu_estimated,
            cpu_efficiency: cpu_util,
            memory_requested_gi: memory_gi,
            memory_estimated_usage_gi: memory_estimated,
            memory_efficiency: mem_util,
            storage_requested_gi: storage_gi,
            storage_estimated_usage_gi: storage_estimated,
            storage_efficiency: storage_util,
            overall_efficiency,
            waste_detected: overall_efficiency < self.waste_threshold,
        }
    }

    /// Check for right-sizing opportunities
    fn check_right_sizing(
        &self,
        spec: &Workload,
        analysis: &ResourceAnalysis,
        recommendations: &mut Vec<Recommendation>,
    ) {
        // Over-provisioned CPU
        if analysis.cpu_efficiency < 0.30 && analysis.cpu_requested > 1.0 {
            let suggested = (analysis.cpu_estimated_usage * 1.5).ceil();
            recommendations.push(Recommendation {
                category: RecommendationCategory::RightSizing,
                priority: Priority::High,
                title: "CPU over-provisioned".to_string(),
                description: format!(
                    "Estimated CPU usage {:.1} cores ({:.0}% of {:.1} requested)",
                    analysis.cpu_estimated_usage,
                    analysis.cpu_efficiency * 100.0,
                    analysis.cpu_requested
                ),
                estimated_savings_pct: (1.0 - analysis.cpu_efficiency) * 40.0,
                action: format!("Reduce CPU to {:.0} cores", suggested),
            });
        }

        // Over-provisioned memory
        if analysis.memory_efficiency < 0.40 && analysis.memory_requested_gi > 2.0 {
            let suggested = (analysis.memory_estimated_usage_gi * 1.3).ceil();
            recommendations.push(Recommendation {
                category: RecommendationCategory::RightSizing,
                priority: Priority::High,
                title: "Memory over-provisioned".to_string(),
                description: format!(
                    "Estimated memory usage {:.1}Gi ({:.0}% of {:.1}Gi requested)",
                    analysis.memory_estimated_usage_gi,
                    analysis.memory_efficiency * 100.0,
                    analysis.memory_requested_gi
                ),
                estimated_savings_pct: (1.0 - analysis.memory_efficiency) * 30.0,
                action: format!("Reduce memory to {:.0}Gi", suggested),
            });
        }

        // Over-provisioned storage
        if analysis.storage_efficiency < 0.20 && analysis.storage_requested_gi > 10.0 {
            recommendations.push(Recommendation {
                category: RecommendationCategory::RightSizing,
                priority: Priority::Medium,
                title: "Storage over-provisioned".to_string(),
                description: format!(
                    "Estimated storage usage {:.1}Gi ({:.0}% of {:.1}Gi requested)",
                    analysis.storage_estimated_usage_gi,
                    analysis.storage_efficiency * 100.0,
                    analysis.storage_requested_gi
                ),
                estimated_savings_pct: (1.0 - analysis.storage_efficiency) * 10.0,
                action: format!(
                    "Reduce storage to {:.0}Gi",
                    (analysis.storage_estimated_usage_gi * 2.0).ceil()
                ),
            });
        }

        // Under-provisioned (potential issues)
        if analysis.cpu_efficiency > 0.85 {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Performance,
                priority: Priority::High,
                title: "CPU may be under-provisioned".to_string(),
                description: format!(
                    "Estimated {:.0}% CPU utilization leaves little headroom",
                    analysis.cpu_efficiency * 100.0
                ),
                estimated_savings_pct: 0.0,
                action: format!(
                    "Consider increasing CPU to {:.0} cores",
                    (spec.requirements.cpu.parse::<f64>().unwrap_or(1.0) * 1.5).ceil()
                ),
            });
        }
    }

    /// Check runtime selection
    fn check_runtime(
        &self,
        spec: &Workload,
        current_runtime: Option<RuntimeKind>,
        classification: &WorkloadType,
        recommendations: &mut Vec<Recommendation>,
    ) {
        let current = match current_runtime {
            Some(r) => r,
            None => return,
        };

        // Development workload on Kubernetes
        if *classification == WorkloadType::Development && current == RuntimeKind::Kubernetes {
            recommendations.push(Recommendation {
                category: RecommendationCategory::RuntimeSelection,
                priority: Priority::Medium,
                title: "Development workload on Kubernetes".to_string(),
                description: "Small workload without production features may be cheaper locally".to_string(),
                estimated_savings_pct: 60.0,
                action: "Consider migrating to Podman for development".to_string(),
            });
        }

        // GPU workload not on KubeVirt or Metal3
        if *classification == WorkloadType::GpuCompute
            && current != RuntimeKind::KubeVirt
            && current != RuntimeKind::Metal3
        {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Performance,
                priority: Priority::High,
                title: "GPU workload on non-optimal runtime".to_string(),
                description: "GPU passthrough works best with VM isolation or bare metal".to_string(),
                estimated_savings_pct: 0.0,
                action: "Consider migrating to KubeVirt for GPU passthrough".to_string(),
            });
        }

        // Stateful on Podman
        if *classification == WorkloadType::Database && current == RuntimeKind::Podman {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Reliability,
                priority: Priority::High,
                title: "Database running on local container".to_string(),
                description: "Persistent workloads benefit from Kubernetes PVC and replication".to_string(),
                estimated_savings_pct: 0.0,
                action: "Consider migrating to Kubernetes for persistent volume support".to_string(),
            });
        }

        // Web service without scaling on K8s
        if *classification == WorkloadType::WebService
            && current == RuntimeKind::Kubernetes
            && spec.scaling.is_none()
        {
            recommendations.push(Recommendation {
                category: RecommendationCategory::CostOptimization,
                priority: Priority::Low,
                title: "Web service without auto-scaling".to_string(),
                description: "Auto-scaling can optimize cost during low-traffic periods".to_string(),
                estimated_savings_pct: 20.0,
                action: "Add scaling configuration with HPA".to_string(),
            });
        }
    }

    /// Check reliability configuration
    fn check_reliability(&self, spec: &Workload, recommendations: &mut Vec<Recommendation>) {
        if spec.health.is_none() {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Reliability,
                priority: Priority::High,
                title: "No health probes configured".to_string(),
                description: "Health probes enable automatic restart on failure".to_string(),
                estimated_savings_pct: 0.0,
                action: "Add liveness and readiness probes to the spec".to_string(),
            });
        } else if let Some(health) = spec.health.as_ref() {
            if health.readiness.is_none() {
                recommendations.push(Recommendation {
                    category: RecommendationCategory::Reliability,
                    priority: Priority::Medium,
                    title: "No readiness probe configured".to_string(),
                    description: "Readiness probes prevent traffic to unhealthy instances".to_string(),
                    estimated_savings_pct: 0.0,
                    action: "Add a readiness probe to the health spec".to_string(),
                });
            }
        }
    }

    /// Check performance configuration
    fn check_performance(
        &self,
        spec: &Workload,
        classification: &WorkloadType,
        recommendations: &mut Vec<Recommendation>,
    ) {
        // Web services should have ingress for external access
        if *classification == WorkloadType::WebService
            && !spec.ingress.as_ref().is_some_and(|i| i.enabled)
        {
            recommendations.push(Recommendation {
                category: RecommendationCategory::Performance,
                priority: Priority::Low,
                title: "Web service without ingress".to_string(),
                description: "Ingress provides TLS termination and path-based routing".to_string(),
                estimated_savings_pct: 0.0,
                action: "Configure ingress with TLS for production".to_string(),
            });
        }
    }

    /// Check security configuration
    fn check_security(&self, spec: &Workload, recommendations: &mut Vec<Recommendation>) {
        // Check for ingress without TLS
        if let Some(ingress) = &spec.ingress {
            if ingress.enabled && !ingress.tls {
                recommendations.push(Recommendation {
                    category: RecommendationCategory::Security,
                    priority: Priority::Critical,
                    title: "Ingress without TLS".to_string(),
                    description: "Traffic is unencrypted, vulnerable to interception".to_string(),
                    estimated_savings_pct: 0.0,
                    action: "Enable TLS on the ingress configuration".to_string(),
                });
            }
        }
    }

    fn calculate_optimization_score(
        &self,
        analysis: &ResourceAnalysis,
        recommendations: &[Recommendation],
    ) -> f64 {
        let mut score = analysis.overall_efficiency * 100.0;

        // Deduct points for issues
        for rec in recommendations {
            match rec.priority {
                Priority::Critical => score -= 20.0,
                Priority::High => score -= 10.0,
                Priority::Medium => score -= 5.0,
                Priority::Low => score -= 2.0,
            }
        }

        score.clamp(0.0, 100.0)
    }

}

/// Format workload profile as a report
pub fn format_profile_report(profile: &WorkloadProfile) -> String {
    let mut output = String::new();

    output.push_str(&format!("Workload: {}\n", profile.name));
    output.push_str(&format!("Type: {}\n", profile.classification));
    output.push_str(&format!(
        "Optimization Score: {:.0}/100\n\n",
        profile.optimization_score
    ));

    output.push_str("Resource Analysis:\n");
    output.push_str(&format!(
        "  CPU:     {:.1} cores requested, ~{:.1} estimated usage ({:.0}% efficiency)\n",
        profile.resource_analysis.cpu_requested,
        profile.resource_analysis.cpu_estimated_usage,
        profile.resource_analysis.cpu_efficiency * 100.0,
    ));
    output.push_str(&format!(
        "  Memory:  {:.1}Gi requested, ~{:.1}Gi estimated usage ({:.0}% efficiency)\n",
        profile.resource_analysis.memory_requested_gi,
        profile.resource_analysis.memory_estimated_usage_gi,
        profile.resource_analysis.memory_efficiency * 100.0,
    ));
    output.push_str(&format!(
        "  Storage: {:.1}Gi requested, ~{:.1}Gi estimated usage ({:.0}% efficiency)\n",
        profile.resource_analysis.storage_requested_gi,
        profile.resource_analysis.storage_estimated_usage_gi,
        profile.resource_analysis.storage_efficiency * 100.0,
    ));

    if profile.resource_analysis.waste_detected {
        output.push_str("\n  ** Resource waste detected **\n");
    }

    if !profile.recommendations.is_empty() {
        output.push_str(&format!(
            "\nRecommendations ({}):\n",
            profile.recommendations.len()
        ));
        for rec in &profile.recommendations {
            output.push_str(&format!(
                "\n  [{}] {} - {}\n",
                rec.priority, rec.category, rec.title
            ));
            output.push_str(&format!("    {}\n", rec.description));
            output.push_str(&format!("    Action: {}\n", rec.action));
            if rec.estimated_savings_pct > 0.0 {
                output.push_str(&format!(
                    "    Estimated savings: {:.0}%\n",
                    rec.estimated_savings_pct
                ));
            }
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

    fn create_test_workload() -> Workload {
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
                cpu: "4".to_string(),
                memory: "8Gi".to_string(),
                storage: "50Gi".to_string(),
                gpu: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Container, RuntimeType::Kube],
            },
            network: NetworkSpec {
                service: true,
                service_type: ServiceType::ClusterIP,
                ports: vec![PortMapping {
                    container_port: 80,
                    service_port: 8080,
                    protocol: "TCP".to_string(),
                }],
            },
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
        }
    }

    fn make_health_spec(with_readiness: bool) -> HealthSpec {
        HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::HttpGet {
                    path: "/health".to_string(),
                    port: 80,
                },
                initial_delay_seconds: 10,
                period_seconds: 10,
            }),
            readiness: if with_readiness {
                Some(HealthProbe {
                    probe_type: ProbeType::HttpGet {
                        path: "/ready".to_string(),
                        port: 80,
                    },
                    initial_delay_seconds: 5,
                    period_seconds: 5,
                })
            } else {
                None
            },
        }
    }

    fn make_ingress(tls: bool) -> IngressSpec {
        IngressSpec {
            enabled: true,
            host: "example.com".to_string(),
            paths: vec![],
            tls,
            annotations: HashMap::new(),
        }
    }

    // ── Original tests (preserved) ─────────────────────────────────────

    #[test]
    fn test_classify_web_service() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = Some(make_ingress(true));
        spec.health = Some(make_health_spec(false));
        assert_eq!(profiler.classify(&spec), WorkloadType::WebService);
    }

    #[test]
    fn test_classify_database() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.network.ports = vec![PortMapping {
            container_port: 5432,
            service_port: 5432,
            protocol: "TCP".to_string(),
        }];
        assert_eq!(profiler.classify(&spec), WorkloadType::Database);
    }

    #[test]
    fn test_profile_generates_recommendations() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let profile = profiler.profile(&spec, Some(RuntimeKind::Kubernetes));
        assert!(!profile.recommendations.is_empty());
    }

    #[test]
    fn test_optimization_score() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let profile = profiler.profile(&spec, None);
        assert!(profile.optimization_score >= 0.0 && profile.optimization_score <= 100.0);
    }

    #[test]
    fn test_format_report() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let profile = profiler.profile(&spec, None);
        let report = format_profile_report(&profile);
        assert!(report.contains("test-app"));
        assert!(report.contains("Resource Analysis"));
    }

    // ── Classification: all WorkloadType variants ──────────────────────

    #[test]
    fn test_classify_gpu_compute() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.gpu = Some(GpuRequirements {
            count: 2,
            vendor: "nvidia".to_string(),
        });
        assert_eq!(profiler.classify(&spec), WorkloadType::GpuCompute);
    }

    #[test]
    fn test_classify_gpu_takes_precedence_over_everything() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        // Set up conditions that would otherwise match Database
        spec.persistence.enabled = true;
        spec.network.ports = vec![PortMapping {
            container_port: 5432,
            service_port: 5432,
            protocol: "TCP".to_string(),
        }];
        // But also add GPU
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "amd".to_string(),
        });
        // GPU should win
        assert_eq!(profiler.classify(&spec), WorkloadType::GpuCompute);
    }

    #[test]
    fn test_classify_worker() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        // Worker: no service, no ingress
        spec.network.service = false;
        spec.ingress = None;
        assert_eq!(profiler.classify(&spec), WorkloadType::Worker);
    }

    #[test]
    fn test_classify_microservice() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        // Microservice: has service, no ingress, has health
        spec.network.service = true;
        spec.ingress = None;
        spec.health = Some(make_health_spec(true));
        assert_eq!(profiler.classify(&spec), WorkloadType::Microservice);
    }

    #[test]
    fn test_classify_database_with_two_ports() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.network.ports = vec![
            PortMapping {
                container_port: 5432,
                service_port: 5432,
                protocol: "TCP".to_string(),
            },
            PortMapping {
                container_port: 9090,
                service_port: 9090,
                protocol: "TCP".to_string(),
            },
        ];
        // 2 ports <= 2, persistence enabled => Database
        assert_eq!(profiler.classify(&spec), WorkloadType::Database);
    }

    #[test]
    fn test_classify_not_database_with_many_ports() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        // More than 2 ports should prevent Database classification
        spec.network.ports = vec![
            PortMapping { container_port: 5432, service_port: 5432, protocol: "TCP".to_string() },
            PortMapping { container_port: 9090, service_port: 9090, protocol: "TCP".to_string() },
            PortMapping { container_port: 8080, service_port: 8080, protocol: "TCP".to_string() },
        ];
        let result = profiler.classify(&spec);
        assert_ne!(result, WorkloadType::Database);
    }

    #[test]
    fn test_classify_web_service_fallback() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        // Has service, has ingress, but no health check
        // Does NOT match the WebService branch (requires health)
        // but service + ingress without health => falls to WebService default
        spec.network.service = true;
        spec.ingress = Some(make_ingress(true));
        spec.health = None;
        spec.requirements.cpu = "4".to_string(); // > 1.0
        // Falls through to the final WebService default
        assert_eq!(profiler.classify(&spec), WorkloadType::WebService);
    }

    // ── Profiler construction ──────────────────────────────────────────

    #[test]
    fn test_profiler_with_defaults() {
        let profiler = Profiler::with_defaults();
        assert!((profiler.waste_threshold - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn test_profiler_custom_threshold() {
        let profiler = Profiler::new(0.30);
        assert!((profiler.waste_threshold - 0.30).abs() < f64::EPSILON);
    }

    // ── CPU parsing ────────────────────────────────────────────────────

    #[test]
    fn test_parse_cpu_whole_cores() {
        assert!((crate::resources::parse_cpu("4") - 4.0).abs() < f64::EPSILON);
        assert!((crate::resources::parse_cpu("1") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_millicores() {
        assert!((crate::resources::parse_cpu("500m") - 0.5).abs() < f64::EPSILON);
        assert!((crate::resources::parse_cpu("2000m") - 2.0).abs() < f64::EPSILON);
        assert!((crate::resources::parse_cpu("250m") - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_invalid() {
        assert!((crate::resources::parse_cpu("")).abs() < f64::EPSILON);
        assert!((crate::resources::parse_cpu("abc")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_fractional() {
        assert!((crate::resources::parse_cpu("0.5") - 0.5).abs() < f64::EPSILON);
        assert!((crate::resources::parse_cpu("1.5") - 1.5).abs() < f64::EPSILON);
    }

    // ── Memory parsing ─────────────────────────────────────────────────

    #[test]
    fn test_parse_memory_gi() {
        assert!((crate::resources::parse_memory_gi("8Gi") - 8.0).abs() < f64::EPSILON);
        assert!((crate::resources::parse_memory_gi("1Gi") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_memory_mi() {
        assert!((crate::resources::parse_memory_gi("1024Mi") - 1.0).abs() < f64::EPSILON);
        assert!((crate::resources::parse_memory_gi("512Mi") - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_memory_unknown_suffix() {
        assert!((crate::resources::parse_memory_gi("8GB")).abs() < f64::EPSILON);
        assert!((crate::resources::parse_memory_gi("abc")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_storage_delegates_to_memory() {
        assert!((crate::resources::parse_memory_gi("50Gi") - 50.0).abs() < f64::EPSILON);
        assert!((crate::resources::parse_memory_gi("512Mi") - 0.5).abs() < f64::EPSILON);
    }

    // ── Resource analysis ──────────────────────────────────────────────

    #[test]
    fn test_analyze_resources_web_service() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();

        let analysis = profiler.analyze_resources(&spec, &WorkloadType::WebService);
        assert!((analysis.cpu_requested - 4.0).abs() < f64::EPSILON);
        assert!((analysis.memory_requested_gi - 8.0).abs() < f64::EPSILON);
        assert!((analysis.storage_requested_gi - 50.0).abs() < f64::EPSILON);
        // WebService utilization: cpu=0.40, mem=0.60, storage=0.30
        assert!((analysis.cpu_efficiency - 0.40).abs() < f64::EPSILON);
        assert!((analysis.memory_efficiency - 0.60).abs() < f64::EPSILON);
        assert!((analysis.storage_efficiency - 0.30).abs() < f64::EPSILON);
    }

    #[test]
    fn test_analyze_resources_estimated_usage() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let analysis = profiler.analyze_resources(&spec, &WorkloadType::Worker);
        // Worker: cpu_util=0.60 => 4*0.60 = 2.4
        assert!((analysis.cpu_estimated_usage - 2.4).abs() < f64::EPSILON);
        // Worker: mem_util=0.50 => 8*0.50 = 4.0
        assert!((analysis.memory_estimated_usage_gi - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_analyze_resources_overall_efficiency() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        // Database: (0.30 + 0.70 + 0.60) / 3.0
        let analysis = profiler.analyze_resources(&spec, &WorkloadType::Database);
        let expected = (0.30 + 0.70 + 0.60) / 3.0;
        assert!((analysis.overall_efficiency - expected).abs() < 1e-10);
    }

    #[test]
    fn test_waste_detected_when_efficiency_below_threshold() {
        let profiler = Profiler::new(0.50);
        let spec = create_test_workload();
        // Development: (0.15 + 0.30 + 0.10) / 3.0 = 0.183 < 0.50
        let analysis = profiler.analyze_resources(&spec, &WorkloadType::Development);
        assert!(analysis.waste_detected);
    }

    #[test]
    fn test_no_waste_when_efficiency_above_threshold() {
        let profiler = Profiler::new(0.30);
        let spec = create_test_workload();
        // GpuCompute: (0.70 + 0.60 + 0.40) / 3.0 = 0.566 > 0.30
        let analysis = profiler.analyze_resources(&spec, &WorkloadType::GpuCompute);
        assert!(!analysis.waste_detected);
    }

    #[test]
    fn test_all_workload_types_have_utilization_values() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let types = [
            WorkloadType::WebService,
            WorkloadType::Database,
            WorkloadType::Worker,
            WorkloadType::GpuCompute,
            WorkloadType::BatchJob,
            WorkloadType::Microservice,
            WorkloadType::Development,
        ];
        for wtype in &types {
            let analysis = profiler.analyze_resources(&spec, wtype);
            assert!(analysis.cpu_efficiency > 0.0, "cpu_efficiency > 0 for {:?}", wtype);
            assert!(analysis.memory_efficiency > 0.0, "mem_efficiency > 0 for {:?}", wtype);
            assert!(analysis.storage_efficiency > 0.0, "storage_efficiency > 0 for {:?}", wtype);
            assert!(analysis.overall_efficiency > 0.0, "overall > 0 for {:?}", wtype);
        }
    }

    // ── Right-sizing recommendations ───────────────────────────────────

    #[test]
    fn test_right_sizing_cpu_over_provisioned() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.cpu = "8".to_string(); // > 1.0

        // Development has cpu_efficiency = 0.15 < 0.30
        let analysis = profiler.analyze_resources(&spec, &WorkloadType::Development);
        let mut recs = Vec::new();
        profiler.check_right_sizing(&spec, &analysis, &mut recs);

        let cpu_rec = recs.iter().find(|r| r.title.contains("CPU over-provisioned"));
        assert!(cpu_rec.is_some(), "Expected CPU over-provisioned recommendation");
        assert_eq!(cpu_rec.unwrap().category, RecommendationCategory::RightSizing);
        assert_eq!(cpu_rec.unwrap().priority, Priority::High);
    }

    #[test]
    fn test_right_sizing_memory_over_provisioned() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.memory = "16Gi".to_string(); // > 2.0Gi

        // Development has mem_efficiency = 0.30 < 0.40
        let analysis = profiler.analyze_resources(&spec, &WorkloadType::Development);
        let mut recs = Vec::new();
        profiler.check_right_sizing(&spec, &analysis, &mut recs);

        let mem_rec = recs.iter().find(|r| r.title.contains("Memory over-provisioned"));
        assert!(mem_rec.is_some(), "Expected Memory over-provisioned recommendation");
    }

    #[test]
    fn test_right_sizing_storage_over_provisioned() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.storage = "100Gi".to_string(); // > 10.0Gi

        // Development has storage_efficiency = 0.10 < 0.20
        let analysis = profiler.analyze_resources(&spec, &WorkloadType::Development);
        let mut recs = Vec::new();
        profiler.check_right_sizing(&spec, &analysis, &mut recs);

        let storage_rec = recs.iter().find(|r| r.title.contains("Storage over-provisioned"));
        assert!(storage_rec.is_some(), "Expected Storage over-provisioned recommendation");
        assert_eq!(storage_rec.unwrap().priority, Priority::Medium);
    }

    #[test]
    fn test_right_sizing_cpu_under_provisioned() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();

        // Create a custom analysis with cpu efficiency > 0.85
        let analysis = ResourceAnalysis {
            cpu_requested: 2.0,
            cpu_estimated_usage: 1.8,
            cpu_efficiency: 0.90,
            memory_requested_gi: 4.0,
            memory_estimated_usage_gi: 2.0,
            memory_efficiency: 0.50,
            storage_requested_gi: 10.0,
            storage_estimated_usage_gi: 3.0,
            storage_efficiency: 0.30,
            overall_efficiency: 0.50,
            waste_detected: false,
        };
        let mut recs = Vec::new();
        profiler.check_right_sizing(&spec, &analysis, &mut recs);

        let under = recs.iter().find(|r| r.title.contains("under-provisioned"));
        assert!(under.is_some(), "Expected CPU under-provisioned recommendation");
        assert_eq!(under.unwrap().category, RecommendationCategory::Performance);
    }

    #[test]
    fn test_no_right_sizing_for_well_sized() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();

        // Efficiency values that do NOT trigger any right-sizing
        let analysis = ResourceAnalysis {
            cpu_requested: 2.0,
            cpu_estimated_usage: 1.0,
            cpu_efficiency: 0.50,   // not < 0.30 and not > 0.85
            memory_requested_gi: 4.0,
            memory_estimated_usage_gi: 2.0,
            memory_efficiency: 0.50, // not < 0.40
            storage_requested_gi: 5.0,  // not > 10.0
            storage_estimated_usage_gi: 1.5,
            storage_efficiency: 0.30,
            overall_efficiency: 0.43,
            waste_detected: false,
        };
        let mut recs = Vec::new();
        profiler.check_right_sizing(&spec, &analysis, &mut recs);
        assert!(recs.is_empty(), "Expected no right-sizing recommendations");
    }

    // ── Runtime recommendations ────────────────────────────────────────

    #[test]
    fn test_runtime_no_recommendations_without_runtime() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let mut recs = Vec::new();
        profiler.check_runtime(&spec, None, &WorkloadType::Development, &mut recs);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_runtime_dev_on_kubernetes() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let mut recs = Vec::new();
        profiler.check_runtime(
            &spec,
            Some(RuntimeKind::Kubernetes),
            &WorkloadType::Development,
            &mut recs,
        );
        assert!(!recs.is_empty());
        assert!(recs[0].title.contains("Development workload on Kubernetes"));
        assert_eq!(recs[0].category, RecommendationCategory::RuntimeSelection);
    }

    #[test]
    fn test_runtime_gpu_on_podman() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let mut recs = Vec::new();
        profiler.check_runtime(
            &spec,
            Some(RuntimeKind::Podman),
            &WorkloadType::GpuCompute,
            &mut recs,
        );
        let gpu_rec = recs.iter().find(|r| r.title.contains("GPU workload"));
        assert!(gpu_rec.is_some());
    }

    #[test]
    fn test_runtime_gpu_on_kubevirt_no_warning() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let mut recs = Vec::new();
        profiler.check_runtime(
            &spec,
            Some(RuntimeKind::KubeVirt),
            &WorkloadType::GpuCompute,
            &mut recs,
        );
        let gpu_rec = recs.iter().find(|r| r.title.contains("GPU workload"));
        assert!(gpu_rec.is_none(), "KubeVirt should be fine for GPU");
    }

    #[test]
    fn test_runtime_gpu_on_metal3_no_warning() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let mut recs = Vec::new();
        profiler.check_runtime(
            &spec,
            Some(RuntimeKind::Metal3),
            &WorkloadType::GpuCompute,
            &mut recs,
        );
        let gpu_rec = recs.iter().find(|r| r.title.contains("GPU workload"));
        assert!(gpu_rec.is_none(), "Metal3 should be fine for GPU");
    }

    #[test]
    fn test_runtime_database_on_podman() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let mut recs = Vec::new();
        profiler.check_runtime(
            &spec,
            Some(RuntimeKind::Podman),
            &WorkloadType::Database,
            &mut recs,
        );
        let db_rec = recs.iter().find(|r| r.title.contains("Database running on local"));
        assert!(db_rec.is_some());
        assert_eq!(db_rec.unwrap().priority, Priority::High);
    }

    #[test]
    fn test_runtime_web_service_without_scaling_on_k8s() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.scaling = None;
        let mut recs = Vec::new();
        profiler.check_runtime(
            &spec,
            Some(RuntimeKind::Kubernetes),
            &WorkloadType::WebService,
            &mut recs,
        );
        let scaling_rec = recs.iter().find(|r| r.title.contains("auto-scaling"));
        assert!(scaling_rec.is_some());
        assert_eq!(scaling_rec.unwrap().priority, Priority::Low);
    }

    #[test]
    fn test_runtime_web_service_with_scaling_on_k8s() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 5,
            metrics: vec![],
        });
        let mut recs = Vec::new();
        profiler.check_runtime(
            &spec,
            Some(RuntimeKind::Kubernetes),
            &WorkloadType::WebService,
            &mut recs,
        );
        let scaling_rec = recs.iter().find(|r| r.title.contains("auto-scaling"));
        assert!(scaling_rec.is_none(), "Should not recommend scaling when already configured");
    }

    // ── Reliability recommendations ────────────────────────────────────

    #[test]
    fn test_reliability_no_health_probes() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.health = None;
        let mut recs = Vec::new();
        profiler.check_reliability(&spec, &mut recs);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].title.contains("No health probes"));
        assert_eq!(recs[0].priority, Priority::High);
    }

    #[test]
    fn test_reliability_no_readiness_probe() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.health = Some(make_health_spec(false)); // liveness only
        let mut recs = Vec::new();
        profiler.check_reliability(&spec, &mut recs);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].title.contains("No readiness probe"));
        assert_eq!(recs[0].priority, Priority::Medium);
    }

    #[test]
    fn test_reliability_full_health_spec_no_recommendations() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.health = Some(make_health_spec(true)); // both probes
        let mut recs = Vec::new();
        profiler.check_reliability(&spec, &mut recs);
        assert!(recs.is_empty());
    }

    // ── Security recommendations ───────────────────────────────────────

    #[test]
    fn test_security_ingress_without_tls() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = Some(make_ingress(false)); // TLS disabled
        let mut recs = Vec::new();
        profiler.check_security(&spec, &mut recs);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].title.contains("Ingress without TLS"));
        assert_eq!(recs[0].priority, Priority::Critical);
    }

    #[test]
    fn test_security_ingress_with_tls_no_warning() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = Some(make_ingress(true));
        let mut recs = Vec::new();
        profiler.check_security(&spec, &mut recs);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_security_no_ingress_no_warning() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = None;
        let mut recs = Vec::new();
        profiler.check_security(&spec, &mut recs);
        assert!(recs.is_empty());
    }

    // ── Performance recommendations ────────────────────────────────────

    #[test]
    fn test_performance_web_service_without_ingress() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = None;
        let mut recs = Vec::new();
        profiler.check_performance(&spec, &WorkloadType::WebService, &mut recs);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].title.contains("without ingress"));
    }

    #[test]
    fn test_performance_web_service_with_ingress_no_warning() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = Some(make_ingress(true));
        let mut recs = Vec::new();
        profiler.check_performance(&spec, &WorkloadType::WebService, &mut recs);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_performance_non_web_service_no_ingress_check() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = None;
        let mut recs = Vec::new();
        profiler.check_performance(&spec, &WorkloadType::Worker, &mut recs);
        assert!(recs.is_empty(), "Worker should not get ingress recommendation");
    }

    // ── Optimization score ─────────────────────────────────────────────

    #[test]
    fn test_optimization_score_clamped_to_zero() {
        let profiler = Profiler::with_defaults();
        let analysis = ResourceAnalysis {
            cpu_requested: 1.0,
            cpu_estimated_usage: 0.5,
            cpu_efficiency: 0.10,
            memory_requested_gi: 1.0,
            memory_estimated_usage_gi: 0.3,
            memory_efficiency: 0.10,
            storage_requested_gi: 1.0,
            storage_estimated_usage_gi: 0.1,
            storage_efficiency: 0.10,
            overall_efficiency: 0.10, // only 10 points from efficiency
            waste_detected: true,
        };
        let critical_recs: Vec<Recommendation> = (0..10)
            .map(|i| Recommendation {
                category: RecommendationCategory::Security,
                priority: Priority::Critical,
                title: format!("issue-{}", i),
                description: "desc".to_string(),
                estimated_savings_pct: 0.0,
                action: "fix".to_string(),
            })
            .collect();
        let score = profiler.calculate_optimization_score(&analysis, &critical_recs);
        assert!((score - 0.0).abs() < f64::EPSILON, "Score should be clamped to 0");
    }

    #[test]
    fn test_optimization_score_no_recommendations() {
        let profiler = Profiler::with_defaults();
        let analysis = ResourceAnalysis {
            cpu_requested: 2.0,
            cpu_estimated_usage: 1.2,
            cpu_efficiency: 0.60,
            memory_requested_gi: 4.0,
            memory_estimated_usage_gi: 2.8,
            memory_efficiency: 0.70,
            storage_requested_gi: 20.0,
            storage_estimated_usage_gi: 12.0,
            storage_efficiency: 0.60,
            overall_efficiency: 0.633,
            waste_detected: false,
        };
        let score = profiler.calculate_optimization_score(&analysis, &[]);
        // 0.633 * 100 = 63.3
        assert!((score - 63.3).abs() < 0.1);
    }

    #[test]
    fn test_optimization_score_deductions() {
        let profiler = Profiler::with_defaults();
        let analysis = ResourceAnalysis {
            cpu_requested: 2.0,
            cpu_estimated_usage: 1.0,
            cpu_efficiency: 0.50,
            memory_requested_gi: 4.0,
            memory_estimated_usage_gi: 2.0,
            memory_efficiency: 0.50,
            storage_requested_gi: 10.0,
            storage_estimated_usage_gi: 5.0,
            storage_efficiency: 0.50,
            overall_efficiency: 0.50,
            waste_detected: false,
        };
        // Base score: 50.0
        let recs = vec![
            Recommendation {
                category: RecommendationCategory::Security,
                priority: Priority::High, // -10
                title: "t".to_string(),
                description: "d".to_string(),
                estimated_savings_pct: 0.0,
                action: "a".to_string(),
            },
            Recommendation {
                category: RecommendationCategory::Security,
                priority: Priority::Medium, // -5
                title: "t2".to_string(),
                description: "d".to_string(),
                estimated_savings_pct: 0.0,
                action: "a".to_string(),
            },
            Recommendation {
                category: RecommendationCategory::Security,
                priority: Priority::Low, // -2
                title: "t3".to_string(),
                description: "d".to_string(),
                estimated_savings_pct: 0.0,
                action: "a".to_string(),
            },
        ];
        let score = profiler.calculate_optimization_score(&analysis, &recs);
        // 50 - 10 - 5 - 2 = 33
        assert!((score - 33.0).abs() < f64::EPSILON);
    }

    // ── Full profile integration ───────────────────────────────────────

    #[test]
    fn test_profile_name_matches_spec() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.metadata.name = "my-api".to_string();
        let profile = profiler.profile(&spec, None);
        assert_eq!(profile.name, "my-api");
    }

    #[test]
    fn test_profile_recommendations_sorted_by_priority() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.health = None;
        spec.ingress = Some(make_ingress(false));
        let profile = profiler.profile(&spec, Some(RuntimeKind::Kubernetes));

        // Check sort order: Critical < High < Medium < Low
        for window in profile.recommendations.windows(2) {
            assert!(window[0].priority <= window[1].priority);
        }
    }

    #[test]
    fn test_profile_gpu_workload() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.gpu = Some(GpuRequirements {
            count: 4,
            vendor: "nvidia".to_string(),
        });
        let profile = profiler.profile(&spec, Some(RuntimeKind::Podman));
        assert_eq!(profile.classification, WorkloadType::GpuCompute);
        // Should recommend migrating away from Podman for GPU
        let gpu_rec = profile.recommendations.iter().find(|r| r.title.contains("GPU"));
        assert!(gpu_rec.is_some());
    }

    #[test]
    fn test_profile_well_configured_service() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = Some(make_ingress(true));
        spec.health = Some(make_health_spec(true));
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 2,
            max_replicas: 10,
            metrics: vec![],
        });
        let profile = profiler.profile(&spec, Some(RuntimeKind::Kubernetes));
        assert!(
            profile.optimization_score > 20.0,
            "Well-configured service should have decent score, got {}",
            profile.optimization_score
        );
    }

    // ── Edge cases: minimal / empty / extreme specs ────────────────────

    #[test]
    fn test_minimal_resource_spec() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.cpu = "100m".to_string();
        spec.requirements.memory = "128Mi".to_string();
        spec.requirements.storage = "256Mi".to_string();
        let profile = profiler.profile(&spec, None);
        assert!(profile.resource_analysis.cpu_requested < 1.0);
        assert!(profile.resource_analysis.memory_requested_gi < 1.0);
        assert!(profile.resource_analysis.storage_requested_gi < 1.0);
    }

    #[test]
    fn test_zero_resource_spec() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.cpu = "0".to_string();
        spec.requirements.memory = "0Gi".to_string();
        spec.requirements.storage = "0Gi".to_string();
        let profile = profiler.profile(&spec, None);
        assert!((profile.resource_analysis.cpu_requested).abs() < f64::EPSILON);
        assert!((profile.resource_analysis.memory_requested_gi).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extreme_resource_spec() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.cpu = "128".to_string();
        spec.requirements.memory = "1024Gi".to_string();
        spec.requirements.storage = "10000Gi".to_string();
        let profile = profiler.profile(&spec, None);
        assert!((profile.resource_analysis.cpu_requested - 128.0).abs() < f64::EPSILON);
        assert!((profile.resource_analysis.memory_requested_gi - 1024.0).abs() < f64::EPSILON);
        assert!((profile.resource_analysis.storage_requested_gi - 10000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_empty_cpu_string() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.cpu = "".to_string();
        let profile = profiler.profile(&spec, None);
        assert!((profile.resource_analysis.cpu_requested).abs() < f64::EPSILON);
    }

    #[test]
    fn test_invalid_memory_string() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.requirements.memory = "not-a-number".to_string();
        let profile = profiler.profile(&spec, None);
        assert!((profile.resource_analysis.memory_requested_gi).abs() < f64::EPSILON);
    }

    // ── Display trait coverage ─────────────────────────────────────────

    #[test]
    fn test_workload_type_display() {
        assert_eq!(format!("{}", WorkloadType::WebService), "Web Service");
        assert_eq!(format!("{}", WorkloadType::Database), "Database");
        assert_eq!(format!("{}", WorkloadType::Worker), "Worker");
        assert_eq!(format!("{}", WorkloadType::GpuCompute), "GPU Compute");
        assert_eq!(format!("{}", WorkloadType::BatchJob), "Batch Job");
        assert_eq!(format!("{}", WorkloadType::Microservice), "Microservice");
        assert_eq!(format!("{}", WorkloadType::Development), "Development");
    }

    #[test]
    fn test_recommendation_category_display() {
        assert_eq!(format!("{}", RecommendationCategory::RightSizing), "Right-Sizing");
        assert_eq!(format!("{}", RecommendationCategory::RuntimeSelection), "Runtime");
        assert_eq!(format!("{}", RecommendationCategory::CostOptimization), "Cost");
        assert_eq!(format!("{}", RecommendationCategory::Performance), "Performance");
        assert_eq!(format!("{}", RecommendationCategory::Reliability), "Reliability");
        assert_eq!(format!("{}", RecommendationCategory::Security), "Security");
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(format!("{}", Priority::Critical), "CRITICAL");
        assert_eq!(format!("{}", Priority::High), "HIGH");
        assert_eq!(format!("{}", Priority::Medium), "MEDIUM");
        assert_eq!(format!("{}", Priority::Low), "LOW");
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Medium);
        assert!(Priority::Medium < Priority::Low);
    }

    // ── Format report ──────────────────────────────────────────────────

    #[test]
    fn test_format_report_contains_cpu_info() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let profile = profiler.profile(&spec, None);
        let report = format_profile_report(&profile);
        assert!(report.contains("CPU:"));
        assert!(report.contains("Memory:"));
        assert!(report.contains("Storage:"));
    }

    #[test]
    fn test_format_report_waste_detected_message() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        // Force Development classification for low efficiency
        spec.network.service = false;
        spec.ingress = None;
        spec.requirements.cpu = "500m".to_string();
        spec.health = None;
        spec.scaling = None;
        let profile = profiler.profile(&spec, None);
        let report = format_profile_report(&profile);
        if profile.resource_analysis.waste_detected {
            assert!(report.contains("waste detected"));
        }
    }

    #[test]
    fn test_format_report_shows_recommendations() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.health = None;
        let profile = profiler.profile(&spec, None);
        let report = format_profile_report(&profile);
        assert!(report.contains("Recommendations"));
        assert!(report.contains("Action:"));
    }

    #[test]
    fn test_format_report_shows_optimization_score() {
        let profiler = Profiler::with_defaults();
        let spec = create_test_workload();
        let profile = profiler.profile(&spec, None);
        let report = format_profile_report(&profile);
        assert!(report.contains("Optimization Score:"));
        assert!(report.contains("/100"));
    }

    #[test]
    fn test_format_report_shows_savings_when_present() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        // Development on K8s should give savings recommendation
        spec.network.service = false;
        spec.ingress = None;
        spec.requirements.cpu = "500m".to_string();
        spec.health = None;
        spec.scaling = None;
        let profile = profiler.profile(&spec, Some(RuntimeKind::Kubernetes));
        let report = format_profile_report(&profile);
        let has_savings = profile.recommendations.iter().any(|r| r.estimated_savings_pct > 0.0);
        if has_savings {
            assert!(report.contains("Estimated savings:"));
        }
    }
}
