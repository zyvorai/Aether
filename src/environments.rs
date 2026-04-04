//! Environment management
//!
//! Multi-environment (dev/staging/prod) management with promotion
//! workflows and environment parity validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::output;

/// Environment tier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EnvTier {
    Development,
    Staging,
    Production,
    Custom(String),
}

impl std::fmt::Display for EnvTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvTier::Development => write!(f, "development"),
            EnvTier::Staging => write!(f, "staging"),
            EnvTier::Production => write!(f, "production"),
            EnvTier::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl std::str::FromStr for EnvTier {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "development" | "dev" => EnvTier::Development,
            "staging" | "stg" => EnvTier::Staging,
            "production" | "prod" => EnvTier::Production,
            other => EnvTier::Custom(other.to_string()),
        })
    }
}

/// Environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub tier: EnvTier,
    pub workloads: HashMap<String, EnvWorkload>,
    pub variables: HashMap<String, String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Workload configuration within an environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvWorkload {
    pub name: String,
    pub spec_path: PathBuf,
    pub runtime_override: Option<String>,
    pub replicas: Option<u32>,
    pub cpu_override: Option<String>,
    pub memory_override: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub deployed: bool,
    pub version: String,
}

/// Promotion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionRequest {
    pub workload: String,
    pub from_env: String,
    pub to_env: String,
    pub strategy: PromotionStrategy,
    pub require_approval: bool,
}

/// Promotion strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromotionStrategy {
    /// Copy spec as-is
    Direct,
    /// Adjust resources for target tier
    TierAdjusted,
    /// Use canary deployment in target
    Canary,
}

impl std::fmt::Display for PromotionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromotionStrategy::Direct => write!(f, "direct"),
            PromotionStrategy::TierAdjusted => write!(f, "tier-adjusted"),
            PromotionStrategy::Canary => write!(f, "canary"),
        }
    }
}

/// Promotion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionResult {
    pub success: bool,
    pub workload: String,
    pub from_env: String,
    pub to_env: String,
    pub changes: Vec<PromotionChange>,
    pub warnings: Vec<String>,
}

/// A change applied during promotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionChange {
    pub field: String,
    pub from_value: String,
    pub to_value: String,
    pub reason: String,
}

/// Parity check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityReport {
    pub env_a: String,
    pub env_b: String,
    pub workload: String,
    pub in_sync: bool,
    pub diffs: Vec<ParityDiff>,
}

/// A difference between environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityDiff {
    pub field: String,
    pub env_a_value: String,
    pub env_b_value: String,
    pub severity: ParitySeverity,
}

/// Parity diff severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParitySeverity {
    /// Expected difference (e.g., replicas differ between dev/prod)
    Expected,
    /// Unexpected difference worth investigating
    Unexpected,
    /// Critical difference that could cause issues
    Critical,
}

impl std::fmt::Display for ParitySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParitySeverity::Expected => write!(f, "expected"),
            ParitySeverity::Unexpected => write!(f, "unexpected"),
            ParitySeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Environment manager
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentManager {
    environments: HashMap<String, Environment>,
}

crate::impl_json_store!(EnvironmentManager, "environments.json");

impl EnvironmentManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new environment
    pub fn create_env(&mut self, name: &str, tier: EnvTier) -> &Environment {
        let now = crate::resources::now_rfc3339();
        let env = Environment {
            name: name.to_string(),
            tier,
            workloads: HashMap::new(),
            variables: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        self.environments.insert(name.to_string(), env);
        self.environments.get(name).expect("just inserted")
    }

    /// Get an environment
    pub fn get_env(&self, name: &str) -> Option<&Environment> {
        self.environments.get(name)
    }

    /// List all environments
    pub fn list_envs(&self) -> Vec<&Environment> {
        let mut envs: Vec<&Environment> = self.environments.values().collect();
        envs.sort_by(|a, b| a.name.cmp(&b.name));
        envs
    }

    /// Add a workload to an environment
    pub fn add_workload(
        &mut self,
        env_name: &str,
        workload: EnvWorkload,
    ) -> anyhow::Result<()> {
        let env = self
            .environments
            .get_mut(env_name)
            .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", env_name))?;
        env.workloads.insert(workload.name.clone(), workload);
        env.updated_at = crate::resources::now_rfc3339();
        Ok(())
    }

    /// Set an environment variable
    pub fn set_var(
        &mut self,
        env_name: &str,
        key: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        let env = self
            .environments
            .get_mut(env_name)
            .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", env_name))?;
        env.variables.insert(key.to_string(), value.to_string());
        env.updated_at = crate::resources::now_rfc3339();
        Ok(())
    }

    /// Promote a workload from one environment to another
    pub fn promote(&mut self, request: &PromotionRequest) -> anyhow::Result<PromotionResult> {
        let source = self
            .environments
            .get(&request.from_env)
            .ok_or_else(|| anyhow::anyhow!("Source environment '{}' not found", request.from_env))?;

        let source_workload = source
            .workloads
            .get(&request.workload)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Workload '{}' not found in '{}'",
                    request.workload,
                    request.from_env
                )
            })?
            .clone();

        let source_tier = source.tier.clone();

        let target = self
            .environments
            .get(&request.to_env)
            .ok_or_else(|| anyhow::anyhow!("Target environment '{}' not found", request.to_env))?;
        let target_tier = target.tier.clone();

        let mut changes = Vec::new();
        let mut warnings = Vec::new();
        let mut promoted = source_workload.clone();

        // Apply tier adjustments
        if request.strategy == PromotionStrategy::TierAdjusted {
            self.apply_tier_adjustments(
                &mut promoted,
                &source_tier,
                &target_tier,
                &mut changes,
                &mut warnings,
            );
        }

        // Bump version
        let new_version = Self::bump_version(&promoted.version);
        changes.push(PromotionChange {
            field: "version".to_string(),
            from_value: promoted.version.clone(),
            to_value: new_version.clone(),
            reason: "Version bump for promotion".to_string(),
        });
        promoted.version = new_version;
        promoted.deployed = false;

        // Check for potential issues
        if target_tier == EnvTier::Production && !source_workload.deployed {
            warnings.push("Promoting undeployed workload to production".to_string());
        }

        // Insert into target
        let target = self.environments.get_mut(&request.to_env)
            .ok_or_else(|| anyhow::anyhow!("Target environment '{}' disappeared during promotion", request.to_env))?;
        target.workloads.insert(request.workload.clone(), promoted);
        target.updated_at = crate::resources::now_rfc3339();

        Ok(PromotionResult {
            success: true,
            workload: request.workload.clone(),
            from_env: request.from_env.clone(),
            to_env: request.to_env.clone(),
            changes,
            warnings,
        })
    }

    /// Check parity between two environments for a workload
    pub fn check_parity(
        &self,
        env_a_name: &str,
        env_b_name: &str,
        workload: &str,
    ) -> anyhow::Result<ParityReport> {
        let env_a = self
            .environments
            .get(env_a_name)
            .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", env_a_name))?;
        let env_b = self
            .environments
            .get(env_b_name)
            .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", env_b_name))?;

        let wl_a = env_a.workloads.get(workload);
        let wl_b = env_b.workloads.get(workload);

        let mut diffs = Vec::new();

        match (wl_a, wl_b) {
            (Some(a), Some(b)) => {
                // Compare versions
                if a.version != b.version {
                    diffs.push(ParityDiff {
                        field: "version".to_string(),
                        env_a_value: a.version.clone(),
                        env_b_value: b.version.clone(),
                        severity: if env_a.tier != env_b.tier {
                            ParitySeverity::Expected
                        } else {
                            ParitySeverity::Unexpected
                        },
                    });
                }

                // Compare runtime
                if a.runtime_override != b.runtime_override {
                    diffs.push(ParityDiff {
                        field: "runtime".to_string(),
                        env_a_value: a.runtime_override.clone().unwrap_or_default(),
                        env_b_value: b.runtime_override.clone().unwrap_or_default(),
                        severity: ParitySeverity::Unexpected,
                    });
                }

                // Compare replicas
                if a.replicas != b.replicas {
                    diffs.push(ParityDiff {
                        field: "replicas".to_string(),
                        env_a_value: a.replicas.map(|r| r.to_string()).unwrap_or_default(),
                        env_b_value: b.replicas.map(|r| r.to_string()).unwrap_or_default(),
                        severity: ParitySeverity::Expected,
                    });
                }

                // Compare CPU
                if a.cpu_override != b.cpu_override {
                    diffs.push(ParityDiff {
                        field: "cpu".to_string(),
                        env_a_value: a.cpu_override.clone().unwrap_or_default(),
                        env_b_value: b.cpu_override.clone().unwrap_or_default(),
                        severity: ParitySeverity::Expected,
                    });
                }

                // Compare memory
                if a.memory_override != b.memory_override {
                    diffs.push(ParityDiff {
                        field: "memory".to_string(),
                        env_a_value: a.memory_override.clone().unwrap_or_default(),
                        env_b_value: b.memory_override.clone().unwrap_or_default(),
                        severity: ParitySeverity::Expected,
                    });
                }

                // Compare env vars
                let all_keys: std::collections::HashSet<_> = a
                    .env_vars
                    .keys()
                    .chain(b.env_vars.keys())
                    .collect();
                for key in all_keys {
                    let val_a = a.env_vars.get(key).map(|s| s.as_str()).unwrap_or("");
                    let val_b = b.env_vars.get(key).map(|s| s.as_str()).unwrap_or("");
                    if val_a != val_b {
                        diffs.push(ParityDiff {
                            field: format!("env.{}", key),
                            env_a_value: val_a.to_string(),
                            env_b_value: val_b.to_string(),
                            severity: ParitySeverity::Unexpected,
                        });
                    }
                }
            }
            (Some(_), None) => {
                diffs.push(ParityDiff {
                    field: "workload".to_string(),
                    env_a_value: "present".to_string(),
                    env_b_value: "missing".to_string(),
                    severity: ParitySeverity::Critical,
                });
            }
            (None, Some(_)) => {
                diffs.push(ParityDiff {
                    field: "workload".to_string(),
                    env_a_value: "missing".to_string(),
                    env_b_value: "present".to_string(),
                    severity: ParitySeverity::Critical,
                });
            }
            (None, None) => {
                // Both environments are missing this workload — they are in sync
            }
        }

        Ok(ParityReport {
            env_a: env_a_name.to_string(),
            env_b: env_b_name.to_string(),
            workload: workload.to_string(),
            in_sync: diffs.is_empty(),
            diffs,
        })
    }

    // --- Private ---

    fn apply_tier_adjustments(
        &self,
        workload: &mut EnvWorkload,
        from_tier: &EnvTier,
        to_tier: &EnvTier,
        changes: &mut Vec<PromotionChange>,
        warnings: &mut Vec<String>,
    ) {
        let scale_factor = match (from_tier, to_tier) {
            (EnvTier::Development, EnvTier::Staging) => 2.0,
            (EnvTier::Development, EnvTier::Production) => 4.0,
            (EnvTier::Staging, EnvTier::Production) => 2.0,
            (EnvTier::Production, EnvTier::Staging) => 0.5,
            (EnvTier::Production, EnvTier::Development) => 0.25,
            (EnvTier::Staging, EnvTier::Development) => 0.5,
            _ => 1.0,
        };

        // Adjust replicas
        if let Some(replicas) = workload.replicas {
            let new_replicas = ((replicas as f64 * scale_factor).ceil() as u32).max(1);
            if new_replicas != replicas {
                changes.push(PromotionChange {
                    field: "replicas".to_string(),
                    from_value: replicas.to_string(),
                    to_value: new_replicas.to_string(),
                    reason: format!("Tier adjustment ({} -> {})", from_tier, to_tier),
                });
                workload.replicas = Some(new_replicas);
            }
        }

        // Warn about production readiness
        if *to_tier == EnvTier::Production
            && workload.replicas.unwrap_or(0) < 2
        {
            warnings.push("Production workload should have at least 2 replicas".to_string());
        }
    }

    fn bump_version(version: &str) -> String {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() == 3 {
            if let Ok(patch) = parts[2].parse::<u32>() {
                return format!("{}.{}.{}", parts[0], parts[1], patch + 1);
            }
        }
        // Avoid repeated "-promoted" suffixes.
        // Strip any existing "-promoted" suffix, then re-append with an incrementing counter.
        let base = version.strip_suffix("-promoted").unwrap_or(version);
        format!("{}-promoted", base)
    }
}

/// Format environment list
pub fn format_env_list(envs: &[&Environment]) -> String {
    let mut out = String::new();

    if envs.is_empty() {
        out.push_str("  No environments configured.\n");
        return out;
    }

    let rows: Vec<Vec<String>> = envs
        .iter()
        .map(|e| vec![
            e.name.clone(),
            format!("{}", e.tier),
            e.workloads.len().to_string(),
            e.variables.len().to_string(),
            e.updated_at[..19].to_string(),
        ])
        .collect();
    out.push_str(&format!(
        "\n{}\n",
        output::table(&["Environment", "Tier", "Workloads", "Variables", "Updated"], rows),
    ));

    out
}

/// Format parity report
pub fn format_parity_report(report: &ParityReport) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "Parity: {} vs {} for '{}'\n",
        report.env_a, report.env_b, report.workload
    ));
    output.push_str(&format!(
        "Status: {}\n\n",
        if report.in_sync {
            "IN SYNC"
        } else {
            "DIFFERENCES FOUND"
        }
    ));

    if report.diffs.is_empty() {
        output.push_str("  No differences detected.\n");
    } else {
        for diff in &report.diffs {
            output.push_str(&format!(
                "  [{}] {}: {} vs {}\n",
                diff.severity, diff.field, diff.env_a_value, diff.env_b_value
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_environment() {
        let mut mgr = EnvironmentManager::new();
        mgr.create_env("dev", EnvTier::Development);
        mgr.create_env("prod", EnvTier::Production);

        assert_eq!(mgr.list_envs().len(), 2);
        assert!(mgr.get_env("dev").is_some());
    }

    #[test]
    fn test_add_workload() {
        let mut mgr = EnvironmentManager::new();
        mgr.create_env("dev", EnvTier::Development);

        let wl = EnvWorkload {
            name: "web-app".to_string(),
            spec_path: PathBuf::from("workload.yaml"),
            runtime_override: None,
            replicas: Some(1),
            cpu_override: None,
            memory_override: None,
            env_vars: HashMap::new(),
            deployed: false,
            version: "1.0.0".to_string(),
        };

        mgr.add_workload("dev", wl).unwrap();
        let env = mgr.get_env("dev").unwrap();
        assert_eq!(env.workloads.len(), 1);
    }

    #[test]
    fn test_promote_direct() {
        let mut mgr = EnvironmentManager::new();
        mgr.create_env("dev", EnvTier::Development);
        mgr.create_env("staging", EnvTier::Staging);

        let wl = EnvWorkload {
            name: "api".to_string(),
            spec_path: PathBuf::from("api.yaml"),
            runtime_override: None,
            replicas: Some(1),
            cpu_override: None,
            memory_override: None,
            env_vars: HashMap::new(),
            deployed: true,
            version: "1.0.0".to_string(),
        };
        mgr.add_workload("dev", wl).unwrap();

        let result = mgr
            .promote(&PromotionRequest {
                workload: "api".to_string(),
                from_env: "dev".to_string(),
                to_env: "staging".to_string(),
                strategy: PromotionStrategy::Direct,
                require_approval: false,
            })
            .unwrap();

        assert!(result.success);
        assert!(mgr.get_env("staging").unwrap().workloads.contains_key("api"));
    }

    #[test]
    fn test_promote_tier_adjusted() {
        let mut mgr = EnvironmentManager::new();
        mgr.create_env("dev", EnvTier::Development);
        mgr.create_env("prod", EnvTier::Production);

        let wl = EnvWorkload {
            name: "api".to_string(),
            spec_path: PathBuf::from("api.yaml"),
            runtime_override: None,
            replicas: Some(1),
            cpu_override: None,
            memory_override: None,
            env_vars: HashMap::new(),
            deployed: true,
            version: "1.0.0".to_string(),
        };
        mgr.add_workload("dev", wl).unwrap();

        let result = mgr
            .promote(&PromotionRequest {
                workload: "api".to_string(),
                from_env: "dev".to_string(),
                to_env: "prod".to_string(),
                strategy: PromotionStrategy::TierAdjusted,
                require_approval: false,
            })
            .unwrap();

        assert!(result.success);
        // Replicas should be scaled up (1 * 4 = 4 for dev->prod)
        let prod_wl = &mgr.get_env("prod").unwrap().workloads["api"];
        assert_eq!(prod_wl.replicas, Some(4));
    }

    #[test]
    fn test_check_parity() {
        let mut mgr = EnvironmentManager::new();
        mgr.create_env("staging", EnvTier::Staging);
        mgr.create_env("prod", EnvTier::Production);

        let wl_staging = EnvWorkload {
            name: "api".to_string(),
            spec_path: PathBuf::from("api.yaml"),
            runtime_override: None,
            replicas: Some(2),
            cpu_override: Some("2".to_string()),
            memory_override: None,
            env_vars: HashMap::new(),
            deployed: true,
            version: "1.0.0".to_string(),
        };

        let wl_prod = EnvWorkload {
            name: "api".to_string(),
            spec_path: PathBuf::from("api.yaml"),
            runtime_override: None,
            replicas: Some(4),
            cpu_override: Some("4".to_string()),
            memory_override: None,
            env_vars: HashMap::new(),
            deployed: true,
            version: "1.0.1".to_string(),
        };

        mgr.add_workload("staging", wl_staging).unwrap();
        mgr.add_workload("prod", wl_prod).unwrap();

        let parity = mgr.check_parity("staging", "prod", "api").unwrap();
        assert!(!parity.in_sync);
        assert!(!parity.diffs.is_empty());
    }

    #[test]
    fn test_format_env_list() {
        let mut mgr = EnvironmentManager::new();
        mgr.create_env("dev", EnvTier::Development);
        let envs = mgr.list_envs();
        let output = format_env_list(&envs);
        assert!(output.contains("dev"));
        assert!(output.contains("development"));
    }

    #[test]
    fn test_version_bump() {
        assert_eq!(EnvironmentManager::bump_version("1.0.0"), "1.0.1");
        assert_eq!(EnvironmentManager::bump_version("2.3.5"), "2.3.6");
    }

    #[test]
    fn test_env_tier_from_str() {
        assert_eq!("development".parse::<EnvTier>().unwrap(), EnvTier::Development);
        assert_eq!("dev".parse::<EnvTier>().unwrap(), EnvTier::Development);
        assert_eq!("staging".parse::<EnvTier>().unwrap(), EnvTier::Staging);
        assert_eq!("stg".parse::<EnvTier>().unwrap(), EnvTier::Staging);
        assert_eq!("production".parse::<EnvTier>().unwrap(), EnvTier::Production);
        assert_eq!("prod".parse::<EnvTier>().unwrap(), EnvTier::Production);
        assert_eq!("custom-tier".parse::<EnvTier>().unwrap(), EnvTier::Custom("custom-tier".to_string()));
    }
}
