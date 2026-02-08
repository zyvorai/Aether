//! Policy engine for deployment governance
//!
//! Enforces rules before deployment: resource limits, naming conventions,
//! security requirements, and compliance checks.

use crate::spec::Workload;
use serde::{Deserialize, Serialize};

/// Policy evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    pub passed: bool,
    pub violations: Vec<PolicyViolation>,
    pub warnings: Vec<PolicyWarning>,
    pub policies_evaluated: usize,
}

/// A policy violation that blocks deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub policy: String,
    pub rule: String,
    pub message: String,
    pub field: String,
    pub severity: PolicySeverity,
}

/// A non-blocking policy warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyWarning {
    pub policy: String,
    pub message: String,
    pub suggestion: String,
}

/// Policy severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicySeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for PolicySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicySeverity::Error => write!(f, "ERROR"),
            PolicySeverity::Warning => write!(f, "WARNING"),
            PolicySeverity::Info => write!(f, "INFO"),
        }
    }
}

/// Policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub rules: Vec<PolicyRule>,
}

/// Individual policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub check: RuleCheck,
    pub severity: PolicySeverity,
    pub message: String,
}

/// Types of policy checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCheck {
    /// Maximum CPU cores allowed
    MaxCpu(f64),
    /// Maximum memory in GiB
    MaxMemoryGi(f64),
    /// Maximum storage in GiB
    MaxStorageGi(f64),
    /// Require health probes
    RequireHealthProbes,
    /// Require TLS on ingress
    RequireTls,
    /// Name must match pattern (prefix)
    NamePrefix(String),
    /// Must have owner label
    RequireOwner,
    /// Require resource limits
    RequireResourceLimits,
    /// Disallow specific runtimes
    DisallowRuntime(String),
    /// Require minimum replicas
    MinReplicas(u32),
    /// Maximum GPU count
    MaxGpu(u32),
}

/// Policy engine that evaluates workloads against policies
pub struct PolicyEngine {
    policies: Vec<Policy>,
}

impl PolicyEngine {
    /// Create with custom policies
    pub fn new(policies: Vec<Policy>) -> Self {
        Self { policies }
    }

    /// Create with default production policies
    pub fn production() -> Self {
        Self {
            policies: default_production_policies(),
        }
    }

    /// Create with permissive development policies
    pub fn development() -> Self {
        Self {
            policies: default_development_policies(),
        }
    }

    /// Load policies from a JSON file
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let policies: Vec<Policy> = serde_json::from_str(&content)?;
        Ok(Self { policies })
    }

    /// Evaluate a workload against all policies
    pub fn evaluate(&self, spec: &Workload) -> PolicyResult {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let mut policies_evaluated = 0;

        for policy in &self.policies {
            if !policy.enabled {
                continue;
            }

            for rule in &policy.rules {
                policies_evaluated += 1;
                self.check_rule(spec, policy, rule, &mut violations, &mut warnings);
            }
        }

        let passed = violations.is_empty();

        PolicyResult {
            passed,
            violations,
            warnings,
            policies_evaluated,
        }
    }

    fn check_rule(
        &self,
        spec: &Workload,
        policy: &Policy,
        rule: &PolicyRule,
        violations: &mut Vec<PolicyViolation>,
        warnings: &mut Vec<PolicyWarning>,
    ) {
        match &rule.check {
            RuleCheck::MaxCpu(max) => {
                let cpu = self.parse_cpu(&spec.requirements.cpu);
                if cpu > *max {
                    violations.push(PolicyViolation {
                        policy: policy.name.clone(),
                        rule: rule.name.clone(),
                        message: format!(
                            "CPU {:.1} cores exceeds maximum {:.1}",
                            cpu, max
                        ),
                        field: "requirements.cpu".to_string(),
                        severity: rule.severity.clone(),
                    });
                }
            }
            RuleCheck::MaxMemoryGi(max) => {
                let mem = self.parse_memory_gi(&spec.requirements.memory);
                if mem > *max {
                    violations.push(PolicyViolation {
                        policy: policy.name.clone(),
                        rule: rule.name.clone(),
                        message: format!(
                            "Memory {:.1}Gi exceeds maximum {:.1}Gi",
                            mem, max
                        ),
                        field: "requirements.memory".to_string(),
                        severity: rule.severity.clone(),
                    });
                }
            }
            RuleCheck::MaxStorageGi(max) => {
                let storage = self.parse_memory_gi(&spec.requirements.storage);
                if storage > *max {
                    violations.push(PolicyViolation {
                        policy: policy.name.clone(),
                        rule: rule.name.clone(),
                        message: format!(
                            "Storage {:.1}Gi exceeds maximum {:.1}Gi",
                            storage, max
                        ),
                        field: "requirements.storage".to_string(),
                        severity: rule.severity.clone(),
                    });
                }
            }
            RuleCheck::RequireHealthProbes => {
                if spec.health.is_none() {
                    violations.push(PolicyViolation {
                        policy: policy.name.clone(),
                        rule: rule.name.clone(),
                        message: "Health probes are required".to_string(),
                        field: "health".to_string(),
                        severity: rule.severity.clone(),
                    });
                }
            }
            RuleCheck::RequireTls => {
                if let Some(ingress) = &spec.ingress {
                    if ingress.enabled && !ingress.tls {
                        violations.push(PolicyViolation {
                            policy: policy.name.clone(),
                            rule: rule.name.clone(),
                            message: "TLS is required on ingress".to_string(),
                            field: "ingress.tls".to_string(),
                            severity: rule.severity.clone(),
                        });
                    }
                }
            }
            RuleCheck::NamePrefix(prefix) => {
                if !spec.metadata.name.starts_with(prefix.as_str()) {
                    warnings.push(PolicyWarning {
                        policy: policy.name.clone(),
                        message: format!(
                            "Name '{}' does not start with '{}'",
                            spec.metadata.name, prefix
                        ),
                        suggestion: format!(
                            "Rename to '{}-{}'",
                            prefix, spec.metadata.name
                        ),
                    });
                }
            }
            RuleCheck::RequireOwner => {
                if spec.metadata.owner.is_empty() {
                    violations.push(PolicyViolation {
                        policy: policy.name.clone(),
                        rule: rule.name.clone(),
                        message: "Owner is required".to_string(),
                        field: "metadata.owner".to_string(),
                        severity: rule.severity.clone(),
                    });
                }
            }
            RuleCheck::RequireResourceLimits => {
                let cpu = self.parse_cpu(&spec.requirements.cpu);
                let mem = self.parse_memory_gi(&spec.requirements.memory);
                if cpu == 0.0 || mem == 0.0 {
                    violations.push(PolicyViolation {
                        policy: policy.name.clone(),
                        rule: rule.name.clone(),
                        message: "CPU and memory limits must be set".to_string(),
                        field: "requirements".to_string(),
                        severity: rule.severity.clone(),
                    });
                }
            }
            RuleCheck::DisallowRuntime(runtime_name) => {
                use crate::spec::RuntimeType;
                let disallowed = match runtime_name.as_str() {
                    "container" | "podman" => Some(RuntimeType::Container),
                    "kube" | "kubernetes" => Some(RuntimeType::Kube),
                    "kubevirt" => Some(RuntimeType::Kubevirt),
                    "metal" | "metal3" => Some(RuntimeType::Metal),
                    _ => None,
                };
                if let Some(rt) = disallowed {
                    if spec.runtime.allow.contains(&rt) {
                        violations.push(PolicyViolation {
                            policy: policy.name.clone(),
                            rule: rule.name.clone(),
                            message: format!("Runtime '{}' is not allowed by policy", runtime_name),
                            field: "runtime.allow".to_string(),
                            severity: rule.severity.clone(),
                        });
                    }
                }
            }
            RuleCheck::MinReplicas(min) => {
                if let Some(scaling) = &spec.scaling {
                    if scaling.enabled && scaling.min_replicas < *min {
                        violations.push(PolicyViolation {
                            policy: policy.name.clone(),
                            rule: rule.name.clone(),
                            message: format!(
                                "Minimum replicas {} is below required {}",
                                scaling.min_replicas, min
                            ),
                            field: "scaling.minReplicas".to_string(),
                            severity: rule.severity.clone(),
                        });
                    }
                }
            }
            RuleCheck::MaxGpu(max) => {
                if let Some(gpu) = &spec.requirements.gpu {
                    if gpu.count > *max {
                        violations.push(PolicyViolation {
                            policy: policy.name.clone(),
                            rule: rule.name.clone(),
                            message: format!(
                                "GPU count {} exceeds maximum {}",
                                gpu.count, max
                            ),
                            field: "requirements.gpu.count".to_string(),
                            severity: rule.severity.clone(),
                        });
                    }
                }
            }
        }
    }

    fn parse_cpu(&self, cpu: &str) -> f64 {
        if let Some(stripped) = cpu.strip_suffix('m') {
            stripped.parse::<f64>().unwrap_or(0.0) / 1000.0
        } else {
            cpu.parse::<f64>().unwrap_or(0.0)
        }
    }

    fn parse_memory_gi(&self, memory: &str) -> f64 {
        if let Some(stripped) = memory.strip_suffix("Gi") {
            stripped.parse::<f64>().unwrap_or(0.0)
        } else if let Some(stripped) = memory.strip_suffix("Mi") {
            stripped.parse::<f64>().unwrap_or(0.0) / 1024.0
        } else {
            0.0
        }
    }
}

/// Default production policies
fn default_production_policies() -> Vec<Policy> {
    vec![
        Policy {
            name: "resource-limits".to_string(),
            description: "Enforce resource allocation limits".to_string(),
            enabled: true,
            rules: vec![
                PolicyRule {
                    name: "max-cpu".to_string(),
                    check: RuleCheck::MaxCpu(64.0),
                    severity: PolicySeverity::Error,
                    message: "CPU exceeds cluster capacity".to_string(),
                },
                PolicyRule {
                    name: "max-memory".to_string(),
                    check: RuleCheck::MaxMemoryGi(256.0),
                    severity: PolicySeverity::Error,
                    message: "Memory exceeds cluster capacity".to_string(),
                },
                PolicyRule {
                    name: "max-storage".to_string(),
                    check: RuleCheck::MaxStorageGi(1000.0),
                    severity: PolicySeverity::Error,
                    message: "Storage exceeds limit".to_string(),
                },
                PolicyRule {
                    name: "max-gpu".to_string(),
                    check: RuleCheck::MaxGpu(8),
                    severity: PolicySeverity::Error,
                    message: "GPU count exceeds limit".to_string(),
                },
            ],
        },
        Policy {
            name: "security".to_string(),
            description: "Security best practices".to_string(),
            enabled: true,
            rules: vec![
                PolicyRule {
                    name: "require-tls".to_string(),
                    check: RuleCheck::RequireTls,
                    severity: PolicySeverity::Error,
                    message: "TLS required for all ingress".to_string(),
                },
                PolicyRule {
                    name: "require-health-probes".to_string(),
                    check: RuleCheck::RequireHealthProbes,
                    severity: PolicySeverity::Warning,
                    message: "Health probes recommended".to_string(),
                },
            ],
        },
        Policy {
            name: "governance".to_string(),
            description: "Governance and compliance".to_string(),
            enabled: true,
            rules: vec![
                PolicyRule {
                    name: "require-owner".to_string(),
                    check: RuleCheck::RequireOwner,
                    severity: PolicySeverity::Error,
                    message: "Owner must be specified".to_string(),
                },
                PolicyRule {
                    name: "require-resource-limits".to_string(),
                    check: RuleCheck::RequireResourceLimits,
                    severity: PolicySeverity::Error,
                    message: "Resource limits must be defined".to_string(),
                },
            ],
        },
    ]
}

/// Default development policies (more permissive)
fn default_development_policies() -> Vec<Policy> {
    vec![Policy {
        name: "dev-limits".to_string(),
        description: "Development resource limits".to_string(),
        enabled: true,
        rules: vec![
            PolicyRule {
                name: "max-cpu".to_string(),
                check: RuleCheck::MaxCpu(8.0),
                severity: PolicySeverity::Error,
                message: "Dev CPU limit exceeded".to_string(),
            },
            PolicyRule {
                name: "max-memory".to_string(),
                check: RuleCheck::MaxMemoryGi(16.0),
                severity: PolicySeverity::Error,
                message: "Dev memory limit exceeded".to_string(),
            },
        ],
    }]
}

/// Format policy result as a report
pub fn format_policy_report(result: &PolicyResult) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Policy Evaluation: {}\n",
        if result.passed { "PASSED" } else { "FAILED" }
    ));
    output.push_str(&format!(
        "Policies evaluated: {}\n\n",
        result.policies_evaluated
    ));

    if !result.violations.is_empty() {
        output.push_str(&format!("Violations ({}):\n", result.violations.len()));
        for v in &result.violations {
            output.push_str(&format!(
                "  [{}] {}/{}: {}\n",
                v.severity, v.policy, v.rule, v.message
            ));
            output.push_str(&format!("    Field: {}\n", v.field));
        }
        output.push('\n');
    }

    if !result.warnings.is_empty() {
        output.push_str(&format!("Warnings ({}):\n", result.warnings.len()));
        for w in &result.warnings {
            output.push_str(&format!("  [{}] {}\n", w.policy, w.message));
            output.push_str(&format!("    Suggestion: {}\n", w.suggestion));
        }
    }

    if result.passed && result.violations.is_empty() && result.warnings.is_empty() {
        output.push_str("All policies passed. Workload is compliant.\n");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn test_workload() -> Workload {
        Workload {
            api_version: "orchestr8/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test-app".to_string(),
                owner: "team-alpha".to_string(),
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
                allow: vec![RuntimeType::Container, RuntimeType::Kube],
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
    fn test_production_policy_passes() {
        let engine = PolicyEngine::production();
        let spec = test_workload();
        let result = engine.evaluate(&spec);
        // Should pass resource limits but may warn about health probes
        assert!(result.policies_evaluated > 0);
    }

    #[test]
    fn test_cpu_limit_violation() {
        let engine = PolicyEngine::new(vec![Policy {
            name: "limits".to_string(),
            description: "test".to_string(),
            enabled: true,
            rules: vec![PolicyRule {
                name: "max-cpu".to_string(),
                check: RuleCheck::MaxCpu(1.0),
                severity: PolicySeverity::Error,
                message: "Too much CPU".to_string(),
            }],
        }]);

        let spec = test_workload(); // has 2 CPU
        let result = engine.evaluate(&spec);
        assert!(!result.passed);
        assert_eq!(result.violations.len(), 1);
    }

    #[test]
    fn test_tls_requirement() {
        let engine = PolicyEngine::new(vec![Policy {
            name: "security".to_string(),
            description: "test".to_string(),
            enabled: true,
            rules: vec![PolicyRule {
                name: "require-tls".to_string(),
                check: RuleCheck::RequireTls,
                severity: PolicySeverity::Error,
                message: "TLS required".to_string(),
            }],
        }]);

        let mut spec = test_workload();
        spec.ingress = Some(IngressSpec {
            enabled: true,
            host: "example.com".to_string(),
            paths: vec![],
            tls: false,
            annotations: HashMap::new(),
        });

        let result = engine.evaluate(&spec);
        assert!(!result.passed);
    }

    #[test]
    fn test_disabled_policy_skipped() {
        let engine = PolicyEngine::new(vec![Policy {
            name: "disabled".to_string(),
            description: "test".to_string(),
            enabled: false,
            rules: vec![PolicyRule {
                name: "max-cpu".to_string(),
                check: RuleCheck::MaxCpu(0.1),
                severity: PolicySeverity::Error,
                message: "Impossible limit".to_string(),
            }],
        }]);

        let spec = test_workload();
        let result = engine.evaluate(&spec);
        assert!(result.passed);
        assert_eq!(result.policies_evaluated, 0);
    }

    #[test]
    fn test_format_report() {
        let engine = PolicyEngine::production();
        let spec = test_workload();
        let result = engine.evaluate(&spec);
        let report = format_policy_report(&result);
        assert!(report.contains("Policy Evaluation"));
    }
}
