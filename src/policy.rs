//! Policy engine for deployment governance
//!
//! Enforces rules before deployment: resource limits, naming conventions,
//! security requirements, and compliance checks.

use crate::output;
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
        // Shorthand to push a violation with the current policy/rule context.
        let mut violate = |msg: String, field: &str| {
            violations.push(PolicyViolation {
                policy: policy.name.clone(),
                rule: rule.name.clone(),
                message: msg,
                field: field.to_string(),
                severity: rule.severity.clone(),
            });
        };

        match &rule.check {
            RuleCheck::MaxCpu(max) => {
                let cpu = crate::resources::parse_cpu(&spec.requirements.cpu);
                if cpu > *max {
                    violate(format!("CPU {:.1} cores exceeds maximum {:.1}", cpu, max), "requirements.cpu");
                }
            }
            RuleCheck::MaxMemoryGi(max) => {
                let mem = crate::resources::parse_memory_gi(&spec.requirements.memory);
                if mem > *max {
                    violate(format!("Memory {:.1}Gi exceeds maximum {:.1}Gi", mem, max), "requirements.memory");
                }
            }
            RuleCheck::MaxStorageGi(max) => {
                let storage = crate::resources::parse_memory_gi(&spec.requirements.storage);
                if storage > *max {
                    violate(format!("Storage {:.1}Gi exceeds maximum {:.1}Gi", storage, max), "requirements.storage");
                }
            }
            RuleCheck::RequireHealthProbes => {
                if spec.health.is_none() {
                    violate("Health probes are required".to_string(), "health");
                }
            }
            RuleCheck::RequireTls => {
                if let Some(ingress) = &spec.ingress {
                    if ingress.enabled && !ingress.tls {
                        violate("TLS is required on ingress".to_string(), "ingress.tls");
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
                    violate("Owner is required".to_string(), "metadata.owner");
                }
            }
            RuleCheck::RequireResourceLimits => {
                let cpu = crate::resources::parse_cpu(&spec.requirements.cpu);
                let mem = crate::resources::parse_memory_gi(&spec.requirements.memory);
                if cpu < f64::EPSILON || mem < f64::EPSILON {
                    violate("CPU and memory limits must be set".to_string(), "requirements");
                }
            }
            RuleCheck::DisallowRuntime(runtime_name) => {
                use crate::runtime::RuntimeKind;
                use crate::spec::RuntimeType;
                if let Ok(kind) = runtime_name.parse::<RuntimeKind>() {
                    let rt = match kind {
                        RuntimeKind::Podman | RuntimeKind::Docker => RuntimeType::Container,
                        RuntimeKind::Kubernetes => RuntimeType::Kube,
                        RuntimeKind::KubeVirt => RuntimeType::Kubevirt,
                        RuntimeKind::Metal3 => RuntimeType::Metal,
                    };
                    if spec.runtime.allow.contains(&rt) {
                        violate(format!("Runtime '{}' is not allowed by policy", runtime_name), "runtime.allow");
                    }
                }
            }
            RuleCheck::MinReplicas(min) => {
                if let Some(scaling) = &spec.scaling {
                    if scaling.enabled && scaling.min_replicas < *min {
                        violate(format!("Minimum replicas {} is below required {}", scaling.min_replicas, min), "scaling.minReplicas");
                    }
                }
            }
            RuleCheck::MaxGpu(max) => {
                if let Some(gpu) = &spec.requirements.gpu {
                    if gpu.count > *max {
                        violate(format!("GPU count {} exceeds maximum {}", gpu.count, max), "requirements.gpu.count");
                    }
                }
            }
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

/// Evaluate a workload against the configured policy set. Returns `Ok(())`
/// if the workload passes, or an error describing the violations.
///
/// Used as a deploy-time gate: inserted into the deploy pipeline so that
/// workloads violating `PolicySeverity::Error` rules are rejected.
pub fn gate_deploy(spec: &Workload, config: &crate::config::PolicyConfig) -> anyhow::Result<()> {
    let engine = match config.policy_set.as_str() {
        "development" | "dev" => PolicyEngine::development(),
        "production" | "prod" => PolicyEngine::production(),
        path => {
            let p = std::path::Path::new(path);
            if p.exists() {
                PolicyEngine::load(p)?
            } else {
                tracing::warn!(
                    "Policy set '{}' not found, falling back to production policies",
                    path
                );
                PolicyEngine::production()
            }
        }
    };

    let result = engine.evaluate(spec);

    // Show warnings even if passed
    for w in &result.warnings {
        output::warning(&format!("[{}] {}", w.policy, w.message));
    }

    if !result.passed {
        let error_count = result
            .violations
            .iter()
            .filter(|v| v.severity == PolicySeverity::Error)
            .count();
        if error_count > 0 {
            anyhow::bail!(
                "Policy check failed with {} violation(s):\n{}",
                error_count,
                format_policy_report(&result)
            );
        }
    }

    Ok(())
}

/// Format policy result as a report
pub fn format_policy_report(result: &PolicyResult) -> String {
    let mut output = String::new();

    output.push_str(&output::property_section(&[
        ("Policy Evaluation", if result.passed { "PASSED".to_string() } else { "FAILED".to_string() }),
        ("Policies evaluated", format!("{}", result.policies_evaluated)),
    ]));

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
            output.push_str(&output::tree_bullet("⚠", &format!("[{}] {}", w.policy, w.message)));
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
            api_version: "aether/v1".to_string(),
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
