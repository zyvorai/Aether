//! Cost estimation for workload resources

use crate::output;
use crate::spec::Workload;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

impl std::str::FromStr for CloudProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aws" => Ok(CloudProvider::AWS),
            "azure" => Ok(CloudProvider::Azure),
            "gcp" => Ok(CloudProvider::GCP),
            "digitalocean" | "do" => Ok(CloudProvider::DigitalOcean),
            "linode" => Ok(CloudProvider::Linode),
            _ => Err(anyhow::anyhow!("Unknown provider: '{}'. Valid: aws, azure, gcp, digitalocean, linode", s)),
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
#[derive(Debug, Clone, Copy)]
struct ProviderPricing {
    cpu_per_core_monthly: f64,      // $ per vCPU per month
    memory_per_gb_monthly: f64,     // $ per GB RAM per month
    storage_per_gb_monthly: f64,    // $ per GB storage per month
    gpu_per_unit_monthly: f64,      // $ per GPU per month
    egress_per_gb: f64,             // $ per GB network egress
    load_balancer_monthly: f64,     // $ per load balancer per month
}

impl ProviderPricing {
    /// Get pricing for a specific provider
    fn for_provider(provider: CloudProvider) -> Self {
        match provider {
            CloudProvider::AWS => Self {
                cpu_per_core_monthly: 15.0,
                memory_per_gb_monthly: 4.0,
                storage_per_gb_monthly: 0.10,
                gpu_per_unit_monthly: 350.0,  // p3.2xlarge equivalent
                egress_per_gb: 0.09,
                load_balancer_monthly: 22.50, // ALB
            },
            CloudProvider::Azure => Self {
                cpu_per_core_monthly: 14.0,
                memory_per_gb_monthly: 3.5,
                storage_per_gb_monthly: 0.12,
                gpu_per_unit_monthly: 320.0,  // NC-series equivalent
                egress_per_gb: 0.087,
                load_balancer_monthly: 25.00,
            },
            CloudProvider::GCP => Self {
                cpu_per_core_monthly: 13.0,
                memory_per_gb_monthly: 3.75,
                storage_per_gb_monthly: 0.17,
                gpu_per_unit_monthly: 300.0,  // T4 equivalent
                egress_per_gb: 0.12,
                load_balancer_monthly: 18.00,
            },
            CloudProvider::DigitalOcean => Self {
                cpu_per_core_monthly: 6.0,
                memory_per_gb_monthly: 6.0,
                storage_per_gb_monthly: 0.15,
                gpu_per_unit_monthly: 500.0,  // GPU droplet
                egress_per_gb: 0.01,
                load_balancer_monthly: 12.00,
            },
            CloudProvider::Linode => Self {
                cpu_per_core_monthly: 5.0,
                memory_per_gb_monthly: 5.0,
                storage_per_gb_monthly: 0.10,
                gpu_per_unit_monthly: 450.0,  // GPU instance
                egress_per_gb: 0.01,
                load_balancer_monthly: 10.00,
            },
        }
    }
}

/// Extended cost estimate with GPU, network, and load balancer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedCostEstimate {
    pub base: CostEstimate,
    pub gpu_cost_monthly: f64,
    pub network_cost_monthly: f64,
    pub load_balancer_cost_monthly: f64,
    pub total_monthly: f64,
    pub total_hourly: f64,
    pub recommendations: Vec<CostRecommendation>,
}

/// Cost optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecommendation {
    pub category: String,
    pub title: String,
    pub description: String,
    pub estimated_savings_monthly: f64,
    pub estimated_savings_pct: f64,
}

/// Estimate extended costs including GPU, network, and load balancer
pub fn estimate_extended_cost(
    workload: &Workload,
    provider: CloudProvider,
    egress_gb: f64,
    include_lb: bool,
) -> Result<ExtendedCostEstimate> {
    let pricing = ProviderPricing::for_provider(provider);
    let base = estimate_cost(workload, provider)?;

    // GPU costs
    let gpu_cost = workload
        .requirements
        .gpu
        .as_ref()
        .map(|g| g.count as f64 * pricing.gpu_per_unit_monthly)
        .unwrap_or(0.0);

    // Network egress
    let network_cost = egress_gb * pricing.egress_per_gb;

    // Load balancer
    let lb_cost = if include_lb {
        pricing.load_balancer_monthly
    } else {
        0.0
    };

    let total_monthly = base.total_monthly + gpu_cost + network_cost + lb_cost;
    let total_hourly = total_monthly / 730.0;

    // Generate recommendations
    let recommendations = generate_cost_recommendations(
        workload, &base, gpu_cost, network_cost, lb_cost, &pricing,
    );

    Ok(ExtendedCostEstimate {
        base,
        gpu_cost_monthly: gpu_cost,
        network_cost_monthly: network_cost,
        load_balancer_cost_monthly: lb_cost,
        total_monthly,
        total_hourly,
        recommendations,
    })
}

/// Generate cost optimization recommendations
fn generate_cost_recommendations(
    workload: &Workload,
    base: &CostEstimate,
    gpu_cost: f64,
    _network_cost: f64,
    _lb_cost: f64,
    _pricing: &ProviderPricing,
) -> Vec<CostRecommendation> {
    let mut recs = Vec::new();

    // Reserved instance recommendation for stable workloads
    if base.total_monthly > 50.0 {
        recs.push(CostRecommendation {
            category: "Reserved Instances".to_string(),
            title: "Consider reserved instances".to_string(),
            description: "1-year commitment can save 30-40% on compute costs".to_string(),
            estimated_savings_monthly: base.total_monthly * 0.35,
            estimated_savings_pct: 35.0,
        });
    }

    // GPU optimization
    if gpu_cost > 0.0 {
        recs.push(CostRecommendation {
            category: "GPU".to_string(),
            title: "GPU spot instances".to_string(),
            description: "Use preemptible/spot GPU instances for non-critical workloads".to_string(),
            estimated_savings_monthly: gpu_cost * 0.60,
            estimated_savings_pct: 60.0,
        });
    }

    // Right-sizing
    let cpu = match parse_cpu(&workload.requirements.cpu) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "Failed to parse CPU '{}' for cost recommendation: {}; skipping right-sizing check",
                workload.requirements.cpu, e
            );
            0.0
        }
    };
    if cpu >= 8.0 {
        recs.push(CostRecommendation {
            category: "Right-Sizing".to_string(),
            title: "Evaluate CPU allocation".to_string(),
            description: format!(
                "{:.0} cores requested — monitor actual usage to avoid over-provisioning",
                cpu
            ),
            estimated_savings_monthly: base.cpu_cost_monthly * 0.20,
            estimated_savings_pct: 20.0,
        });
    }

    recs
}

/// Parse CPU requirement to cores
/// Parse CPU string to fractional cores, delegating to shared utility.
fn parse_cpu(cpu: &str) -> Result<f64> {
    Ok(crate::resources::parse_cpu(cpu))
}

/// Parse memory/storage string to GiB, delegating to shared utility.
fn parse_memory(mem: &str) -> Result<f64> {
    Ok(crate::resources::parse_memory_gi(mem))
}

/// Parse storage requirement to GiB (same logic as memory).
fn parse_storage(storage: &str) -> Result<f64> {
    parse_memory(storage)
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
    estimates.sort_by(|a, b| {
        a.total_monthly.partial_cmp(&b.total_monthly).unwrap_or_else(|| {
            // Push NaN values to the end
            if a.total_monthly.is_nan() { std::cmp::Ordering::Greater }
            else { std::cmp::Ordering::Less }
        })
    });

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

        let cheapest = estimates.first().map(|e| e.provider).unwrap_or(CloudProvider::AWS);
        let most_expensive = estimates.last().map(|e| e.provider).unwrap_or(CloudProvider::AWS);

        let cheapest_cost = estimates.first().map(|e| e.total_monthly).unwrap_or(0.0);
        let expensive_cost = estimates.last().map(|e| e.total_monthly).unwrap_or(0.0);

        let savings = if expensive_cost > 0.0 {
            ((expensive_cost - cheapest_cost) / expensive_cost) * 100.0
        } else {
            0.0
        };

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

        output.push_str(&output::property_section(&[
            ("Cost Estimate", self.workload_name.clone()),
            ("CPU", self.cpu.clone()),
            ("Memory", self.memory.clone()),
            ("Storage", self.storage.clone()),
        ]));

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

// ─── Regional / live pricing & chargeback ─────────────────────────────

/// Active pricing configuration (baseline table, optional live overlay, regional multiplier).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingConfig {
    pub provider: CloudProvider,
    pub region: String,
    pub source: String,
    pub regional_multiplier: f64,
    pub spot_discount_pct: f64,
    pub reserved_discount_pct: f64,
}

/// Per-workload chargeback line.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargebackLine {
    pub workload: String,
    pub owner: String,
    pub project: String,
    pub monthly_usd: f64,
    pub spot_monthly_usd: f64,
    pub reserved_monthly_usd: f64,
}

/// Fleet chargeback / showback report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargebackReport {
    pub provider: CloudProvider,
    pub region: String,
    pub pricing_source: String,
    pub total_monthly_usd: f64,
    pub total_spot_monthly_usd: f64,
    pub total_reserved_monthly_usd: f64,
    pub tco_36_months_usd: f64,
    pub lines: Vec<ChargebackLine>,
    pub by_owner: HashMap<String, f64>,
    pub by_project: HashMap<String, f64>,
}

/// Optional live pricing document from `AETHER_PRICING_URL` (JSON object with provider keys).
#[derive(Debug, Clone, Deserialize)]
struct LivePricingDoc {
    #[serde(default)]
    cpu_per_core_monthly: Option<f64>,
    #[serde(default)]
    memory_per_gb_monthly: Option<f64>,
    #[serde(default)]
    storage_per_gb_monthly: Option<f64>,
    #[serde(default)]
    gpu_per_unit_monthly: Option<f64>,
}

static LIVE_PRICING_CACHE: std::sync::OnceLock<std::sync::Mutex<Option<(std::time::Instant, ProviderPricing)>>> =
    std::sync::OnceLock::new();

/// Default provider for chargeback (`AETHER_COST_PROVIDER` or AWS).
pub fn default_chargeback_provider() -> CloudProvider {
    std::env::var("AETHER_COST_PROVIDER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(CloudProvider::AWS)
}

/// Region for pricing multiplier (`AETHER_COST_REGION`, default `us-east-1`).
pub fn pricing_region() -> String {
    std::env::var("AETHER_COST_REGION")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "us-east-1".to_string())
}

/// Regional cost multiplier (baseline table is US-centric).
pub fn regional_multiplier(region: &str) -> f64 {
    match region.to_lowercase().as_str() {
        "us-east-1" | "us-west-2" | "eastus" | "us-central1" | "global" => 1.0,
        "eu-west-1" | "eu-central-1" | "westeurope" | "northeurope" | "europe-west1" => 1.08,
        "ap-southeast-1" | "ap-northeast-1" | "southeastasia" | "asia-southeast1" => 1.12,
        "ap-south-1" | "southindia" => 1.10,
        _ => 1.05,
    }
}

fn spot_discount_pct() -> f64 {
    std::env::var("AETHER_COST_SPOT_DISCOUNT_PCT")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(60.0)
        .clamp(0.0, 90.0)
}

fn reserved_discount_pct() -> f64 {
    std::env::var("AETHER_COST_RESERVED_DISCOUNT_PCT")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(35.0)
        .clamp(0.0, 70.0)
}

fn fetch_live_pricing(provider: CloudProvider) -> Option<ProviderPricing> {
    let url = std::env::var("AETHER_PRICING_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let cache = LIVE_PRICING_CACHE.get_or_init(|| std::sync::Mutex::new(None));
    let mut guard = cache.lock().ok()?;
    if let Some((at, pricing)) = guard.as_ref() {
        if at.elapsed() < std::time::Duration::from_secs(3600) {
            return Some(*pricing);
        }
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let doc: serde_json::Value = resp.json().ok()?;
    let key = provider.to_string().to_lowercase();
    let node = doc.get(&key).or_else(|| doc.get("default"))?;
    let live: LivePricingDoc = serde_json::from_value(node.clone()).ok()?;
    let mut base = ProviderPricing::for_provider(provider);
    if let Some(v) = live.cpu_per_core_monthly {
        base.cpu_per_core_monthly = v;
    }
    if let Some(v) = live.memory_per_gb_monthly {
        base.memory_per_gb_monthly = v;
    }
    if let Some(v) = live.storage_per_gb_monthly {
        base.storage_per_gb_monthly = v;
    }
    if let Some(v) = live.gpu_per_unit_monthly {
        base.gpu_per_unit_monthly = v;
    }
    *guard = Some((std::time::Instant::now(), base));
    Some(base)
}

fn resolve_pricing(provider: CloudProvider) -> (ProviderPricing, String) {
    if std::env::var("AETHER_PRICING_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        if let Some(live) = fetch_live_pricing(provider) {
            return (live, "live".to_string());
        }
    }
    (ProviderPricing::for_provider(provider), "baseline".to_string())
}

/// Current pricing configuration for `/api/cost/pricing`.
pub fn pricing_config() -> PricingConfig {
    let provider = default_chargeback_provider();
    let region = pricing_region();
    let (_, source) = resolve_pricing(provider);
    PricingConfig {
        provider,
        region: region.clone(),
        source,
        regional_multiplier: regional_multiplier(&region),
        spot_discount_pct: spot_discount_pct(),
        reserved_discount_pct: reserved_discount_pct(),
    }
}

/// Estimate monthly cost with regional multiplier and optional live pricing overlay.
pub fn estimate_cost_priced(workload: &Workload, provider: CloudProvider) -> Result<CostEstimate> {
    let (pricing, _) = resolve_pricing(provider);
    let region = pricing_region();
    let mult = regional_multiplier(&region);

    let cpu_cores = parse_cpu(&workload.requirements.cpu)?;
    let memory_gb = parse_memory(&workload.requirements.memory)?;
    let storage_gb = parse_storage(&workload.requirements.storage)?;

    let cpu_cost = cpu_cores * pricing.cpu_per_core_monthly * mult;
    let memory_cost = memory_gb * pricing.memory_per_gb_monthly * mult;
    let storage_cost = storage_gb * pricing.storage_per_gb_monthly * mult;
    let total_monthly = cpu_cost + memory_cost + storage_cost;
    let total_hourly = total_monthly / 730.0;

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

fn apply_purchase_model(monthly: f64) -> (f64, f64) {
    let spot = monthly * (1.0 - spot_discount_pct() / 100.0);
    let reserved = monthly * (1.0 - reserved_discount_pct() / 100.0);
    (spot, reserved)
}

/// Build a chargeback report from deployed workloads and on-disk specs.
pub fn chargeback_report(
    workloads: &[(&str, &std::path::PathBuf)],
    provider: CloudProvider,
) -> Result<ChargebackReport> {
    let region = pricing_region();
    let (_, source) = resolve_pricing(provider);
    let mut lines = Vec::new();
    let mut by_owner = HashMap::new();
    let mut by_project = HashMap::new();
    let mut total = 0.0;
    let mut total_spot = 0.0;
    let mut total_reserved = 0.0;

    for (name, spec_path) in workloads {
        let spec = match Workload::from_file(spec_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let est = estimate_cost_priced(&spec, provider)?;
        let (spot, reserved) = apply_purchase_model(est.total_monthly);
        total += est.total_monthly;
        total_spot += spot;
        total_reserved += reserved;
        *by_owner.entry(spec.metadata.owner.clone()).or_insert(0.0) += est.total_monthly;
        *by_project.entry(spec.metadata.project.clone()).or_insert(0.0) += est.total_monthly;
        lines.push(ChargebackLine {
            workload: (*name).to_string(),
            owner: spec.metadata.owner.clone(),
            project: spec.metadata.project.clone(),
            monthly_usd: est.total_monthly,
            spot_monthly_usd: spot,
            reserved_monthly_usd: reserved,
        });
    }

    lines.sort_by(|a, b| {
        b.monthly_usd
            .partial_cmp(&a.monthly_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(ChargebackReport {
        provider,
        region,
        pricing_source: source,
        total_monthly_usd: total,
        total_spot_monthly_usd: total_spot,
        total_reserved_monthly_usd: total_reserved,
        tco_36_months_usd: total * 36.0,
        lines,
        by_owner,
        by_project,
    })
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
            autonomy: None,
            confidential: None,
            schedule: None,
        kubernetes: None,
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

    #[test]
    fn test_regional_multiplier() {
        assert_eq!(regional_multiplier("us-east-1"), 1.0);
        assert!(regional_multiplier("eu-west-1") > 1.0);
    }

    #[test]
    fn test_estimate_cost_priced() {
        let workload = create_test_workload();
        let est = estimate_cost_priced(&workload, CloudProvider::AWS).unwrap();
        assert!(est.total_monthly > 0.0);
    }

    #[test]
    fn test_cloud_provider_from_str() {
        assert_eq!("aws".parse::<CloudProvider>().unwrap(), CloudProvider::AWS);
        assert_eq!("AWS".parse::<CloudProvider>().unwrap(), CloudProvider::AWS);
        assert_eq!("azure".parse::<CloudProvider>().unwrap(), CloudProvider::Azure);
        assert_eq!("gcp".parse::<CloudProvider>().unwrap(), CloudProvider::GCP);
        assert_eq!("digitalocean".parse::<CloudProvider>().unwrap(), CloudProvider::DigitalOcean);
        assert_eq!("do".parse::<CloudProvider>().unwrap(), CloudProvider::DigitalOcean);
        assert_eq!("linode".parse::<CloudProvider>().unwrap(), CloudProvider::Linode);
        assert!("hetzner".parse::<CloudProvider>().is_err());
    }
}
