//! Event and notification system
//!
//! Event bus for SLA violations, drift detection, policy failures,
//! and operational alerts with configurable notification channels.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for EventSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSeverity::Info => write!(f, "INFO"),
            EventSeverity::Warning => write!(f, "WARN"),
            EventSeverity::Error => write!(f, "ERROR"),
            EventSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl std::str::FromStr for EventSeverity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(EventSeverity::Info),
            "warning" | "warn" => Ok(EventSeverity::Warning),
            "error" => Ok(EventSeverity::Error),
            "critical" => Ok(EventSeverity::Critical),
            _ => Err(anyhow::anyhow!("Unknown severity: '{}'. Valid: info, warning, error, critical", s)),
        }
    }
}

/// Event categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventCategory {
    Deployment,
    Migration,
    SlaViolation,
    DriftDetected,
    PolicyViolation,
    ScalingEvent,
    HealthCheck,
    SecretRotation,
    CostAnomaly,
    SystemAlert,
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventCategory::Deployment => write!(f, "DEPLOY"),
            EventCategory::Migration => write!(f, "MIGRATE"),
            EventCategory::SlaViolation => write!(f, "SLA"),
            EventCategory::DriftDetected => write!(f, "DRIFT"),
            EventCategory::PolicyViolation => write!(f, "POLICY"),
            EventCategory::ScalingEvent => write!(f, "SCALE"),
            EventCategory::HealthCheck => write!(f, "HEALTH"),
            EventCategory::SecretRotation => write!(f, "SECRET"),
            EventCategory::CostAnomaly => write!(f, "COST"),
            EventCategory::SystemAlert => write!(f, "SYSTEM"),
        }
    }
}

/// An event in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub timestamp: String,
    pub severity: EventSeverity,
    pub category: EventCategory,
    pub source: String,
    pub workload: Option<String>,
    pub title: String,
    pub message: String,
    pub metadata: HashMap<String, String>,
    pub acknowledged: bool,
}

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub name: String,
    pub channel_type: ChannelType,
    pub enabled: bool,
    pub min_severity: EventSeverity,
    pub categories: Vec<EventCategory>,
}

/// Types of notification channels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelType {
    /// Log to file
    File { path: String },
    /// HTTP webhook (Slack, Discord, PagerDuty, etc.)
    Webhook { url: String, method: String },
    /// Write to stdout
    Console,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::File { path } => write!(f, "file:{}", path),
            ChannelType::Webhook { url, .. } => write!(f, "webhook:{}", url),
            ChannelType::Console => write!(f, "console"),
        }
    }
}

/// Alert rule for automatic event generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub enabled: bool,
    pub condition: AlertCondition,
    pub severity: EventSeverity,
    pub message_template: String,
    pub cooldown_seconds: u64,
    pub last_triggered: Option<String>,
}

/// Conditions that trigger alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// SLA uptime drops below threshold
    SlaUptimeBelow(f64),
    /// Error rate exceeds threshold
    ErrorRateAbove(f64),
    /// Cost exceeds monthly budget
    CostExceeds(f64),
    /// Restart count exceeds threshold in period
    ExcessiveRestarts(u32),
    /// Drift detected on any workload
    DriftDetected,
    /// Policy violation on deployment
    PolicyViolation,
    /// Secret approaching rotation deadline
    SecretExpiring(u32),
}

impl std::fmt::Display for AlertCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertCondition::SlaUptimeBelow(v) => write!(f, "SLA uptime < {:.2}%", v),
            AlertCondition::ErrorRateAbove(v) => write!(f, "Error rate > {:.2}%", v),
            AlertCondition::CostExceeds(v) => write!(f, "Cost > ${:.2}/mo", v),
            AlertCondition::ExcessiveRestarts(v) => write!(f, "Restarts > {}/day", v),
            AlertCondition::DriftDetected => write!(f, "Drift detected"),
            AlertCondition::PolicyViolation => write!(f, "Policy violation"),
            AlertCondition::SecretExpiring(v) => write!(f, "Secret expires in {} days", v),
        }
    }
}

/// Event bus managing events, channels, and rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBus {
    events: Vec<Event>,
    channels: Vec<NotificationChannel>,
    rules: Vec<AlertRule>,
    next_id: u64,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

crate::impl_json_store!(EventBus, "events.json");

impl EventBus {
    /// Maximum number of events to retain
    const MAX_EVENTS: usize = 10_000;

    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            channels: vec![NotificationChannel {
                name: "console".to_string(),
                channel_type: ChannelType::Console,
                enabled: true,
                min_severity: EventSeverity::Warning,
                categories: vec![],
            }],
            rules: Self::default_rules(),
            next_id: 1,
        }
    }

    /// Emit an event
    pub fn emit(&mut self, event: Event) -> u64 {
        let id = self.next_id;
        let mut event = event;
        event.id = id;
        if event.timestamp.is_empty() {
            event.timestamp = crate::resources::now_rfc3339();
        }

        // Check notification channels
        let notifications = self.check_channels(&event);

        self.events.push(event);
        self.next_id += 1;

        // Auto-prune to prevent unbounded growth
        if self.events.len() > Self::MAX_EVENTS {
            let drain = self.events.len() - Self::MAX_EVENTS;
            self.events.drain(..drain);
        }

        // Process notifications (log format for now)
        for notification in notifications {
            self.deliver_notification(&notification);
        }

        id
    }

    /// Create and emit an event with builder pattern
    pub fn emit_simple(
        &mut self,
        severity: EventSeverity,
        category: EventCategory,
        source: &str,
        workload: Option<&str>,
        title: &str,
        message: &str,
    ) -> u64 {
        let event = Event {
            id: 0,
            timestamp: crate::resources::now_rfc3339(),
            severity,
            category,
            source: source.to_string(),
            workload: workload.map(|s| s.to_string()),
            title: title.to_string(),
            message: message.to_string(),
            metadata: HashMap::new(),
            acknowledged: false,
        };
        self.emit(event)
    }

    /// Get all events
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Get events by category
    pub fn events_by_category(&self, category: &EventCategory) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.category == *category)
            .collect()
    }

    /// Get events by severity (and above)
    pub fn events_by_severity(&self, min_severity: &EventSeverity) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.severity >= *min_severity)
            .collect()
    }

    /// Get events for a specific workload
    pub fn events_for_workload(&self, workload: &str) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.workload.as_deref() == Some(workload))
            .collect()
    }

    /// Get unacknowledged events
    pub fn unacknowledged(&self) -> Vec<&Event> {
        self.events.iter().filter(|e| !e.acknowledged).collect()
    }

    /// Acknowledge an event
    pub fn acknowledge(&mut self, id: u64) -> bool {
        if let Some(event) = self.events.iter_mut().find(|e| e.id == id) {
            event.acknowledged = true;
            true
        } else {
            false
        }
    }

    /// Get last N events
    pub fn last_n(&self, n: usize) -> Vec<&Event> {
        self.events.iter().rev().take(n).collect()
    }

    /// Add a notification channel
    pub fn add_channel(&mut self, channel: NotificationChannel) {
        self.channels.push(channel);
    }

    /// Remove a notification channel by name.
    /// Returns `true` if a channel was removed.
    pub fn remove_channel(&mut self, name: &str) -> bool {
        let before = self.channels.len();
        self.channels.retain(|c| c.name != name);
        self.channels.len() < before
    }

    /// List channels
    pub fn channels(&self) -> &[NotificationChannel] {
        &self.channels
    }

    /// Add an alert rule
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    /// List rules
    pub fn rules(&self) -> &[AlertRule] {
        &self.rules
    }

    /// Get event summary statistics
    pub fn summary(&self) -> EventSummary {
        let total = self.events.len();
        let unacknowledged = self.events.iter().filter(|e| !e.acknowledged).count();

        let mut by_severity: HashMap<String, usize> = HashMap::new();
        let mut by_category: HashMap<String, usize> = HashMap::new();

        for event in &self.events {
            *by_severity
                .entry(format!("{}", event.severity))
                .or_insert(0) += 1;
            *by_category
                .entry(format!("{}", event.category))
                .or_insert(0) += 1;
        }

        let critical_count = self
            .events
            .iter()
            .filter(|e| e.severity == EventSeverity::Critical && !e.acknowledged)
            .count();

        EventSummary {
            total_events: total,
            unacknowledged,
            critical_unacked: critical_count,
            by_severity,
            by_category,
        }
    }

    /// Prune old events
    pub fn prune(&mut self, max_events: usize) {
        if self.events.len() > max_events {
            let drain_count = self.events.len() - max_events;
            self.events.drain(..drain_count);
        }
    }

    // --- Private ---

    fn default_rules() -> Vec<AlertRule> {
        vec![
            AlertRule {
                name: "sla-violation".to_string(),
                enabled: true,
                condition: AlertCondition::SlaUptimeBelow(99.9),
                severity: EventSeverity::Critical,
                message_template: "SLA violation: uptime below threshold".to_string(),
                cooldown_seconds: 300,
                last_triggered: None,
            },
            AlertRule {
                name: "high-error-rate".to_string(),
                enabled: true,
                condition: AlertCondition::ErrorRateAbove(5.0),
                severity: EventSeverity::Error,
                message_template: "Error rate exceeds 5%".to_string(),
                cooldown_seconds: 600,
                last_triggered: None,
            },
            AlertRule {
                name: "drift-alert".to_string(),
                enabled: true,
                condition: AlertCondition::DriftDetected,
                severity: EventSeverity::Warning,
                message_template: "Configuration drift detected".to_string(),
                cooldown_seconds: 3600,
                last_triggered: None,
            },
        ]
    }

    fn check_channels(&self, event: &Event) -> Vec<NotificationPayload> {
        let mut notifications = Vec::new();

        for channel in &self.channels {
            if !channel.enabled {
                continue;
            }
            if event.severity < channel.min_severity {
                continue;
            }
            if !channel.categories.is_empty()
                && !channel.categories.contains(&event.category)
            {
                continue;
            }

            notifications.push(NotificationPayload {
                channel_name: channel.name.clone(),
                channel_type: channel.channel_type.clone(),
                event_id: event.id,
                title: event.title.clone(),
                message: format!(
                    "[{}] [{}] {} - {}",
                    event.severity, event.category, event.title, event.message
                ),
            });
        }

        notifications
    }

    fn deliver_notification(&self, notification: &NotificationPayload) {
        match &notification.channel_type {
            ChannelType::Console => {
                eprintln!("[ALERT] {}", notification.message);
            }
            ChannelType::File { path } => {
                // Check file size before writing to prevent unbounded growth
                const MAX_EVENT_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50 MB
                let should_write = match std::fs::metadata(path) {
                    Ok(meta) => meta.len() < MAX_EVENT_FILE_SIZE,
                    Err(_) => true, // file doesn't exist yet, ok to create
                };
                if should_write {
                    match std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                    {
                        Ok(mut file) => {
                            use std::io::Write;
                            let _ = writeln!(file, "{}", notification.message);
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to open event log file '{}': {} — notification channel is broken",
                                path, e
                            );
                        }
                    }
                } else {
                    tracing::error!(
                        "Event log file {} exceeds {} MB — events are being dropped! \
                         Rotate or truncate the file to resume logging.",
                        path,
                        MAX_EVENT_FILE_SIZE / (1024 * 1024)
                    );
                }
            }
            ChannelType::Webhook { url, method } => {
                let payload = WebhookPayload {
                    event_id: notification.event_id,
                    title: notification.title.clone(),
                    message: notification.message.clone(),
                    channel: notification.channel_name.clone(),
                    timestamp: crate::resources::now_rfc3339(),
                };

                let url = url.clone();
                let method = method.clone();
                let channel_name = notification.channel_name.clone();
                let event_id = notification.event_id;
                let title = notification.title.clone();
                let message = notification.message.clone();

                // Fire-and-forget on a blocking thread to avoid blocking
                // the caller and to avoid recreating the client per call.
                std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_secs(10))
                        .build()
                        .unwrap_or_else(|_| reqwest::blocking::Client::new());

                    let result = if method.eq_ignore_ascii_case("GET") {
                        client.get(&url).query(&[
                            ("event_id", event_id.to_string()),
                            ("title", title),
                            ("message", message),
                        ]).send()
                    } else {
                        client.post(&url).json(&payload).send()
                    };

                    match result {
                        Ok(resp) => {
                            tracing::info!(
                                "Webhook delivered to {} (status: {})",
                                channel_name,
                                resp.status()
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Webhook delivery failed for {}: {}",
                                channel_name,
                                e
                            );
                        }
                    }
                });
            }
        }
    }
}

/// Notification payload
#[derive(Debug, Clone, Serialize)]
pub struct NotificationPayload {
    pub channel_name: String,
    #[serde(skip)]
    pub channel_type: ChannelType,
    pub event_id: u64,
    pub title: String,
    pub message: String,
}

/// Webhook-specific JSON payload sent to HTTP endpoints
#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    pub event_id: u64,
    pub title: String,
    pub message: String,
    pub channel: String,
    pub timestamp: String,
}

/// Event summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub total_events: usize,
    pub unacknowledged: usize,
    pub critical_unacked: usize,
    pub by_severity: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
}

/// Format event list
pub fn format_event_list(events: &[&Event], limit: usize) -> String {
    let mut output = String::new();
    output.push_str("Events:\n\n");

    if events.is_empty() {
        output.push_str("  No events.\n");
        return output;
    }

    for event in events.iter().take(limit) {
        let ack = if event.acknowledged { " ✓" } else { "" };
        output.push_str(&format!(
            "  [{}] [{}] {}{}\n",
            event.severity, event.category, event.title, ack
        ));
        output.push_str(&format!("    {}\n", event.message));
        if let Some(workload) = &event.workload {
            output.push_str(&format!("    Workload: {}\n", workload));
        }
        output.push_str(&format!(
            "    Time: {}\n\n",
            &event.timestamp[..19]
        ));
    }

    output
}

/// Format event summary
pub fn format_event_summary(summary: &EventSummary) -> String {
    let mut output = String::new();
    output.push_str("Event Summary:\n\n");
    output.push_str(&format!("  Total: {}\n", summary.total_events));
    output.push_str(&format!("  Unacknowledged: {}\n", summary.unacknowledged));
    output.push_str(&format!("  Critical (unacked): {}\n\n", summary.critical_unacked));

    if !summary.by_severity.is_empty() {
        output.push_str("  By Severity:\n");
        let mut items: Vec<_> = summary.by_severity.iter().collect();
        items.sort_by(|a, b| b.1.cmp(a.1));
        for (sev, count) in items {
            output.push_str(&format!("    {}: {}\n", sev, count));
        }
    }

    if !summary.by_category.is_empty() {
        output.push_str("\n  By Category:\n");
        let mut items: Vec<_> = summary.by_category.iter().collect();
        items.sort_by(|a, b| b.1.cmp(a.1));
        for (cat, count) in items {
            output.push_str(&format!("    {}: {}\n", cat, count));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_event() {
        let mut bus = EventBus::new();
        let id = bus.emit_simple(
            EventSeverity::Warning,
            EventCategory::DriftDetected,
            "drift-detector",
            Some("web-app"),
            "Drift detected",
            "Runtime mismatch on web-app",
        );
        assert_eq!(id, 1);
        assert_eq!(bus.events().len(), 1);
    }

    #[test]
    fn test_filter_by_category() {
        let mut bus = EventBus::new();
        bus.emit_simple(
            EventSeverity::Info,
            EventCategory::Deployment,
            "engine",
            Some("app"),
            "Deployed",
            "Success",
        );
        bus.emit_simple(
            EventSeverity::Warning,
            EventCategory::DriftDetected,
            "drift",
            Some("app"),
            "Drift",
            "Found drift",
        );

        assert_eq!(bus.events_by_category(&EventCategory::DriftDetected).len(), 1);
        assert_eq!(bus.events_by_category(&EventCategory::Deployment).len(), 1);
    }

    #[test]
    fn test_acknowledge() {
        let mut bus = EventBus::new();
        let id = bus.emit_simple(
            EventSeverity::Critical,
            EventCategory::SlaViolation,
            "sla",
            Some("api"),
            "SLA Violated",
            "Uptime below target",
        );

        assert_eq!(bus.unacknowledged().len(), 1);
        assert!(bus.acknowledge(id));
        assert_eq!(bus.unacknowledged().len(), 0);
    }

    #[test]
    fn test_summary() {
        let mut bus = EventBus::new();
        bus.emit_simple(EventSeverity::Info, EventCategory::Deployment, "test", None, "Deploy", "ok");
        bus.emit_simple(EventSeverity::Warning, EventCategory::DriftDetected, "test", None, "Drift", "found");
        bus.emit_simple(EventSeverity::Critical, EventCategory::SlaViolation, "test", None, "SLA", "violated");

        let summary = bus.summary();
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.critical_unacked, 1);
    }

    #[test]
    fn test_prune() {
        let mut bus = EventBus::new();
        for i in 0..100 {
            bus.emit_simple(EventSeverity::Info, EventCategory::Deployment, "test", None, &format!("Event {}", i), "msg");
        }
        assert_eq!(bus.events().len(), 100);
        bus.prune(50);
        assert_eq!(bus.events().len(), 50);
    }

    #[test]
    fn test_format_event_list() {
        let mut bus = EventBus::new();
        bus.emit_simple(EventSeverity::Warning, EventCategory::DriftDetected, "test", Some("app"), "Drift", "found");
        let events: Vec<&Event> = bus.events().iter().collect();
        let output = format_event_list(&events, 10);
        assert!(output.contains("Drift"));
        assert!(output.contains("app"));
    }

    #[test]
    fn test_format_summary() {
        let mut bus = EventBus::new();
        bus.emit_simple(EventSeverity::Info, EventCategory::Deployment, "test", None, "Deploy", "ok");
        let summary = bus.summary();
        let output = format_event_summary(&summary);
        assert!(output.contains("Event Summary"));
    }

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            event_id: 42,
            title: "Test Alert".to_string(),
            message: "Something happened".to_string(),
            channel: "slack-ops".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"event_id\":42"));
        assert!(json.contains("\"title\":\"Test Alert\""));
        assert!(json.contains("\"channel\":\"slack-ops\""));
    }

    #[test]
    fn test_remove_channel() {
        let mut bus = EventBus::new();
        bus.add_channel(NotificationChannel {
            name: "slack".to_string(),
            channel_type: ChannelType::Webhook {
                url: "https://hooks.slack.com/test".to_string(),
                method: "POST".to_string(),
            },
            enabled: true,
            min_severity: EventSeverity::Warning,
            categories: vec![],
        });
        let initial_count = bus.channels().len();
        assert!(bus.remove_channel("slack"));
        assert_eq!(bus.channels().len(), initial_count - 1);
        // Removing again should return false
        assert!(!bus.remove_channel("slack"));
    }

    #[test]
    fn test_channel_matching_severity_filter() {
        let mut bus = EventBus::new();
        // Remove default console channel
        bus.remove_channel("console");

        // Add a channel that only accepts Error and above
        bus.add_channel(NotificationChannel {
            name: "errors-only".to_string(),
            channel_type: ChannelType::Console,
            enabled: true,
            min_severity: EventSeverity::Error,
            categories: vec![],
        });

        // Info event should not match
        bus.emit_simple(EventSeverity::Info, EventCategory::Deployment, "test", None, "Info", "ok");
        // Warning event should not match
        bus.emit_simple(EventSeverity::Warning, EventCategory::DriftDetected, "test", None, "Warn", "drift");
        // Error event should match
        bus.emit_simple(EventSeverity::Error, EventCategory::Deployment, "test", None, "Error", "fail");
        // Critical event should match
        bus.emit_simple(EventSeverity::Critical, EventCategory::SlaViolation, "test", None, "Critical", "bad");

        assert_eq!(bus.events().len(), 4);
    }

    #[test]
    fn test_event_severity_from_str() {
        assert_eq!("info".parse::<EventSeverity>().unwrap(), EventSeverity::Info);
        assert_eq!("warning".parse::<EventSeverity>().unwrap(), EventSeverity::Warning);
        assert_eq!("warn".parse::<EventSeverity>().unwrap(), EventSeverity::Warning);
        assert_eq!("error".parse::<EventSeverity>().unwrap(), EventSeverity::Error);
        assert_eq!("critical".parse::<EventSeverity>().unwrap(), EventSeverity::Critical);
        assert_eq!("CRITICAL".parse::<EventSeverity>().unwrap(), EventSeverity::Critical);
        assert!("debug".parse::<EventSeverity>().is_err());
    }
}
