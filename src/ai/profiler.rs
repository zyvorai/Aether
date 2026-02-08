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
        let cpu = self.parse_cpu(&spec.requirements.cpu);
        if cpu <= 1.0 && spec.health.is_none() && spec.scaling.is_none() {
            return WorkloadType::Development;
        }

        WorkloadType::WebService
    }

    /// Analyze resource allocation and estimate efficiency
    fn analyze_resources(&self, spec: &Workload, classification: &WorkloadType) -> ResourceAnalysis {
        let cpu = self.parse_cpu(&spec.requirements.cpu);
        let memory_gi = self.parse_memory_gi(&spec.requirements.memory);
        let storage_gi = self.parse_storage_gi(&spec.requirements.storage);

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
        } else {
            let health = spec.health.as_ref().unwrap();
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

    fn parse_cpu(&self, cpu: &str) -> f64 {
        if let Some(stripped) = cpu.strip_suffix('m') {
            stripped.parse::<f64>().unwrap_or(0.0) / 1000.0
        } else {
            cpu.parse::<f64>().unwrap_or(0.0)
        }
    }

    fn parse_memory_gi(&self, memory: &str) -> f64 {
        let memory = memory.trim();
        if let Some(stripped) = memory.strip_suffix("Gi") {
            stripped.parse::<f64>().unwrap_or(0.0)
        } else if let Some(stripped) = memory.strip_suffix("Mi") {
            stripped.parse::<f64>().unwrap_or(0.0) / 1024.0
        } else {
            0.0
        }
    }

    fn parse_storage_gi(&self, storage: &str) -> f64 {
        self.parse_memory_gi(storage)
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

    #[test]
    fn test_classify_web_service() {
        let profiler = Profiler::with_defaults();
        let mut spec = create_test_workload();
        spec.ingress = Some(IngressSpec {
            enabled: true,
            host: "example.com".to_string(),
            paths: vec![],
            tls: true,
            annotations: HashMap::new(),
        });
        spec.health = Some(HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::HttpGet {
                    path: "/health".to_string(),
                    port: 80,
                },
                initial_delay_seconds: 10,
                period_seconds: 10,
            }),
            readiness: None,
        });
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

        // Should have at least health probe recommendation
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
}
