// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Reactive autoscaling state for the `aether serve` control loop.
//!
//! Holds the per-workload rolling utilization history and last-scale timestamps
//! (for cooldown) that the scaling engine needs, and wraps
//! [`crate::ai::scaling::ScalingEngine`] so the loop stays a thin driver. The
//! utilization samples fed in are real ratios from `metrics.k8s.io` (see
//! [`crate::kubecluster::workload_utilization`]) — never synthetic.
//!
//! Only the *state and decision* live here (pure, unit-testable); the async
//! metric reads and the imperative scale patch stay in the serve loop.

use crate::ai::scaling::{ScalingEngine, ScalingRecommendation, TimeSeries};
use std::collections::HashMap;

/// Maximum utilization samples retained per workload (a bounded rolling window).
const HISTORY_MAX: usize = 20;

/// Per-workload autoscaling state: rolling CPU/memory ratios plus the last time
/// each workload was scaled (Unix epoch seconds) for cooldown enforcement.
#[derive(Debug, Default)]
pub struct AutoscaleState {
    cpu: HashMap<String, Vec<f64>>,
    mem: HashMap<String, Vec<f64>>,
    last_scaled_epoch: HashMap<String, i64>,
}

impl AutoscaleState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a utilization sample (0–1 ratios) for a workload.
    pub fn record(&mut self, workload: &str, cpu_ratio: f64, mem_ratio: f64) {
        push_bounded(self.cpu.entry(workload.to_string()).or_default(), cpu_ratio);
        push_bounded(self.mem.entry(workload.to_string()).or_default(), mem_ratio);
    }

    /// Number of samples retained for a workload.
    pub fn history_len(&self, workload: &str) -> usize {
        self.cpu.get(workload).map(|v| v.len()).unwrap_or(0)
    }

    /// Whether the scale cooldown has elapsed (no prior scale ⇒ always true).
    pub fn cooldown_elapsed(&self, workload: &str, now_epoch: i64, cooldown_secs: u64) -> bool {
        match self.last_scaled_epoch.get(workload) {
            Some(&t) => now_epoch.saturating_sub(t) >= cooldown_secs as i64,
            None => true,
        }
    }

    /// Record that a workload was scaled at `now_epoch` (starts its cooldown).
    pub fn mark_scaled(&mut self, workload: &str, now_epoch: i64) {
        self.last_scaled_epoch
            .insert(workload.to_string(), now_epoch);
    }

    /// Produce a scaling recommendation from the recorded history, or `None`
    /// until at least `min_points` samples exist for the workload.
    #[allow(clippy::too_many_arguments)]
    pub fn recommend(
        &self,
        engine: &ScalingEngine,
        workload: &str,
        current_replicas: u32,
        min_replicas: u32,
        max_replicas: u32,
        cost_per_replica_hourly: f64,
        min_points: usize,
    ) -> Option<ScalingRecommendation> {
        let cpu = self.cpu.get(workload)?;
        let mem = self.mem.get(workload)?;
        if cpu.len() < min_points.max(1) {
            return None;
        }
        let cpu_ts = to_series("cpu", cpu);
        let mem_ts = to_series("memory", mem);
        Some(engine.recommend(
            &cpu_ts,
            &mem_ts,
            current_replicas,
            min_replicas,
            max_replicas,
            cost_per_replica_hourly,
        ))
    }
}

fn push_bounded(v: &mut Vec<f64>, x: f64) {
    v.push(x);
    if v.len() > HISTORY_MAX {
        let excess = v.len() - HISTORY_MAX;
        v.drain(0..excess);
    }
}

fn to_series(name: &str, values: &[f64]) -> TimeSeries {
    let mut ts = TimeSeries::new(name, "ratio");
    for (i, &v) in values.iter().enumerate() {
        ts.add(i as f64, v);
    }
    ts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::scaling::ScalingAction;

    #[test]
    fn history_is_bounded() {
        let mut s = AutoscaleState::new();
        for _ in 0..(HISTORY_MAX + 10) {
            s.record("w", 0.5, 0.4);
        }
        assert_eq!(s.history_len("w"), HISTORY_MAX);
    }

    #[test]
    fn cooldown_gates_by_time() {
        let mut s = AutoscaleState::new();
        assert!(s.cooldown_elapsed("w", 1_000, 300)); // never scaled → allowed
        s.mark_scaled("w", 1_000);
        assert!(!s.cooldown_elapsed("w", 1_100, 300)); // 100s < 300s
        assert!(s.cooldown_elapsed("w", 1_400, 300)); // 400s ≥ 300s
    }

    #[test]
    fn no_recommendation_until_enough_points() {
        let mut s = AutoscaleState::new();
        let engine = ScalingEngine::with_defaults();
        s.record("w", 0.9, 0.5);
        assert!(s.recommend(&engine, "w", 2, 1, 10, 0.05, 3).is_none());
        s.record("w", 0.9, 0.5);
        s.record("w", 0.9, 0.5);
        assert!(s.recommend(&engine, "w", 2, 1, 10, 0.05, 3).is_some());
    }

    #[test]
    fn sustained_high_cpu_recommends_scale_up_within_bounds() {
        let mut s = AutoscaleState::new();
        let engine = ScalingEngine::with_defaults(); // scale_up_threshold 0.80
        for _ in 0..6 {
            s.record("w", 0.95, 0.6);
        }
        let rec = s.recommend(&engine, "w", 2, 1, 10, 0.05, 3).unwrap();
        assert_eq!(rec.action, ScalingAction::ScaleUp);
        assert!(rec.recommended_replicas > 2);
        assert!(rec.recommended_replicas <= 10, "clamped to max");
    }

    #[test]
    fn sustained_low_cpu_recommends_scale_down_but_respects_min() {
        let mut s = AutoscaleState::new();
        let engine = ScalingEngine::with_defaults(); // scale_down_threshold 0.30
        for _ in 0..6 {
            s.record("w", 0.05, 0.05);
        }
        let rec = s.recommend(&engine, "w", 5, 2, 10, 0.05, 3).unwrap();
        assert_eq!(rec.action, ScalingAction::ScaleDown);
        assert!(rec.recommended_replicas >= 2, "never below min");
    }
}
