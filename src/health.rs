//! Workload health history and timeline tracking
//!
//! Records periodic health check results for each workload, providing
//! uptime calculations, restart tracking, and historical timelines.

use crate::runtime::{InstanceState, RuntimeKind};
use serde::{Deserialize, Serialize};

/// A single health check observation for a workload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthRecord {
    /// Timestamp of the health check in RFC 3339 format.
    pub timestamp: String,
    /// Name of the workload.
    pub workload: String,
    /// Runtime kind that hosts the workload.
    pub runtime: RuntimeKind,
    /// Instance state at the time of the check.
    pub state: InstanceState,
    /// Whether the workload was ready (healthy) at the time of the check.
    pub ready: bool,
    /// Cumulative restart count reported by the runtime.
    pub restart_count: u32,
    /// Optional health check probe latency in milliseconds.
    pub latency_ms: Option<f64>,
}

/// Accumulated health history for all workloads.
///
/// Stores a bounded ring of [`HealthRecord`]s, pruning the oldest entries
/// when `max_records` is reached to prevent unbounded growth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthHistory {
    /// Ordered list of health records (oldest first).
    pub records: Vec<HealthRecord>,
    /// Maximum number of records to retain.
    pub max_records: usize,
}

impl Default for HealthHistory {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            max_records: 1000,
        }
    }
}

crate::impl_json_store!(HealthHistory, "health.json");

/// Summary statistics for a single workload's health history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    /// Name of the workload.
    pub workload: String,
    /// Total number of health checks recorded.
    pub total_checks: usize,
    /// Number of checks where the workload was ready.
    pub ready_checks: usize,
    /// Percentage of checks where the workload was ready (0.0 -- 100.0).
    pub uptime_percent: f64,
    /// Most recent restart count reported by the runtime.
    pub last_restart_count: u32,
    /// String representation of the most recent instance state.
    pub last_state: String,
}

impl HealthHistory {
    /// Record a new health check observation.
    ///
    /// If the history has reached `max_records`, the oldest record is
    /// removed before the new one is appended.
    pub fn record(&mut self, record: HealthRecord) {
        if self.records.len() >= self.max_records {
            let drain = self.records.len() - self.max_records + 1;
            self.records.drain(..drain);
        }
        self.records.push(record);
    }

    /// Calculate the uptime percentage for a workload.
    ///
    /// Uptime is defined as the ratio of `ready == true` records to total
    /// records for the given workload, expressed as a percentage (0.0 -- 100.0).
    /// Returns `0.0` if there are no records for the workload.
    pub fn uptime_percent(&self, workload: &str) -> f64 {
        let total = self.records.iter().filter(|r| r.workload == workload).count();
        if total == 0 {
            return 0.0;
        }
        let ready = self
            .records
            .iter()
            .filter(|r| r.workload == workload && r.ready)
            .count();
        (ready as f64 / total as f64) * 100.0
    }

    /// Return the latest restart count for a workload.
    ///
    /// Returns `0` if there are no records for the workload.
    pub fn restart_count(&self, workload: &str) -> u32 {
        self.records
            .iter()
            .rev()
            .find(|r| r.workload == workload)
            .map(|r| r.restart_count)
            .unwrap_or(0)
    }

    /// Return the last `last_n` health records for a workload.
    ///
    /// Records are returned in chronological order (oldest first).
    pub fn timeline(&self, workload: &str, last_n: usize) -> Vec<&HealthRecord> {
        let matching: Vec<&HealthRecord> = self
            .records
            .iter()
            .filter(|r| r.workload == workload)
            .collect();
        let skip = matching.len().saturating_sub(last_n);
        matching.into_iter().skip(skip).collect()
    }

    /// Produce a [`HealthSummary`] for a workload.
    pub fn summary(&self, workload: &str) -> HealthSummary {
        let records: Vec<&HealthRecord> = self
            .records
            .iter()
            .filter(|r| r.workload == workload)
            .collect();

        let total_checks = records.len();
        let ready_checks = records.iter().filter(|r| r.ready).count();
        let uptime_percent = if total_checks == 0 {
            0.0
        } else {
            (ready_checks as f64 / total_checks as f64) * 100.0
        };

        let last = records.last();
        let last_restart_count = last.map(|r| r.restart_count).unwrap_or(0);
        let last_state = last
            .map(|r| r.state.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        HealthSummary {
            workload: workload.to_string(),
            total_checks,
            ready_checks,
            uptime_percent,
            last_restart_count,
            last_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{InstanceState, RuntimeKind};

    /// Helper to create a test health record.
    fn make_record(workload: &str, ready: bool, restart_count: u32, state: InstanceState) -> HealthRecord {
        HealthRecord {
            timestamp: "2026-01-15T10:00:00Z".to_string(),
            workload: workload.to_string(),
            runtime: RuntimeKind::Podman,
            state,
            ready,
            restart_count,
            latency_ms: None,
        }
    }

    fn make_record_with_latency(workload: &str, ready: bool, latency_ms: f64) -> HealthRecord {
        HealthRecord {
            timestamp: "2026-01-15T10:00:00Z".to_string(),
            workload: workload.to_string(),
            runtime: RuntimeKind::Kubernetes,
            state: if ready { InstanceState::Running } else { InstanceState::Failed },
            ready,
            restart_count: 0,
            latency_ms: Some(latency_ms),
        }
    }

    // ── Default / construction ────────────────────────────────────────

    #[test]
    fn test_default_history_is_empty() {
        let history = HealthHistory::default();
        assert!(history.records.is_empty());
        assert_eq!(history.max_records, 1000);
    }

    // ── record() ──────────────────────────────────────────────────────

    #[test]
    fn test_record_appends() {
        let mut history = HealthHistory::default();
        history.record(make_record("app", true, 0, InstanceState::Running));
        assert_eq!(history.records.len(), 1);
        history.record(make_record("app", true, 0, InstanceState::Running));
        assert_eq!(history.records.len(), 2);
    }

    #[test]
    fn test_record_prunes_oldest_at_max() {
        let mut history = HealthHistory {
            records: Vec::new(),
            max_records: 3,
        };

        for i in 0..5 {
            history.record(HealthRecord {
                timestamp: format!("2026-01-15T10:0{}:00Z", i),
                workload: "app".to_string(),
                runtime: RuntimeKind::Podman,
                state: InstanceState::Running,
                ready: true,
                restart_count: i as u32,
                latency_ms: None,
            });
        }

        assert_eq!(history.records.len(), 3);
        // Oldest surviving record should have restart_count == 2
        assert_eq!(history.records[0].restart_count, 2);
        assert_eq!(history.records[2].restart_count, 4);
    }

    #[test]
    fn test_record_prunes_exactly_at_boundary() {
        let mut history = HealthHistory {
            records: Vec::new(),
            max_records: 2,
        };
        history.record(make_record("a", true, 0, InstanceState::Running));
        history.record(make_record("a", true, 1, InstanceState::Running));
        assert_eq!(history.records.len(), 2);

        // Adding a third should prune the first
        history.record(make_record("a", true, 2, InstanceState::Running));
        assert_eq!(history.records.len(), 2);
        assert_eq!(history.records[0].restart_count, 1);
    }

    // ── uptime_percent() ──────────────────────────────────────────────

    #[test]
    fn test_uptime_percent_all_ready() {
        let mut history = HealthHistory::default();
        for _ in 0..10 {
            history.record(make_record("web", true, 0, InstanceState::Running));
        }
        assert!((history.uptime_percent("web") - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_uptime_percent_none_ready() {
        let mut history = HealthHistory::default();
        for _ in 0..5 {
            history.record(make_record("web", false, 0, InstanceState::Failed));
        }
        assert!((history.uptime_percent("web") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_uptime_percent_mixed() {
        let mut history = HealthHistory::default();
        // 3 ready, 1 not ready => 75%
        history.record(make_record("api", true, 0, InstanceState::Running));
        history.record(make_record("api", true, 0, InstanceState::Running));
        history.record(make_record("api", true, 0, InstanceState::Running));
        history.record(make_record("api", false, 1, InstanceState::Failed));
        assert!((history.uptime_percent("api") - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_uptime_percent_no_records() {
        let history = HealthHistory::default();
        assert!((history.uptime_percent("ghost") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_uptime_percent_isolates_workloads() {
        let mut history = HealthHistory::default();
        history.record(make_record("good", true, 0, InstanceState::Running));
        history.record(make_record("bad", false, 0, InstanceState::Failed));
        assert!((history.uptime_percent("good") - 100.0).abs() < f64::EPSILON);
        assert!((history.uptime_percent("bad") - 0.0).abs() < f64::EPSILON);
    }

    // ── restart_count() ───────────────────────────────────────────────

    #[test]
    fn test_restart_count_returns_latest() {
        let mut history = HealthHistory::default();
        history.record(make_record("svc", true, 0, InstanceState::Running));
        history.record(make_record("svc", true, 2, InstanceState::Running));
        history.record(make_record("svc", false, 5, InstanceState::Failed));
        assert_eq!(history.restart_count("svc"), 5);
    }

    #[test]
    fn test_restart_count_no_records() {
        let history = HealthHistory::default();
        assert_eq!(history.restart_count("missing"), 0);
    }

    #[test]
    fn test_restart_count_isolates_workloads() {
        let mut history = HealthHistory::default();
        history.record(make_record("a", true, 3, InstanceState::Running));
        history.record(make_record("b", true, 7, InstanceState::Running));
        assert_eq!(history.restart_count("a"), 3);
        assert_eq!(history.restart_count("b"), 7);
    }

    // ── timeline() ────────────────────────────────────────────────────

    #[test]
    fn test_timeline_returns_last_n() {
        let mut history = HealthHistory::default();
        for i in 0..10 {
            history.record(HealthRecord {
                timestamp: format!("2026-01-15T10:{:02}:00Z", i),
                workload: "web".to_string(),
                runtime: RuntimeKind::Podman,
                state: InstanceState::Running,
                ready: true,
                restart_count: i as u32,
                latency_ms: None,
            });
        }

        let tl = history.timeline("web", 3);
        assert_eq!(tl.len(), 3);
        assert_eq!(tl[0].restart_count, 7);
        assert_eq!(tl[1].restart_count, 8);
        assert_eq!(tl[2].restart_count, 9);
    }

    #[test]
    fn test_timeline_fewer_than_n() {
        let mut history = HealthHistory::default();
        history.record(make_record("web", true, 0, InstanceState::Running));
        let tl = history.timeline("web", 10);
        assert_eq!(tl.len(), 1);
    }

    #[test]
    fn test_timeline_no_records() {
        let history = HealthHistory::default();
        let tl = history.timeline("ghost", 5);
        assert!(tl.is_empty());
    }

    #[test]
    fn test_timeline_filters_by_workload() {
        let mut history = HealthHistory::default();
        history.record(make_record("a", true, 0, InstanceState::Running));
        history.record(make_record("b", true, 0, InstanceState::Running));
        history.record(make_record("a", true, 1, InstanceState::Running));

        let tl = history.timeline("a", 10);
        assert_eq!(tl.len(), 2);
        assert!(tl.iter().all(|r| r.workload == "a"));
    }

    // ── summary() ─────────────────────────────────────────────────────

    #[test]
    fn test_summary_complete() {
        let mut history = HealthHistory::default();
        history.record(make_record("api", true, 0, InstanceState::Running));
        history.record(make_record("api", true, 0, InstanceState::Running));
        history.record(make_record("api", false, 1, InstanceState::Failed));

        let s = history.summary("api");
        assert_eq!(s.workload, "api");
        assert_eq!(s.total_checks, 3);
        assert_eq!(s.ready_checks, 2);
        assert!((s.uptime_percent - 66.66666666666667).abs() < 0.001);
        assert_eq!(s.last_restart_count, 1);
        assert_eq!(s.last_state, "failed");
    }

    #[test]
    fn test_summary_no_records() {
        let history = HealthHistory::default();
        let s = history.summary("void");
        assert_eq!(s.total_checks, 0);
        assert_eq!(s.ready_checks, 0);
        assert!((s.uptime_percent - 0.0).abs() < f64::EPSILON);
        assert_eq!(s.last_restart_count, 0);
        assert_eq!(s.last_state, "unknown");
    }

    #[test]
    fn test_summary_all_ready() {
        let mut history = HealthHistory::default();
        for _ in 0..5 {
            history.record(make_record("healthy", true, 0, InstanceState::Running));
        }
        let s = history.summary("healthy");
        assert_eq!(s.total_checks, 5);
        assert_eq!(s.ready_checks, 5);
        assert!((s.uptime_percent - 100.0).abs() < f64::EPSILON);
    }

    // ── Latency field ─────────────────────────────────────────────────

    #[test]
    fn test_latency_field_some() {
        let r = make_record_with_latency("fast", true, 12.5);
        assert_eq!(r.latency_ms, Some(12.5));
    }

    #[test]
    fn test_latency_field_none() {
        let r = make_record("basic", true, 0, InstanceState::Running);
        assert_eq!(r.latency_ms, None);
    }

    // ── Serialization roundtrip ───────────────────────────────────────

    #[test]
    fn test_serialization_roundtrip() {
        let mut history = HealthHistory::default();
        history.record(make_record("web", true, 0, InstanceState::Running));
        history.record(make_record_with_latency("web", false, 42.0));

        let json = serde_json::to_string(&history).unwrap();
        let loaded: HealthHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.records.len(), 2);
        assert_eq!(loaded.max_records, 1000);
        assert!(loaded.records[0].ready);
        assert_eq!(loaded.records[1].latency_ms, Some(42.0));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("health.json");

        let mut history = HealthHistory::default();
        history.record(make_record("svc", true, 0, InstanceState::Running));
        history.record(make_record("svc", false, 3, InstanceState::Failed));
        history.save(&path).unwrap();

        let loaded = HealthHistory::load(&path).unwrap();
        assert_eq!(loaded.records.len(), 2);
        assert_eq!(loaded.records[1].restart_count, 3);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = std::path::Path::new("/tmp/aether_test_health_does_not_exist.json");
        let history = HealthHistory::load(path).unwrap();
        assert!(history.records.is_empty());
        assert_eq!(history.max_records, 1000);
    }

    #[test]
    fn test_default_path_ends_with_health_json() {
        let path = HealthHistory::default_path();
        assert!(path.ends_with(".aether/health.json"));
    }

    // ── Multiple workloads ────────────────────────────────────────────

    #[test]
    fn test_multiple_workloads_summary() {
        let mut history = HealthHistory::default();
        history.record(make_record("alpha", true, 0, InstanceState::Running));
        history.record(make_record("alpha", true, 0, InstanceState::Running));
        history.record(make_record("beta", false, 2, InstanceState::Failed));
        history.record(make_record("beta", true, 2, InstanceState::Running));

        let sa = history.summary("alpha");
        assert_eq!(sa.total_checks, 2);
        assert_eq!(sa.ready_checks, 2);

        let sb = history.summary("beta");
        assert_eq!(sb.total_checks, 2);
        assert_eq!(sb.ready_checks, 1);
        assert_eq!(sb.last_restart_count, 2);
    }

    #[test]
    fn test_many_records_pruning() {
        let mut history = HealthHistory {
            records: Vec::new(),
            max_records: 100,
        };
        for i in 0..200 {
            history.record(HealthRecord {
                timestamp: format!("2026-01-15T10:00:{:02}Z", i % 60),
                workload: "load-test".to_string(),
                runtime: RuntimeKind::Metal3,
                state: InstanceState::Running,
                ready: true,
                restart_count: i as u32,
                latency_ms: Some(i as f64),
            });
        }
        assert_eq!(history.records.len(), 100);
        // Latest record should have restart_count 199
        assert_eq!(history.records.last().unwrap().restart_count, 199);
    }

    // ── Edge case: timeline with zero ─────────────────────────────────

    #[test]
    fn test_timeline_zero_requested() {
        let mut history = HealthHistory::default();
        history.record(make_record("app", true, 0, InstanceState::Running));
        let tl = history.timeline("app", 0);
        assert!(tl.is_empty());
    }
}
