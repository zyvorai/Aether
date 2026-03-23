//! Workload scheduler and optimizer
//!
//! Intelligent scheduling of workloads across runtimes based on
//! resource availability, affinity scores, cost constraints,
//! and placement policies.

use crate::runtime::RuntimeKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A workload scheduling request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRequest {
    pub workload_name: String,
    pub cpu_required: f64,
    pub memory_required_mb: u64,
    pub storage_required_mb: u64,
    pub gpu_required: u32,
    pub preferred_runtime: Option<RuntimeKind>,
    pub constraints: Vec<ScheduleConstraint>,
    pub priority: Priority,
}

/// Scheduling constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleConstraint {
    /// Must run on specific runtime
    RequireRuntime(RuntimeKind),
    /// Must not run on specific runtime
    ExcludeRuntime(RuntimeKind),
    /// Co-locate with another workload
    CoLocate(String),
    /// Anti-affinity with another workload
    AntiAffinity(String),
    /// Maximum cost per day
    MaxCostPerDay(f64),
    /// Require GPU
    RequireGpu,
    /// Require bare metal
    RequireBareMetal,
    /// Region/zone constraint
    Zone(String),
}

impl std::fmt::Display for ScheduleConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleConstraint::RequireRuntime(rt) => write!(f, "require:{}", rt),
            ScheduleConstraint::ExcludeRuntime(rt) => write!(f, "exclude:{}", rt),
            ScheduleConstraint::CoLocate(name) => write!(f, "co-locate:{}", name),
            ScheduleConstraint::AntiAffinity(name) => write!(f, "anti-affinity:{}", name),
            ScheduleConstraint::MaxCostPerDay(cost) => write!(f, "max-cost:${:.2}/day", cost),
            ScheduleConstraint::RequireGpu => write!(f, "require:gpu"),
            ScheduleConstraint::RequireBareMetal => write!(f, "require:bare-metal"),
            ScheduleConstraint::Zone(zone) => write!(f, "zone:{}", zone),
        }
    }
}

/// Workload priority
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "Low"),
            Priority::Normal => write!(f, "Normal"),
            Priority::High => write!(f, "High"),
            Priority::Critical => write!(f, "Critical"),
        }
    }
}

/// Runtime capacity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapacity {
    pub runtime: RuntimeKind,
    pub total_cpu: f64,
    pub available_cpu: f64,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub total_storage_mb: u64,
    pub available_storage_mb: u64,
    pub gpu_available: u32,
    pub cost_per_cpu_day: f64,
    pub cost_per_gb_day: f64,
    pub current_workloads: usize,
    pub max_workloads: usize,
    pub healthy: bool,
    pub zones: Vec<String>,
}

impl RuntimeCapacity {
    /// Default capacity for a runtime (for simulation/testing)
    pub fn default_for(runtime: RuntimeKind) -> Self {
        match runtime {
            RuntimeKind::Podman => Self {
                runtime,
                total_cpu: 8.0,
                available_cpu: 6.0,
                total_memory_mb: 16384,
                available_memory_mb: 12288,
                total_storage_mb: 102400,
                available_storage_mb: 81920,
                gpu_available: 0,
                cost_per_cpu_day: 0.5,
                cost_per_gb_day: 0.1,
                current_workloads: 2,
                max_workloads: 20,
                healthy: true,
                zones: vec!["local".to_string()],
            },
            RuntimeKind::Kubernetes => Self {
                runtime,
                total_cpu: 64.0,
                available_cpu: 48.0,
                total_memory_mb: 131072,
                available_memory_mb: 98304,
                total_storage_mb: 1048576,
                available_storage_mb: 786432,
                gpu_available: 2,
                cost_per_cpu_day: 1.0,
                cost_per_gb_day: 0.15,
                current_workloads: 10,
                max_workloads: 200,
                healthy: true,
                zones: vec!["us-east-1a".to_string(), "us-east-1b".to_string()],
            },
            RuntimeKind::KubeVirt => Self {
                runtime,
                total_cpu: 32.0,
                available_cpu: 24.0,
                total_memory_mb: 65536,
                available_memory_mb: 49152,
                total_storage_mb: 524288,
                available_storage_mb: 393216,
                gpu_available: 4,
                cost_per_cpu_day: 2.0,
                cost_per_gb_day: 0.25,
                current_workloads: 3,
                max_workloads: 50,
                healthy: true,
                zones: vec!["us-east-1a".to_string()],
            },
            RuntimeKind::Metal3 => Self {
                runtime,
                total_cpu: 128.0,
                available_cpu: 96.0,
                total_memory_mb: 524288,
                available_memory_mb: 393216,
                total_storage_mb: 4194304,
                available_storage_mb: 3145728,
                gpu_available: 8,
                cost_per_cpu_day: 3.0,
                cost_per_gb_day: 0.30,
                current_workloads: 1,
                max_workloads: 20,
                healthy: true,
                zones: vec!["dc-1".to_string()],
            },
        }
    }

    /// Check if this runtime can fit the request
    fn can_fit(&self, request: &ScheduleRequest) -> bool {
        self.healthy
            && self.available_cpu >= request.cpu_required
            && self.available_memory_mb >= request.memory_required_mb
            && self.available_storage_mb >= request.storage_required_mb
            && self.gpu_available >= request.gpu_required
            && self.current_workloads < self.max_workloads
    }

    /// Estimated daily cost for a workload
    fn estimated_cost(&self, request: &ScheduleRequest) -> f64 {
        let cpu_cost = request.cpu_required * self.cost_per_cpu_day;
        let mem_cost = (request.memory_required_mb as f64 / 1024.0) * self.cost_per_gb_day;
        cpu_cost + mem_cost
    }

    /// Resource utilization after placing workload (0.0-1.0)
    fn utilization_after(&self, request: &ScheduleRequest) -> f64 {
        let cpu_util = (self.total_cpu - self.available_cpu + request.cpu_required) / self.total_cpu;
        let mem_util = (self.total_memory_mb - self.available_memory_mb + request.memory_required_mb) as f64
            / self.total_memory_mb as f64;
        (cpu_util + mem_util) / 2.0
    }
}

/// Scheduling decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDecision {
    pub workload_name: String,
    pub selected_runtime: RuntimeKind,
    pub score: f64,
    pub estimated_cost_per_day: f64,
    pub reasons: Vec<String>,
    pub alternatives: Vec<Alternative>,
    pub warnings: Vec<String>,
}

/// Alternative placement option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub runtime: RuntimeKind,
    pub score: f64,
    pub estimated_cost_per_day: f64,
    pub reason: String,
}

/// Scheduling strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ScheduleStrategy {
    /// Minimize cost
    CostOptimized,
    /// Maximize performance (spread workloads)
    PerformanceOptimized,
    /// Pack workloads tightly (bin-packing)
    BinPacking,
    /// Balance across runtimes
    #[default]
    Balanced,
}

impl std::fmt::Display for ScheduleStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleStrategy::CostOptimized => write!(f, "Cost Optimized"),
            ScheduleStrategy::PerformanceOptimized => write!(f, "Performance Optimized"),
            ScheduleStrategy::BinPacking => write!(f, "Bin Packing"),
            ScheduleStrategy::Balanced => write!(f, "Balanced"),
        }
    }
}

impl std::str::FromStr for ScheduleStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cost" | "cost-optimized" => Ok(ScheduleStrategy::CostOptimized),
            "performance" | "performance-optimized" => Ok(ScheduleStrategy::PerformanceOptimized),
            "bin-packing" | "binpacking" => Ok(ScheduleStrategy::BinPacking),
            "balanced" => Ok(ScheduleStrategy::Balanced),
            _ => Err(anyhow::anyhow!("Unknown schedule strategy: '{}'. Valid: balanced, cost, performance, bin-packing", s)),
        }
    }
}

/// The workload scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scheduler {
    capacities: HashMap<RuntimeKind, RuntimeCapacity>,
    placements: Vec<Placement>,
    strategy: ScheduleStrategy,
    /// Optional affinity scores per runtime (from AffinityEngine)
    #[serde(default)]
    affinity_scores: HashMap<RuntimeKind, f64>,
}

/// A workload placement record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    pub workload_name: String,
    pub runtime: RuntimeKind,
    pub cpu_reserved: f64,
    pub memory_reserved_mb: u64,
    pub placed_at: String,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

crate::impl_json_store!(Scheduler, "scheduler.json");

impl Scheduler {
    pub fn new() -> Self {
        let mut capacities = HashMap::new();
        for rt in &[
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            RuntimeKind::KubeVirt,
            RuntimeKind::Metal3,
        ] {
            capacities.insert(*rt, RuntimeCapacity::default_for(*rt));
        }
        Self {
            capacities,
            placements: Vec::new(),
            strategy: ScheduleStrategy::default(),
            affinity_scores: HashMap::new(),
        }
    }

    /// Create with a specific strategy
    pub fn with_strategy(strategy: ScheduleStrategy) -> Self {
        let mut scheduler = Self::new();
        scheduler.strategy = strategy;
        scheduler
    }

    /// Set scheduling strategy
    pub fn set_strategy(&mut self, strategy: ScheduleStrategy) {
        self.strategy = strategy;
    }

    /// Set affinity scores from the affinity engine
    ///
    /// Scores should be composite affinity scores (0.0 - 1.0) keyed by runtime.
    /// These are factored into scheduling decisions as a bonus.
    pub fn set_affinity_scores(&mut self, scores: HashMap<RuntimeKind, f64>) {
        self.affinity_scores = scores;
    }

    /// Get current strategy
    pub fn strategy(&self) -> ScheduleStrategy {
        self.strategy
    }

    /// Update runtime capacity information
    pub fn update_capacity(&mut self, capacity: RuntimeCapacity) {
        self.capacities.insert(capacity.runtime, capacity);
    }

    /// Schedule a workload
    pub fn schedule(&mut self, request: &ScheduleRequest) -> Result<ScheduleDecision, ScheduleError> {
        // Filter runtimes by constraints
        let candidates = self.filter_by_constraints(request);

        if candidates.is_empty() {
            return Err(ScheduleError::NoFeasibleRuntime {
                workload: request.workload_name.clone(),
                reason: "No runtime satisfies all constraints and has sufficient capacity".to_string(),
            });
        }

        // Score each candidate
        let mut scored: Vec<(RuntimeKind, f64, Vec<String>)> = candidates
            .iter()
            .map(|rt| {
                let (score, reasons) = self.score_runtime(rt, request);
                (*rt, score, reasons)
            })
            .collect();

        // Filter out NaN/Inf scores before sorting
        scored.retain(|(_, score, _)| score.is_finite());

        if scored.is_empty() {
            return Err(ScheduleError::NoFeasibleRuntime {
                workload: request.workload_name.clone(),
                reason: "All candidate scores were non-finite (NaN/Inf)".to_string(),
            });
        }

        // Sort by score descending (all scores are finite after the retain above)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (selected, score, reasons) = scored[0].clone();
        let capacity = &self.capacities[&selected];
        let estimated_cost = capacity.estimated_cost(request);

        // Build alternatives
        let alternatives: Vec<Alternative> = scored[1..]
            .iter()
            .map(|(rt, sc, reasons)| {
                let cap = &self.capacities[rt];
                Alternative {
                    runtime: *rt,
                    score: *sc,
                    estimated_cost_per_day: cap.estimated_cost(request),
                    reason: reasons.first().cloned().unwrap_or_default(),
                }
            })
            .collect();

        // Check for warnings
        let mut warnings = Vec::new();
        if capacity.utilization_after(request) > 0.85 {
            warnings.push(format!(
                "Runtime {} will be at {:.0}% utilization after placement",
                selected,
                capacity.utilization_after(request) * 100.0
            ));
        }
        if capacity.current_workloads + 1 >= capacity.max_workloads {
            warnings.push(format!(
                "Runtime {} is approaching workload limit ({}/{})",
                selected,
                capacity.current_workloads + 1,
                capacity.max_workloads
            ));
        }

        // Record placement
        let placement = Placement {
            workload_name: request.workload_name.clone(),
            runtime: selected,
            cpu_reserved: request.cpu_required,
            memory_reserved_mb: request.memory_required_mb,
            placed_at: crate::resources::now_rfc3339(),
        };
        self.placements.push(placement);

        // Update capacity
        if let Some(cap) = self.capacities.get_mut(&selected) {
            if request.cpu_required > cap.available_cpu
                || request.memory_required_mb > cap.available_memory_mb
                || request.storage_required_mb > cap.available_storage_mb
            {
                tracing::warn!(
                    "Workload '{}' exceeds available capacity on {:?} — placement may be over-committed",
                    request.workload_name, selected
                );
            }
            cap.available_cpu -= request.cpu_required;
            cap.available_memory_mb = cap.available_memory_mb.saturating_sub(request.memory_required_mb);
            cap.available_storage_mb = cap.available_storage_mb.saturating_sub(request.storage_required_mb);
            cap.current_workloads += 1;
        }

        Ok(ScheduleDecision {
            workload_name: request.workload_name.clone(),
            selected_runtime: selected,
            score,
            estimated_cost_per_day: estimated_cost,
            reasons,
            alternatives,
            warnings,
        })
    }

    /// Remove a workload placement (when workload is deleted)
    pub fn release(&mut self, workload_name: &str) {
        if let Some(idx) = self.placements.iter().position(|p| p.workload_name == workload_name) {
            let placement = self.placements.remove(idx);
            if let Some(cap) = self.capacities.get_mut(&placement.runtime) {
                cap.available_cpu += placement.cpu_reserved;
                cap.available_memory_mb += placement.memory_reserved_mb;
                cap.current_workloads = cap.current_workloads.saturating_sub(1);
            }
        }
    }

    /// Get all placements
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    /// Get cluster utilization summary
    pub fn utilization_summary(&self) -> Vec<UtilizationInfo> {
        self.capacities
            .values()
            .map(|cap| {
                let cpu_util = if cap.total_cpu > 0.0 {
                    (cap.total_cpu - cap.available_cpu) / cap.total_cpu
                } else {
                    0.0
                };
                let mem_util = if cap.total_memory_mb > 0 {
                    (cap.total_memory_mb - cap.available_memory_mb) as f64 / cap.total_memory_mb as f64
                } else {
                    0.0
                };
                UtilizationInfo {
                    runtime: cap.runtime,
                    cpu_utilization: cpu_util,
                    memory_utilization: mem_util,
                    workload_count: cap.current_workloads,
                    max_workloads: cap.max_workloads,
                    healthy: cap.healthy,
                    estimated_cost_per_day: self.runtime_cost(cap),
                }
            })
            .collect()
    }

    /// Get optimization suggestions
    pub fn optimize(&self) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Check for unbalanced utilization
        let utils = self.utilization_summary();
        let avg_cpu: f64 = utils.iter().map(|u| u.cpu_utilization).sum::<f64>() / utils.len().max(1) as f64;

        for u in &utils {
            if u.cpu_utilization > 0.9 {
                suggestions.push(OptimizationSuggestion {
                    category: OptCategory::Capacity,
                    runtime: Some(u.runtime),
                    message: format!(
                        "{} CPU utilization at {:.0}% - consider scaling up or migrating workloads",
                        u.runtime,
                        u.cpu_utilization * 100.0
                    ),
                    potential_saving: None,
                });
            }
            if u.cpu_utilization < 0.1 && u.workload_count > 0 {
                suggestions.push(OptimizationSuggestion {
                    category: OptCategory::Cost,
                    runtime: Some(u.runtime),
                    message: format!(
                        "{} is underutilized ({:.0}% CPU) with {} workloads - consider consolidation",
                        u.runtime,
                        u.cpu_utilization * 100.0,
                        u.workload_count
                    ),
                    potential_saving: Some(u.estimated_cost_per_day * 0.5),
                });
            }
            if (u.cpu_utilization - avg_cpu).abs() > 0.3 {
                suggestions.push(OptimizationSuggestion {
                    category: OptCategory::Balance,
                    runtime: Some(u.runtime),
                    message: format!(
                        "{} utilization ({:.0}%) deviates significantly from average ({:.0}%)",
                        u.runtime,
                        u.cpu_utilization * 100.0,
                        avg_cpu * 100.0
                    ),
                    potential_saving: None,
                });
            }
        }

        // Check for cost optimization across placements
        for placement in &self.placements {
            if placement.runtime == RuntimeKind::Metal3 && placement.cpu_reserved < 4.0 {
                suggestions.push(OptimizationSuggestion {
                    category: OptCategory::Cost,
                    runtime: Some(placement.runtime),
                    message: format!(
                        "'{}' uses only {:.0} CPUs on Metal3 - consider moving to Kubernetes or Podman",
                        placement.workload_name, placement.cpu_reserved
                    ),
                    potential_saving: Some(placement.cpu_reserved * 2.0), // rough saving estimate
                });
            }
        }

        suggestions
    }

    // --- Private ---

    fn filter_by_constraints(&self, request: &ScheduleRequest) -> Vec<RuntimeKind> {
        let mut candidates: Vec<RuntimeKind> = self
            .capacities
            .values()
            .filter(|cap| cap.can_fit(request))
            .map(|cap| cap.runtime)
            .collect();

        for constraint in &request.constraints {
            match constraint {
                ScheduleConstraint::RequireRuntime(rt) => {
                    candidates.retain(|c| c == rt);
                }
                ScheduleConstraint::ExcludeRuntime(rt) => {
                    candidates.retain(|c| c != rt);
                }
                ScheduleConstraint::RequireGpu => {
                    candidates.retain(|c| {
                        self.capacities.get(c).is_some_and(|cap| cap.gpu_available > 0)
                    });
                }
                ScheduleConstraint::RequireBareMetal => {
                    candidates.retain(|c| *c == RuntimeKind::Metal3);
                }
                ScheduleConstraint::MaxCostPerDay(max_cost) => {
                    candidates.retain(|c| {
                        self.capacities
                            .get(c)
                            .is_some_and(|cap| cap.estimated_cost(request) <= *max_cost)
                    });
                }
                ScheduleConstraint::Zone(zone) => {
                    candidates.retain(|c| {
                        self.capacities
                            .get(c)
                            .is_some_and(|cap| cap.zones.contains(zone))
                    });
                }
                ScheduleConstraint::CoLocate(name) => {
                    if let Some(placement) = self.placements.iter().find(|p| p.workload_name == *name) {
                        candidates.retain(|c| *c == placement.runtime);
                    }
                }
                ScheduleConstraint::AntiAffinity(name) => {
                    if let Some(placement) = self.placements.iter().find(|p| p.workload_name == *name) {
                        candidates.retain(|c| *c != placement.runtime);
                    }
                }
            }
        }

        candidates
    }

    fn score_runtime(&self, runtime: &RuntimeKind, request: &ScheduleRequest) -> (f64, Vec<String>) {
        let cap = &self.capacities[runtime];
        let mut score = 0.0;
        let mut reasons = Vec::new();

        match self.strategy {
            ScheduleStrategy::CostOptimized => {
                // Lower cost = higher score
                let cost = cap.estimated_cost(request);
                let max_cost = 50.0; // normalize against max expected cost
                score += (1.0 - (cost / max_cost).min(1.0)) * 0.6;
                reasons.push(format!("Cost: ${:.2}/day", cost));
            }
            ScheduleStrategy::PerformanceOptimized => {
                // Lower utilization = higher score (more headroom)
                let util = cap.utilization_after(request);
                score += (1.0 - util) * 0.6;
                reasons.push(format!("Utilization after: {:.0}%", util * 100.0));
            }
            ScheduleStrategy::BinPacking => {
                // Higher utilization = higher score (pack tightly)
                let util = cap.utilization_after(request);
                score += util * 0.6;
                reasons.push(format!("Bin-pack utilization: {:.0}%", util * 100.0));
            }
            ScheduleStrategy::Balanced => {
                // Balance cost and performance
                let cost = cap.estimated_cost(request);
                let max_cost = 50.0;
                let util = cap.utilization_after(request);
                score += (1.0 - (cost / max_cost).min(1.0)) * 0.3;
                score += (1.0 - util) * 0.3;
                reasons.push(format!("Balanced: cost=${:.2}, util={:.0}%", cost, util * 100.0));
            }
        }

        // Preferred runtime bonus
        if request.preferred_runtime == Some(*runtime) {
            score += 0.2;
            reasons.push("Preferred runtime".to_string());
        }

        // Affinity score bonus (from learned deployment outcomes)
        if let Some(&affinity) = self.affinity_scores.get(runtime) {
            let bonus = affinity * 0.15; // up to 15% weight from affinity
            score += bonus;
            reasons.push(format!("Affinity: {:.0}%", affinity * 100.0));
        }

        // Capacity headroom bonus
        let cpu_headroom = cap.available_cpu / cap.total_cpu;
        score += cpu_headroom * 0.1;

        // Health bonus
        if cap.healthy {
            score += 0.1;
        }

        (score, reasons)
    }

    fn runtime_cost(&self, cap: &RuntimeCapacity) -> f64 {
        let used_cpu = cap.total_cpu - cap.available_cpu;
        let used_mem_gb = (cap.total_memory_mb - cap.available_memory_mb) as f64 / 1024.0;
        used_cpu * cap.cost_per_cpu_day + used_mem_gb * cap.cost_per_gb_day
    }
}

/// Scheduling error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleError {
    NoFeasibleRuntime { workload: String, reason: String },
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleError::NoFeasibleRuntime { workload, reason } => {
                write!(f, "Cannot schedule '{}': {}", workload, reason)
            }
        }
    }
}

impl std::error::Error for ScheduleError {}

/// Runtime utilization info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilizationInfo {
    pub runtime: RuntimeKind,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub workload_count: usize,
    pub max_workloads: usize,
    pub healthy: bool,
    pub estimated_cost_per_day: f64,
}

/// Optimization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub category: OptCategory,
    pub runtime: Option<RuntimeKind>,
    pub message: String,
    pub potential_saving: Option<f64>,
}

/// Optimization category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptCategory {
    Cost,
    Capacity,
    Balance,
    Performance,
}

impl std::fmt::Display for OptCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptCategory::Cost => write!(f, "COST"),
            OptCategory::Capacity => write!(f, "CAPACITY"),
            OptCategory::Balance => write!(f, "BALANCE"),
            OptCategory::Performance => write!(f, "PERFORMANCE"),
        }
    }
}

/// Format schedule decision
pub fn format_schedule_decision(decision: &ScheduleDecision) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "Schedule Decision: {}  →  {}\n\n",
        decision.workload_name, decision.selected_runtime
    ));
    output.push_str(&format!("  Score: {:.0}%\n", decision.score * 100.0));
    output.push_str(&format!(
        "  Estimated Cost: ${:.2}/day\n\n",
        decision.estimated_cost_per_day
    ));

    if !decision.reasons.is_empty() {
        output.push_str("  Reasons:\n");
        for reason in &decision.reasons {
            output.push_str(&format!("    - {}\n", reason));
        }
        output.push('\n');
    }

    if !decision.alternatives.is_empty() {
        output.push_str("  Alternatives:\n");
        for alt in &decision.alternatives {
            output.push_str(&format!(
                "    {} - Score: {:.0}%, Cost: ${:.2}/day\n",
                alt.runtime,
                alt.score * 100.0,
                alt.estimated_cost_per_day
            ));
        }
        output.push('\n');
    }

    if !decision.warnings.is_empty() {
        output.push_str("  Warnings:\n");
        for warn in &decision.warnings {
            output.push_str(&format!("    ! {}\n", warn));
        }
    }

    output
}

/// Format utilization summary
pub fn format_utilization(utils: &[UtilizationInfo]) -> String {
    let mut output = String::new();
    output.push_str("Runtime Utilization:\n\n");
    output.push_str(&format!(
        "  {:<14} {:>8} {:>8} {:>12} {:>8}\n",
        "Runtime", "CPU", "Memory", "Workloads", "Cost/Day"
    ));
    output.push_str(&format!("  {}\n", "-".repeat(56)));

    for u in utils {
        let health = if u.healthy { "" } else { " [UNHEALTHY]" };
        output.push_str(&format!(
            "  {:<14} {:>7.0}% {:>7.0}% {:>5}/{:<5} ${:>7.2}{}\n",
            u.runtime.to_string(),
            u.cpu_utilization * 100.0,
            u.memory_utilization * 100.0,
            u.workload_count,
            u.max_workloads,
            u.estimated_cost_per_day,
            health,
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_request(name: &str) -> ScheduleRequest {
        ScheduleRequest {
            workload_name: name.to_string(),
            cpu_required: 2.0,
            memory_required_mb: 2048,
            storage_required_mb: 10240,
            gpu_required: 0,
            preferred_runtime: None,
            constraints: vec![],
            priority: Priority::Normal,
        }
    }

    #[test]
    fn test_schedule_basic() {
        let mut scheduler = Scheduler::new();
        let request = basic_request("web-app");
        let decision = scheduler.schedule(&request).unwrap();
        assert!(!decision.workload_name.is_empty());
        assert!(decision.score > 0.0);
    }

    #[test]
    fn test_schedule_with_runtime_constraint() {
        let mut scheduler = Scheduler::new();
        let mut request = basic_request("api-server");
        request.constraints.push(ScheduleConstraint::RequireRuntime(RuntimeKind::Kubernetes));

        let decision = scheduler.schedule(&request).unwrap();
        assert_eq!(decision.selected_runtime, RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_schedule_exclude_runtime() {
        let mut scheduler = Scheduler::new();
        let mut request = basic_request("worker");
        request.constraints.push(ScheduleConstraint::ExcludeRuntime(RuntimeKind::Metal3));
        request.constraints.push(ScheduleConstraint::ExcludeRuntime(RuntimeKind::KubeVirt));

        let decision = scheduler.schedule(&request).unwrap();
        assert!(decision.selected_runtime != RuntimeKind::Metal3);
        assert!(decision.selected_runtime != RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_schedule_cost_optimized() {
        let mut scheduler = Scheduler::with_strategy(ScheduleStrategy::CostOptimized);
        let request = basic_request("cheap-worker");
        let decision = scheduler.schedule(&request).unwrap();
        // Podman should win on cost (lowest cost_per_cpu_day)
        assert_eq!(decision.selected_runtime, RuntimeKind::Podman);
    }

    #[test]
    fn test_schedule_no_feasible_runtime() {
        let mut scheduler = Scheduler::new();
        let mut request = basic_request("impossible");
        request.cpu_required = 9999.0; // exceeds all capacity

        let result = scheduler.schedule(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_release_workload() {
        let mut scheduler = Scheduler::new();
        let request = basic_request("temp-job");
        let decision = scheduler.schedule(&request).unwrap();
        assert_eq!(scheduler.placements().len(), 1);

        scheduler.release("temp-job");
        assert_eq!(scheduler.placements().len(), 0);

        // Capacity should be restored
        let cap = &scheduler.capacities[&decision.selected_runtime];
        let default_cap = RuntimeCapacity::default_for(decision.selected_runtime);
        assert_eq!(cap.available_cpu, default_cap.available_cpu);
    }

    #[test]
    fn test_utilization_summary() {
        let scheduler = Scheduler::new();
        let utils = scheduler.utilization_summary();
        assert_eq!(utils.len(), 4); // one per runtime
    }

    #[test]
    fn test_colocate_constraint() {
        let mut scheduler = Scheduler::new();
        let req1 = basic_request("frontend");
        let decision1 = scheduler.schedule(&req1).unwrap();

        let mut req2 = basic_request("backend");
        req2.constraints.push(ScheduleConstraint::CoLocate("frontend".to_string()));
        let decision2 = scheduler.schedule(&req2).unwrap();

        assert_eq!(decision1.selected_runtime, decision2.selected_runtime);
    }

    #[test]
    fn test_format_decision() {
        let mut scheduler = Scheduler::new();
        let request = basic_request("web-app");
        let decision = scheduler.schedule(&request).unwrap();
        let output = format_schedule_decision(&decision);
        assert!(output.contains("web-app"));
        assert!(output.contains("Score"));
    }

    #[test]
    fn test_format_utilization() {
        let scheduler = Scheduler::new();
        let utils = scheduler.utilization_summary();
        let output = format_utilization(&utils);
        assert!(output.contains("Runtime Utilization"));
    }

    #[test]
    fn test_schedule_strategy_from_str() {
        assert_eq!("balanced".parse::<ScheduleStrategy>().unwrap(), ScheduleStrategy::Balanced);
        assert_eq!("cost".parse::<ScheduleStrategy>().unwrap(), ScheduleStrategy::CostOptimized);
        assert_eq!("cost-optimized".parse::<ScheduleStrategy>().unwrap(), ScheduleStrategy::CostOptimized);
        assert_eq!("performance".parse::<ScheduleStrategy>().unwrap(), ScheduleStrategy::PerformanceOptimized);
        assert_eq!("bin-packing".parse::<ScheduleStrategy>().unwrap(), ScheduleStrategy::BinPacking);
        assert!("unknown".parse::<ScheduleStrategy>().is_err());
    }
}
