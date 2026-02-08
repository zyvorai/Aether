//! Configuration system for Orchestr8
//!
//! Loads tunable parameters from `~/.orchestr8/config.yaml`

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub engine: EngineConfig,
    #[serde(default)]
    pub migration: MigrationConfig,
    #[serde(default)]
    pub cost: CostConfig,
    #[serde(default)]
    pub scaling: ScalingConfig,
    #[serde(default)]
    pub profiler: ProfilerConfig,
    #[serde(default)]
    pub analyzer: AnalyzerConfig,
}

impl Config {
    /// Load configuration from default path
    pub fn load() -> Self {
        Self::load_from(&Self::default_path())
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &PathBuf) -> Self {
        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(path) {
            Ok(content) => serde_yaml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save configuration to default path
    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&Self::default_path())
    }

    /// Save configuration to a specific path
    pub fn save_to(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Default config file path
    pub fn default_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".orchestr8/config.yaml")
    }
}

/// Decision engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    /// CPU threshold (cores) above which Metal3 is preferred
    pub metal3_cpu_threshold: f64,
    /// Memory threshold (GiB) above which Metal3 is preferred
    pub metal3_memory_threshold_gi: f64,
    /// Enable AI scoring engine for runtime selection
    pub enable_scoring: bool,
    /// Scoring weights for multi-factor decisions
    pub scoring_weights: ScoringWeights,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            metal3_cpu_threshold: 16.0,
            metal3_memory_threshold_gi: 64.0,
            enable_scoring: true,
            scoring_weights: ScoringWeights::default(),
        }
    }
}

/// Scoring weights for runtime selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for cost efficiency (0.0 - 1.0)
    pub cost: f64,
    /// Weight for performance characteristics (0.0 - 1.0)
    pub performance: f64,
    /// Weight for reliability (0.0 - 1.0)
    pub reliability: f64,
    /// Weight for availability (0.0 - 1.0)
    pub availability: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            cost: 0.30,
            performance: 0.30,
            reliability: 0.25,
            availability: 0.15,
        }
    }
}

impl ScoringWeights {
    /// Normalize weights to sum to 1.0
    pub fn normalized(&self) -> Self {
        let total = self.cost + self.performance + self.reliability + self.availability;
        if total == 0.0 {
            return Self::default();
        }
        Self {
            cost: self.cost / total,
            performance: self.performance / total,
            reliability: self.reliability / total,
            availability: self.availability / total,
        }
    }
}

/// Migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationConfig {
    /// Graceful shutdown wait in seconds
    pub graceful_shutdown_secs: u64,
    /// Blue-green traffic switch delay in seconds
    pub traffic_switch_delay_secs: u64,
    /// Rolling traffic percentages
    pub rolling_steps: Vec<u32>,
    /// Interval between rolling steps in seconds
    pub rolling_interval_secs: u64,
    /// Maximum validation retries
    pub max_validation_retries: u32,
    /// Retry wait interval in seconds
    pub retry_wait_secs: u64,
    /// Error rate threshold (0.0 - 1.0) to trigger rollback during canary
    pub canary_error_threshold: f64,
    /// Enable adaptive timing
    pub enable_adaptive_timing: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            graceful_shutdown_secs: 5,
            traffic_switch_delay_secs: 10,
            rolling_steps: vec![25, 50, 75, 100],
            rolling_interval_secs: 5,
            max_validation_retries: 3,
            retry_wait_secs: 5,
            canary_error_threshold: 0.05,
            enable_adaptive_timing: true,
        }
    }
}

/// Cost estimation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostConfig {
    /// Include GPU cost in estimates
    pub include_gpu: bool,
    /// Include network egress cost
    pub include_network: bool,
    /// Estimated monthly network egress in GB
    pub estimated_egress_gb: f64,
    /// Include load balancer cost
    pub include_load_balancer: bool,
    /// Hours per month for cost calculation
    pub hours_per_month: f64,
    /// Enable right-sizing recommendations
    pub enable_recommendations: bool,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            include_gpu: true,
            include_network: true,
            estimated_egress_gb: 100.0,
            include_load_balancer: true,
            hours_per_month: 730.0,
            enable_recommendations: true,
        }
    }
}

/// Scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScalingConfig {
    /// Enable predictive scaling
    pub enable_predictive: bool,
    /// Lookback window for trend analysis in minutes
    pub lookback_minutes: u64,
    /// Forecast horizon in minutes
    pub forecast_minutes: u64,
    /// Scale-up threshold (percentage above forecast)
    pub scale_up_threshold: f64,
    /// Scale-down threshold (percentage below forecast)
    pub scale_down_threshold: f64,
    /// Cooldown period in seconds between scale actions
    pub cooldown_secs: u64,
    /// Enable cost-aware scaling decisions
    pub cost_aware: bool,
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            enable_predictive: true,
            lookback_minutes: 60,
            forecast_minutes: 15,
            scale_up_threshold: 0.80,
            scale_down_threshold: 0.30,
            cooldown_secs: 300,
            cost_aware: true,
        }
    }
}

/// Workload profiler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilerConfig {
    /// Enable workload classification
    pub enable_classification: bool,
    /// Resource waste threshold (percentage of over-provisioning)
    pub waste_threshold: f64,
    /// History retention in days
    pub history_days: u32,
    /// Enable right-sizing suggestions
    pub enable_right_sizing: bool,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            enable_classification: true,
            waste_threshold: 0.50,
            history_days: 30,
            enable_right_sizing: true,
        }
    }
}

/// Log analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerConfig {
    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,
    /// Z-score threshold for anomaly detection
    pub anomaly_z_threshold: f64,
    /// Sliding window size for error rate calculation
    pub window_size: usize,
    /// Maximum patterns to track
    pub max_patterns: usize,
    /// Enable error correlation
    pub enable_correlation: bool,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            enable_anomaly_detection: true,
            anomaly_z_threshold: 2.5,
            window_size: 100,
            max_patterns: 1000,
            enable_correlation: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.engine.metal3_cpu_threshold, 16.0);
        assert!(config.engine.enable_scoring);
        assert_eq!(config.migration.max_validation_retries, 3);
        assert!(config.cost.include_gpu);
        assert!(config.scaling.enable_predictive);
    }

    #[test]
    fn test_scoring_weights_normalized() {
        let weights = ScoringWeights {
            cost: 2.0,
            performance: 2.0,
            reliability: 2.0,
            availability: 2.0,
        };
        let normalized = weights.normalized();
        let total = normalized.cost + normalized.performance
            + normalized.reliability + normalized.availability;
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.engine.metal3_cpu_threshold, 16.0);
    }

    #[test]
    fn test_load_missing_file() {
        let config = Config::load_from(&PathBuf::from("/nonexistent/config.yaml"));
        assert_eq!(config.engine.metal3_cpu_threshold, 16.0);
    }
}
