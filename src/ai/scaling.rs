// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Predictive scaling engine
//!
//! Analyzes resource usage patterns to forecast demand and recommend
//! scaling actions. Uses time-series analysis and trend detection.

use crate::config::ScalingConfig;
use crate::output;
use serde::{Deserialize, Serialize};

/// A single metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: f64, // Unix timestamp
    pub value: f64,
}

/// Time-series data for a metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub name: String,
    pub unit: String,
    pub points: Vec<MetricPoint>,
}

impl TimeSeries {
    pub fn new(name: &str, unit: &str) -> Self {
        Self {
            name: name.to_string(),
            unit: unit.to_string(),
            points: Vec::new(),
        }
    }

    pub fn add(&mut self, timestamp: f64, value: f64) {
        self.points.push(MetricPoint { timestamp, value });
    }
}

/// Scaling recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingRecommendation {
    pub action: ScalingAction,
    pub current_replicas: u32,
    pub recommended_replicas: u32,
    pub reason: String,
    pub confidence: f64,
    pub forecast: Forecast,
    pub cost_impact: CostImpact,
}

/// Scaling action to take
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScalingAction {
    ScaleUp,
    ScaleDown,
    NoChange,
}

impl std::fmt::Display for ScalingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalingAction::ScaleUp => write!(f, "Scale Up"),
            ScalingAction::ScaleDown => write!(f, "Scale Down"),
            ScalingAction::NoChange => write!(f, "No Change"),
        }
    }
}

/// Forecast result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast {
    pub trend: Trend,
    pub predicted_value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub horizon_minutes: u64,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Trend {
    Rising,
    Falling,
    Stable,
    Volatile,
}

impl std::fmt::Display for Trend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Trend::Rising => write!(f, "Rising"),
            Trend::Falling => write!(f, "Falling"),
            Trend::Stable => write!(f, "Stable"),
            Trend::Volatile => write!(f, "Volatile"),
        }
    }
}

/// Cost impact of scaling action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostImpact {
    pub current_hourly: f64,
    pub projected_hourly: f64,
    pub delta_hourly: f64,
    pub delta_monthly: f64,
}

/// Predictive scaling engine
pub struct ScalingEngine {
    config: ScalingConfig,
}

impl ScalingEngine {
    pub fn new(config: ScalingConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(ScalingConfig::default())
    }

    /// Analyze metrics and produce scaling recommendation
    pub fn recommend(
        &self,
        cpu_series: &TimeSeries,
        memory_series: &TimeSeries,
        current_replicas: u32,
        min_replicas: u32,
        max_replicas: u32,
        cost_per_replica_hourly: f64,
    ) -> ScalingRecommendation {
        // Analyze trends
        let cpu_forecast = self.forecast(cpu_series);
        let memory_forecast = self.forecast(memory_series);

        // Use the more demanding forecast
        let primary_forecast = if cpu_forecast.predicted_value > memory_forecast.predicted_value {
            &cpu_forecast
        } else {
            &memory_forecast
        };

        // Determine action
        let (action, recommended_replicas, reason, confidence) = self.decide_action(
            primary_forecast,
            current_replicas,
            min_replicas,
            max_replicas,
        );

        // Calculate cost impact
        let cost_impact = CostImpact {
            current_hourly: current_replicas as f64 * cost_per_replica_hourly,
            projected_hourly: recommended_replicas as f64 * cost_per_replica_hourly,
            delta_hourly: (recommended_replicas as f64 - current_replicas as f64)
                * cost_per_replica_hourly,
            delta_monthly: (recommended_replicas as f64 - current_replicas as f64)
                * cost_per_replica_hourly
                * 730.0, // hours per month
        };

        ScalingRecommendation {
            action,
            current_replicas,
            recommended_replicas,
            reason,
            confidence,
            forecast: primary_forecast.clone(),
            cost_impact,
        }
    }

    /// Forecast future value using linear regression
    fn forecast(&self, series: &TimeSeries) -> Forecast {
        let points = &series.points;

        if points.len() < 2 {
            return Forecast {
                trend: Trend::Stable,
                predicted_value: points.first().map(|p| p.value).unwrap_or(0.0),
                lower_bound: 0.0,
                upper_bound: 1.0,
                horizon_minutes: self.config.forecast_minutes,
            };
        }

        // Simple linear regression
        let n = points.len() as f64;
        let sum_x: f64 = points.iter().map(|p| p.timestamp).sum();
        let sum_y: f64 = points.iter().map(|p| p.value).sum();
        let sum_xy: f64 = points.iter().map(|p| p.timestamp * p.value).sum();
        let sum_xx: f64 = points.iter().map(|p| p.timestamp * p.timestamp).sum();

        let denominator = n * sum_xx - sum_x * sum_x;
        let (slope, intercept) = if denominator.abs() > f64::EPSILON {
            let slope = (n * sum_xy - sum_x * sum_y) / denominator;
            let intercept = (sum_y - slope * sum_x) / n;
            (slope, intercept)
        } else {
            (0.0, sum_y / n)
        };

        // Predict future value
        let last_timestamp = points.last().map(|p| p.timestamp).unwrap_or(0.0);
        let future_timestamp = last_timestamp + (self.config.forecast_minutes as f64 * 60.0);
        let predicted_value = (slope * future_timestamp + intercept).max(0.0);

        // Calculate standard deviation for bounds
        let mean = sum_y / n;
        let variance: f64 =
            (points.iter().map(|p| (p.value - mean).powi(2)).sum::<f64>() / n).max(0.0);
        let stddev = variance.sqrt();

        // Determine trend
        let trend = self.classify_trend(slope, stddev, mean);

        // Confidence bounds (1 stddev), guarding against NaN
        let stddev = if stddev.is_finite() { stddev } else { 0.0 };
        let lower_bound = (predicted_value - stddev).max(0.0);
        let upper_bound = predicted_value + stddev;

        Forecast {
            trend,
            predicted_value,
            lower_bound,
            upper_bound,
            horizon_minutes: self.config.forecast_minutes,
        }
    }

    /// Classify trend from regression slope
    fn classify_trend(&self, slope: f64, stddev: f64, mean: f64) -> Trend {
        let coefficient_of_variation = if mean > 0.0 { stddev / mean } else { 0.0 };

        if coefficient_of_variation > 0.5 {
            Trend::Volatile
        } else if slope > 0.0001 {
            Trend::Rising
        } else if slope < -0.0001 {
            Trend::Falling
        } else {
            Trend::Stable
        }
    }

    /// Decide scaling action
    fn decide_action(
        &self,
        forecast: &Forecast,
        current_replicas: u32,
        min_replicas: u32,
        max_replicas: u32,
    ) -> (ScalingAction, u32, String, f64) {
        let predicted = forecast.predicted_value;

        if predicted > self.config.scale_up_threshold {
            // Scale up
            let headroom = predicted / self.config.scale_up_threshold;
            let target = ((current_replicas as f64 * headroom).ceil() as u32).min(max_replicas);

            if target > current_replicas {
                let confidence = if forecast.trend == Trend::Rising {
                    0.85
                } else {
                    0.65
                };
                return (
                    ScalingAction::ScaleUp,
                    target,
                    format!(
                        "Predicted utilization {:.0}% exceeds {:.0}% threshold (trend: {})",
                        predicted * 100.0,
                        self.config.scale_up_threshold * 100.0,
                        forecast.trend
                    ),
                    confidence,
                );
            }
        }

        if predicted < self.config.scale_down_threshold && current_replicas > min_replicas {
            // Scale down (more conservative)
            let threshold = if self.config.scale_up_threshold > 0.0 {
                self.config.scale_up_threshold
            } else {
                0.80 // safe default if misconfigured
            };
            let target =
                ((current_replicas as f64 * predicted / threshold).ceil() as u32).max(min_replicas);

            if target < current_replicas {
                let confidence = if forecast.trend == Trend::Falling {
                    0.75
                } else {
                    0.50
                };

                if self.config.cost_aware {
                    return (
                        ScalingAction::ScaleDown,
                        target,
                        format!(
                            "Predicted utilization {:.0}% below {:.0}% threshold, cost savings available (trend: {})",
                            predicted * 100.0,
                            self.config.scale_down_threshold * 100.0,
                            forecast.trend
                        ),
                        confidence,
                    );
                }
            }
        }

        let confidence = if forecast.trend == Trend::Volatile {
            0.50
        } else {
            0.90
        };
        (
            ScalingAction::NoChange,
            current_replicas,
            format!(
                "Utilization {:.0}% within acceptable range (trend: {})",
                predicted * 100.0,
                forecast.trend
            ),
            confidence,
        )
    }
}

/// Format scaling recommendation as a report
pub fn format_scaling_report(rec: &ScalingRecommendation) -> String {
    let mut output = String::new();

    output.push_str(&output::property_section(&[
        ("Action", format!("{}", rec.action)),
        (
            "Replicas",
            format!("{} -> {}", rec.current_replicas, rec.recommended_replicas),
        ),
        ("Confidence", format!("{:.0}%", rec.confidence * 100.0)),
        ("Reason", rec.reason.clone()),
    ]));

    output.push_str(&output::property_section(&[
        ("Trend", format!("{}", rec.forecast.trend)),
        (
            "Predicted",
            format!("{:.1}%", rec.forecast.predicted_value * 100.0),
        ),
        (
            "Range",
            format!(
                "{:.1}% - {:.1}%",
                rec.forecast.lower_bound * 100.0,
                rec.forecast.upper_bound * 100.0
            ),
        ),
        (
            "Horizon",
            format!("{} minutes", rec.forecast.horizon_minutes),
        ),
    ]));

    if rec.action != ScalingAction::NoChange {
        output.push_str("Cost Impact:\n");
        output.push_str(&format!(
            "  Current: ${:.4}/hr\n",
            rec.cost_impact.current_hourly
        ));
        output.push_str(&format!(
            "  Projected: ${:.4}/hr\n",
            rec.cost_impact.projected_hourly
        ));
        output.push_str(&format!(
            "  Delta: {:+.4}/hr ({:+.2}/mo)\n",
            rec.cost_impact.delta_hourly, rec.cost_impact.delta_monthly
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_rising_series() -> TimeSeries {
        let mut series = TimeSeries::new("cpu_utilization", "ratio");
        let base_time = 1700000000.0;
        for i in 0..10 {
            series.add(base_time + (i as f64 * 60.0), 0.3 + (i as f64 * 0.06));
        }
        series
    }

    fn create_stable_series() -> TimeSeries {
        let mut series = TimeSeries::new("memory_utilization", "ratio");
        let base_time = 1700000000.0;
        for i in 0..10 {
            series.add(base_time + (i as f64 * 60.0), 0.50 + (i as f64 * 0.001));
        }
        series
    }

    fn create_low_series() -> TimeSeries {
        let mut series = TimeSeries::new("cpu_utilization", "ratio");
        let base_time = 1700000000.0;
        for i in 0..10 {
            series.add(base_time + (i as f64 * 60.0), 0.15 - (i as f64 * 0.005));
        }
        series
    }

    #[test]
    fn test_scale_up_recommendation() {
        let engine = ScalingEngine::with_defaults();
        let cpu = create_rising_series();
        let mem = create_stable_series();

        let rec = engine.recommend(&cpu, &mem, 2, 1, 10, 0.05);
        assert_eq!(rec.action, ScalingAction::ScaleUp);
        assert!(rec.recommended_replicas > 2);
    }

    #[test]
    fn test_no_change_recommendation() {
        let engine = ScalingEngine::with_defaults();
        let cpu = create_stable_series();
        let mem = create_stable_series();

        let rec = engine.recommend(&cpu, &mem, 3, 1, 10, 0.05);
        assert_eq!(rec.action, ScalingAction::NoChange);
        assert_eq!(rec.recommended_replicas, 3);
    }

    #[test]
    fn test_scale_down_recommendation() {
        let engine = ScalingEngine::with_defaults();
        let cpu = create_low_series();
        let mem = create_low_series();

        let rec = engine.recommend(&cpu, &mem, 5, 1, 10, 0.05);
        assert_eq!(rec.action, ScalingAction::ScaleDown);
        assert!(rec.recommended_replicas < 5);
    }

    #[test]
    fn test_respects_min_replicas() {
        let engine = ScalingEngine::with_defaults();
        let cpu = create_low_series();
        let mem = create_low_series();

        let rec = engine.recommend(&cpu, &mem, 3, 2, 10, 0.05);
        assert!(rec.recommended_replicas >= 2);
    }

    #[test]
    fn test_respects_max_replicas() {
        let engine = ScalingEngine::with_defaults();
        let cpu = create_rising_series();
        let mem = create_rising_series();

        let rec = engine.recommend(&cpu, &mem, 4, 1, 5, 0.05);
        assert!(rec.recommended_replicas <= 5);
    }

    #[test]
    fn test_cost_impact_calculated() {
        let engine = ScalingEngine::with_defaults();
        let cpu = create_rising_series();
        let mem = create_stable_series();

        let rec = engine.recommend(&cpu, &mem, 2, 1, 10, 0.05);
        if rec.action == ScalingAction::ScaleUp {
            assert!(rec.cost_impact.delta_hourly > 0.0);
        }
    }

    #[test]
    fn test_format_report() {
        let engine = ScalingEngine::with_defaults();
        let cpu = create_rising_series();
        let mem = create_stable_series();
        let rec = engine.recommend(&cpu, &mem, 2, 1, 10, 0.05);
        let report = format_scaling_report(&rec);
        assert!(report.contains("Action"));
        assert!(report.contains("Replicas"));
    }
}
