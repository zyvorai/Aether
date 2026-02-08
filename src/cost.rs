//! Cost estimation for workload resources

use crate::spec::Workload;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Cloud provider for cost estimation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudProvider {
    AWS,
    Azure,
    GCP,
    DigitalOcean,
    Linode,
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudProvider::AWS => write!(f, "AWS"),
            CloudProvider::Azure => write!(f, "Azure"),
            CloudProvider::GCP => write!(f, "GCP"),
            CloudProvider::DigitalOcean => write!(f, "DigitalOcean"),
            CloudProvider::Linode => write!(f, "Linode"),
        }
    }
}

/// Cost breakdown for a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostEstimate {
    pub provider: CloudProvider,
    pub cpu_cost_monthly: f64,
    pub memory_cost_monthly: f64,
    pub storage_cost_monthly: f64,
    pub total_monthly: f64,
    pub total_hourly: f64,
    pub currency: String,
}

impl CostEstimate {
    /// Display cost estimate in formatted way
    pub fn display(&self) -> String {
        format!(
            "{} - ${:.2}/mo (${:.4}/hr)\n  CPU: ${:.2}/mo | Memory: ${:.2}/mo | Storage: ${:.2}/mo",
            self.provider,
            self.total_monthly,
            self.total_hourly,
            self.cpu_cost_monthly,
            self.memory_cost_monthly,
            self.storage_cost_monthly
        )
    }
}

/// Resource pricing for a cloud provider
#[derive(Debug, Clone)]
struct ProviderPricing {
    cpu_per_core_monthly: f64,      // $ per vCPU per month
    memory_per_gb_monthly: f64,     // $ per GB RAM per month
    storage_per_gb_monthly: f64,    // $ per GB storage per month
}

impl ProviderPricing {
    /// Get pricing for a specific provider
    fn for_provider(provider: CloudProvider) -> Self {
        match provider {
            CloudProvider::AWS => Self {
                // AWS EC2 t3.medium baseline pricing (us-east-1)
                cpu_per_core_monthly: 15.0,
                memory_per_gb_monthly: 4.0,
                storage_per_gb_monthly: 0.10, // EBS gp3
            },
            CloudProvider::Azure => Self {
                // Azure B2s baseline pricing (East US)
                cpu_per_core_monthly: 14.0,
                memory_per_gb_monthly: 3.5,
                storage_per_gb_monthly: 0.12, // Standard SSD
            },
            CloudProvider::GCP => Self {
                // GCP e2-medium baseline pricing (us-central1)
                cpu_per_core_monthly: 13.0,
                memory_per_gb_monthly: 3.75,
                storage_per_gb_monthly: 0.17, // SSD persistent disk
            },
            CloudProvider::DigitalOcean => Self {
                // DigitalOcean basic droplet pricing
                cpu_per_core_monthly: 6.0,
                memory_per_gb_monthly: 6.0,
                storage_per_gb_monthly: 0.15, // Block storage
            },
            CloudProvider::Linode => Self {
                // Linode shared CPU pricing
                cpu_per_core_monthly: 5.0,
                memory_per_gb_monthly: 5.0,
                storage_per_gb_monthly: 0.10, // Block storage
            },
        }
    }
}

/// Parse CPU requirement to cores
fn parse_cpu(cpu: &str) -> Result<f64> {
    let cpu = cpu.trim();

    if cpu.ends_with('m') {
        // Millicores (e.g., "1000m")
        let millicores: f64 = cpu
            .trim_end_matches('m')
            .parse()
            .context("Invalid millicores value")?;
        Ok(millicores / 1000.0)
    } else {
        // Cores (e.g., "2")
        cpu.parse().context("Invalid CPU value")
    }
}

/// Parse memory requirement to GB
fn parse_memory(memory: &str) -> Result<f64> {
    let memory = memory.trim();

    if memory.ends_with("Gi") {
        memory
            .trim_end_matches("Gi")
            .parse()
            .context("Invalid GiB value")
    } else if memory.ends_with("Mi") {
        let mib: f64 = memory
            .trim_end_matches("Mi")
            .parse()
            .context("Invalid MiB value")?;
        Ok(mib / 1024.0)
    } else if memory.ends_with("GB") {
        memory
            .trim_end_matches("GB")
            .parse()
            .context("Invalid GB value")
    } else if memory.ends_with("MB") {
        let mb: f64 = memory
            .trim_end_matches("MB")
            .parse()
            .context("Invalid MB value")?;
        Ok(mb / 1000.0)
    } else {
        Err(anyhow::anyhow!("Unknown memory format: {}", memory))
    }
}

/// Parse storage requirement to GB
fn parse_storage(storage: &str) -> Result<f64> {
    parse_memory(storage) // Same parsing logic as memory
}

/// Estimate cost for a workload
pub fn estimate_cost(workload: &Workload, provider: CloudProvider) -> Result<CostEstimate> {
    let pricing = ProviderPricing::for_provider(provider);

    // Parse resources
    let cpu_cores = parse_cpu(&workload.requirements.cpu)?;
    let memory_gb = parse_memory(&workload.requirements.memory)?;
    let storage_gb = parse_storage(&workload.requirements.storage)?;

    // Calculate costs
    let cpu_cost = cpu_cores * pricing.cpu_per_core_monthly;
    let memory_cost = memory_gb * pricing.memory_per_gb_monthly;
    let storage_cost = storage_gb * pricing.storage_per_gb_monthly;

    let total_monthly = cpu_cost + memory_cost + storage_cost;
    let total_hourly = total_monthly / 730.0; // Average hours per month

    Ok(CostEstimate {
        provider,
        cpu_cost_monthly: cpu_cost,
        memory_cost_monthly: memory_cost,
        storage_cost_monthly: storage_cost,
        total_monthly,
        total_hourly,
        currency: "USD".to_string(),
    })
}

/// Estimate costs across all providers
pub fn estimate_all_providers(workload: &Workload) -> Result<Vec<CostEstimate>> {
    let providers = vec![
        CloudProvider::AWS,
        CloudProvider::Azure,
        CloudProvider::GCP,
        CloudProvider::DigitalOcean,
        CloudProvider::Linode,
    ];

    let mut estimates = Vec::new();
    for provider in providers {
        estimates.push(estimate_cost(workload, provider)?);
    }

    // Sort by total monthly cost
    estimates.sort_by(|a, b| a.total_monthly.partial_cmp(&b.total_monthly).unwrap());

    Ok(estimates)
}

/// Cost comparison report
#[derive(Debug, Serialize, Deserialize)]
pub struct CostComparison {
    pub workload_name: String,
    pub cpu: String,
    pub memory: String,
    pub storage: String,
    pub estimates: Vec<CostEstimate>,
    pub cheapest: CloudProvider,
    pub most_expensive: CloudProvider,
    pub savings_percentage: f64,
}

impl CostComparison {
    /// Create cost comparison for a workload
    pub fn for_workload(workload: &Workload) -> Result<Self> {
        let estimates = estimate_all_providers(workload)?;

        let cheapest = estimates.first().unwrap().provider;
        let most_expensive = estimates.last().unwrap().provider;

        let cheapest_cost = estimates.first().unwrap().total_monthly;
        let expensive_cost = estimates.last().unwrap().total_monthly;

        let savings = ((expensive_cost - cheapest_cost) / expensive_cost) * 100.0;

        Ok(Self {
            workload_name: workload.metadata.name.clone(),
            cpu: workload.requirements.cpu.clone(),
            memory: workload.requirements.memory.clone(),
            storage: workload.requirements.storage.clone(),
            estimates,
            cheapest,
            most_expensive,
            savings_percentage: savings,
        })
    }

    /// Display comparison report
    pub fn display(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("📊 Cost Estimate for '{}'\n\n", self.workload_name));
        output.push_str("Resources:\n");
        output.push_str(&format!("  CPU: {}\n", self.cpu));
        output.push_str(&format!("  Memory: {}\n", self.memory));
        output.push_str(&format!("  Storage: {}\n\n", self.storage));

        output.push_str("Monthly Cost Estimates:\n\n");

        for (idx, estimate) in self.estimates.iter().enumerate() {
            let prefix = if idx == 0 {
                "✅ " // Cheapest
            } else if idx == self.estimates.len() - 1 {
                "💰 " // Most expensive
            } else {
                "   "
            };

            output.push_str(&format!("{}{}\n", prefix, estimate.display()));
        }

        output.push_str(&format!(
            "\n💡 Savings: {:.1}% by choosing {} over {}\n",
            self.savings_percentage, self.cheapest, self.most_expensive
        ));

        output
    }
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
    fn test_parse_cpu() {
        assert_eq!(parse_cpu("2").unwrap(), 2.0);
        assert_eq!(parse_cpu("1000m").unwrap(), 1.0);
        assert_eq!(parse_cpu("500m").unwrap(), 0.5);
    }

    #[test]
    fn test_parse_memory() {
        assert_eq!(parse_memory("4Gi").unwrap(), 4.0);
        assert_eq!(parse_memory("2048Mi").unwrap(), 2.0);
        assert_eq!(parse_memory("1024Mi").unwrap(), 1.0);
    }

    #[test]
    fn test_estimate_cost() {
        let workload = create_test_workload();

        let estimate = estimate_cost(&workload, CloudProvider::AWS).unwrap();
        assert!(estimate.total_monthly > 0.0);
        assert!(estimate.total_hourly > 0.0);
        assert_eq!(estimate.provider, CloudProvider::AWS);
    }

    #[test]
    fn test_estimate_all_providers() {
        let workload = create_test_workload();

        let estimates = estimate_all_providers(&workload).unwrap();
        assert_eq!(estimates.len(), 5);

        // Should be sorted by cost (ascending)
        for i in 1..estimates.len() {
            assert!(estimates[i].total_monthly >= estimates[i - 1].total_monthly);
        }
    }

    #[test]
    fn test_cost_comparison() {
        let workload = create_test_workload();

        let comparison = CostComparison::for_workload(&workload).unwrap();
        assert_eq!(comparison.workload_name, "test-app");
        assert!(comparison.savings_percentage >= 0.0);
        assert!(comparison.savings_percentage <= 100.0);

        let report = comparison.display();
        assert!(report.contains("test-app"));
        assert!(report.contains("Savings"));
    }
}
