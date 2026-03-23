//! Audit trail for all operations
//!
//! Records every operation performed on workloads with timestamps,
//! action type, result, and context for compliance and debugging.

use serde::{Deserialize, Serialize};

/// A single audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: u64,
    pub timestamp: String,
    pub action: AuditAction,
    pub workload: String,
    pub runtime: Option<String>,
    pub result: ActionResult,
    pub message: String,
    pub details: Option<String>,
}

/// Type of auditable action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditAction {
    Build,
    Deploy,
    Start,
    Stop,
    Delete,
    Migrate,
    Scale,
    ConfigChange,
    BackupCreate,
    BackupRestore,
    PolicyCheck,
    DriftDetected,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::Build => write!(f, "BUILD"),
            AuditAction::Deploy => write!(f, "DEPLOY"),
            AuditAction::Start => write!(f, "START"),
            AuditAction::Stop => write!(f, "STOP"),
            AuditAction::Delete => write!(f, "DELETE"),
            AuditAction::Migrate => write!(f, "MIGRATE"),
            AuditAction::Scale => write!(f, "SCALE"),
            AuditAction::ConfigChange => write!(f, "CONFIG"),
            AuditAction::BackupCreate => write!(f, "BACKUP"),
            AuditAction::BackupRestore => write!(f, "RESTORE"),
            AuditAction::PolicyCheck => write!(f, "POLICY"),
            AuditAction::DriftDetected => write!(f, "DRIFT"),
        }
    }
}

/// Result of an action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionResult {
    Success,
    Failure,
    Warning,
}

impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionResult::Success => write!(f, "OK"),
            ActionResult::Failure => write!(f, "FAIL"),
            ActionResult::Warning => write!(f, "WARN"),
        }
    }
}

/// Audit log store
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
    next_id: u64,
}

impl AuditLog {
    /// Maximum number of audit events to retain
    const MAX_EVENTS: usize = 10_000;

    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_id: 1,
        }
    }

    /// Record a new event
    pub fn record(
        &mut self,
        action: AuditAction,
        workload: &str,
        runtime: Option<&str>,
        result: ActionResult,
        message: &str,
        details: Option<&str>,
    ) {
        let event = AuditEvent {
            id: self.next_id,
            timestamp: crate::resources::now_rfc3339(),
            action,
            workload: workload.to_string(),
            runtime: runtime.map(|s| s.to_string()),
            result,
            message: message.to_string(),
            details: details.map(|s| s.to_string()),
        };

        self.events.push(event);
        self.next_id += 1;

        // Auto-prune to prevent unbounded growth
        if self.events.len() > Self::MAX_EVENTS {
            let drain = self.events.len() - Self::MAX_EVENTS;
            self.events.drain(..drain);
        }
    }

    /// Get all events
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Get events for a specific workload
    pub fn events_for(&self, workload: &str) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.workload == workload)
            .collect()
    }

    /// Get events by action type
    pub fn events_by_action(&self, action: &AuditAction) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.action == *action)
            .collect()
    }

    /// Get last N events
    pub fn last_n(&self, n: usize) -> Vec<&AuditEvent> {
        self.events.iter().rev().take(n).collect()
    }

    /// Get failure events
    pub fn failures(&self) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.result == ActionResult::Failure)
            .collect()
    }

    /// Count events by action type
    pub fn summary(&self) -> AuditSummary {
        let total = self.events.len();
        let successes = self
            .events
            .iter()
            .filter(|e| e.result == ActionResult::Success)
            .count();
        let failures = self
            .events
            .iter()
            .filter(|e| e.result == ActionResult::Failure)
            .count();

        let mut by_action: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for event in &self.events {
            *by_action.entry(format!("{}", event.action)).or_insert(0) += 1;
        }

        let mut workloads: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for event in &self.events {
            workloads.insert(event.workload.clone());
        }

        AuditSummary {
            total_events: total,
            successes,
            failures,
            unique_workloads: workloads.len(),
            events_by_action: by_action,
        }
    }

    /// Prune events older than N days.
    ///
    /// Parses timestamps to proper DateTime for comparison to avoid
    /// issues with different UTC offset formats (e.g. "+00:00" vs "Z").
    pub fn prune(&mut self, days: i64) {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        self.events.retain(|e| {
            match chrono::DateTime::parse_from_rfc3339(&e.timestamp) {
                Ok(dt) => dt.with_timezone(&chrono::Utc) >= cutoff,
                Err(_) => {
                    // Keep events with unparseable timestamps (don't silently drop data)
                    tracing::warn!("Could not parse event timestamp '{}', keeping event", e.timestamp);
                    true
                }
            }
        });
    }

}

crate::impl_json_store!(AuditLog, "audit.json");

/// Audit summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_events: usize,
    pub successes: usize,
    pub failures: usize,
    pub unique_workloads: usize,
    pub events_by_action: std::collections::HashMap<String, usize>,
}

/// Format audit log as a report
pub fn format_audit_report(log: &AuditLog, limit: usize) -> String {
    let mut output = String::new();
    let summary = log.summary();

    output.push_str("Audit Trail\n\n");
    output.push_str(&format!("Total events: {}\n", summary.total_events));
    output.push_str(&format!("Successes: {}\n", summary.successes));
    output.push_str(&format!("Failures: {}\n", summary.failures));
    output.push_str(&format!("Workloads: {}\n\n", summary.unique_workloads));

    if !summary.events_by_action.is_empty() {
        output.push_str("By Action:\n");
        let mut actions: Vec<_> = summary.events_by_action.iter().collect();
        actions.sort_by(|a, b| b.1.cmp(a.1));
        for (action, count) in actions {
            output.push_str(&format!("  {}: {}\n", action, count));
        }
        output.push('\n');
    }

    let recent = log.last_n(limit);
    if !recent.is_empty() {
        output.push_str(&format!("Recent Events (last {}):\n", recent.len()));
        for event in recent {
            output.push_str(&format!(
                "  [{}] {} {} {} - {}\n",
                event.timestamp.chars().take(19).collect::<String>(),
                event.result,
                event.action,
                event.workload,
                event.message,
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_event() {
        let mut log = AuditLog::new();
        log.record(
            AuditAction::Deploy,
            "web-app",
            Some("podman"),
            ActionResult::Success,
            "Deployed successfully",
            None,
        );

        assert_eq!(log.events().len(), 1);
        assert_eq!(log.events()[0].id, 1);
        assert_eq!(log.events()[0].workload, "web-app");
    }

    #[test]
    fn test_events_for_workload() {
        let mut log = AuditLog::new();
        log.record(AuditAction::Deploy, "web", None, ActionResult::Success, "ok", None);
        log.record(AuditAction::Deploy, "api", None, ActionResult::Success, "ok", None);
        log.record(AuditAction::Stop, "web", None, ActionResult::Success, "ok", None);

        assert_eq!(log.events_for("web").len(), 2);
        assert_eq!(log.events_for("api").len(), 1);
    }

    #[test]
    fn test_failures() {
        let mut log = AuditLog::new();
        log.record(AuditAction::Deploy, "web", None, ActionResult::Success, "ok", None);
        log.record(AuditAction::Deploy, "api", None, ActionResult::Failure, "timeout", None);

        assert_eq!(log.failures().len(), 1);
        assert_eq!(log.failures()[0].workload, "api");
    }

    #[test]
    fn test_summary() {
        let mut log = AuditLog::new();
        log.record(AuditAction::Deploy, "web", None, ActionResult::Success, "ok", None);
        log.record(AuditAction::Deploy, "api", None, ActionResult::Success, "ok", None);
        log.record(AuditAction::Stop, "web", None, ActionResult::Failure, "err", None);

        let summary = log.summary();
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.successes, 2);
        assert_eq!(summary.failures, 1);
        assert_eq!(summary.unique_workloads, 2);
    }

    #[test]
    fn test_last_n() {
        let mut log = AuditLog::new();
        for i in 0..10 {
            log.record(
                AuditAction::Deploy,
                &format!("app-{}", i),
                None,
                ActionResult::Success,
                "ok",
                None,
            );
        }

        assert_eq!(log.last_n(3).len(), 3);
        assert_eq!(log.last_n(20).len(), 10);
    }

    #[test]
    fn test_format_report() {
        let mut log = AuditLog::new();
        log.record(AuditAction::Deploy, "web", None, ActionResult::Success, "deployed", None);
        let report = format_audit_report(&log, 10);
        assert!(report.contains("Audit Trail"));
        assert!(report.contains("DEPLOY"));
    }
}
