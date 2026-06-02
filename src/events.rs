// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Event and notification system
//!
//! Event bus for SLA violations, drift detection, policy failures,
//! and operational alerts with configurable notification channels.

use crate::output;
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
            _ => Err(anyhow::anyhow!(
                "Unknown severity: '{}'. Valid: info, warning, error, critical",
                s
            )),
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
    IntentViolation,
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
            EventCategory::IntentViolation => write!(f, "INTENT"),
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
    /// Slack webhook (posts Block Kit formatted messages)
    Slack {
        /// Slack webhook URL (from Slack app config)
        webhook_url: String,
        /// Channel name for display purposes
        channel: String,
    },
    /// Write to stdout
    Console,
    /// PagerDuty Events API v2
    PagerDuty {
        /// PagerDuty integration/routing key
        routing_key: String,
    },
    /// Email via SMTP
    Email {
        /// SMTP server hostname
        smtp_host: String,
        /// SMTP server port (default: 587)
        smtp_port: u16,
        /// Sender email address
        from: String,
        /// Recipient email addresses
        to: Vec<String>,
    },
    /// Microsoft Teams webhook
    Teams {
        /// Teams incoming webhook URL
        webhook_url: String,
    },
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::File { path } => write!(f, "file:{}", path),
            ChannelType::Webhook { url, .. } => write!(f, "webhook:{}", url),
            ChannelType::Slack { channel, .. } => write!(f, "slack:#{}", channel),
            ChannelType::Console => write!(f, "console"),
            ChannelType::PagerDuty { .. } => write!(f, "pagerduty"),
            ChannelType::Email { from, .. } => write!(f, "email:{}", from),
            ChannelType::Teams { .. } => write!(f, "teams"),
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
    /// When set, rule only evaluates metrics for this workload name.
    #[serde(default)]
    pub workload: Option<String>,
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
                workload: None,
            },
            AlertRule {
                name: "high-error-rate".to_string(),
                enabled: true,
                condition: AlertCondition::ErrorRateAbove(5.0),
                severity: EventSeverity::Error,
                message_template: "Error rate exceeds 5%".to_string(),
                cooldown_seconds: 600,
                last_triggered: None,
                workload: None,
            },
            AlertRule {
                name: "drift-alert".to_string(),
                enabled: true,
                condition: AlertCondition::DriftDetected,
                severity: EventSeverity::Warning,
                message_template: "Configuration drift detected".to_string(),
                cooldown_seconds: 3600,
                last_triggered: None,
                workload: None,
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
            if !channel.categories.is_empty() && !channel.categories.contains(&event.category) {
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
                severity: event.severity.clone(),
                category: event.category.clone(),
                workload: event.workload.clone(),
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

                // Queue with immediate delivery attempt + persistent retry on failure
                WebhookQueue::enqueue(&payload, url, method);
            }
            ChannelType::Slack { webhook_url, .. } => {
                let payload_json = format_slack_payload(notification);
                // Slack webhooks are always POST
                WebhookQueue::enqueue_raw(&payload_json, webhook_url, "POST");
            }
            ChannelType::PagerDuty { routing_key } => {
                let payload_json = format_pagerduty_payload(notification, routing_key);
                WebhookQueue::enqueue_raw(
                    &payload_json,
                    "https://events.pagerduty.com/v2/enqueue",
                    "POST",
                );
            }
            ChannelType::Email {
                smtp_host,
                smtp_port,
                from,
                to,
            } => {
                use lettre::message::header::ContentType;
                use lettre::{Message, SmtpTransport, Transport};

                let subject = format!(
                    "[Aether] [{}] {}",
                    notification.severity, notification.title
                );
                let body = format!(
                    "Aether Notification\n\
                     ====================\n\n\
                     Severity: {}\n\
                     Category: {}\n\
                     Workload: {}\n\n\
                     {}\n\n\
                     --\nAether Runtime Control Plane",
                    notification.severity,
                    notification.category,
                    notification.workload.as_deref().unwrap_or("N/A"),
                    notification.message,
                );

                for recipient in to {
                    let email = match Message::builder()
                        .from(from.parse().unwrap_or_else(|_| {
                            tracing::warn!("Invalid email from address: {}", from);
                            "aether@localhost".parse().unwrap()
                        }))
                        .to(match recipient.parse() {
                            Ok(addr) => addr,
                            Err(e) => {
                                tracing::warn!("Invalid email recipient '{}': {}", recipient, e);
                                continue;
                            }
                        })
                        .subject(&subject)
                        .header(ContentType::TEXT_PLAIN)
                        .body(body.clone())
                    {
                        Ok(msg) => msg,
                        Err(e) => {
                            tracing::warn!("Failed to build email message: {}", e);
                            continue;
                        }
                    };

                    // NOTE: builder_dangerous disables TLS certificate verification.
                    // Suitable for local/dev SMTP relays. For production with TLS,
                    // switch to SmtpTransport::relay() which enforces TLS.
                    match SmtpTransport::builder_dangerous(smtp_host)
                        .port(*smtp_port)
                        .build()
                        .send(&email)
                    {
                        Ok(_) => tracing::info!("Email notification sent to {}", recipient),
                        Err(e) => tracing::warn!("Failed to send email to {}: {}", recipient, e),
                    }
                }
            }
            ChannelType::Teams { webhook_url } => {
                let payload_json = format_teams_payload(notification);
                WebhookQueue::enqueue_raw(&payload_json, webhook_url, "POST");
            }
        }
    }
}

/// Format a notification as a Slack Block Kit message with color-coded attachments.
fn format_slack_payload(notification: &NotificationPayload) -> String {
    let color = match notification.severity {
        EventSeverity::Critical => "#dc3545",
        EventSeverity::Error => "#fd7e14",
        EventSeverity::Warning => "#ffc107",
        EventSeverity::Info => "#28a745",
    };

    let workload_display = notification.workload.as_deref().unwrap_or("(none)");

    serde_json::json!({
        "attachments": [{
            "color": color,
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": format!("Aether: {}", notification.category)
                    }
                },
                {
                    "type": "section",
                    "fields": [
                        {
                            "type": "mrkdwn",
                            "text": format!("*Severity:* {}", notification.severity)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Workload:* {}", workload_display)
                        }
                    ]
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": notification.message.clone()
                    }
                }
            ]
        }]
    })
    .to_string()
}

/// Format a notification as a PagerDuty Events API v2 payload.
fn format_pagerduty_payload(notification: &NotificationPayload, routing_key: &str) -> String {
    let severity = match notification.severity {
        EventSeverity::Critical => "critical",
        EventSeverity::Error => "error",
        EventSeverity::Warning => "warning",
        EventSeverity::Info => "info",
    };

    serde_json::json!({
        "routing_key": routing_key,
        "event_action": "trigger",
        "payload": {
            "summary": format!("[Aether] {}", notification.title),
            "source": "aether",
            "severity": severity,
            "component": notification.workload.as_deref().unwrap_or("unknown"),
            "group": format!("{}", notification.category),
            "custom_details": {
                "message": notification.message,
                "event_id": notification.event_id,
                "category": format!("{}", notification.category),
            }
        }
    })
    .to_string()
}

/// Format a notification as a Microsoft Teams Adaptive Card payload.
fn format_teams_payload(notification: &NotificationPayload) -> String {
    let color = match notification.severity {
        EventSeverity::Critical => "attention",
        EventSeverity::Error => "warning",
        EventSeverity::Warning => "accent",
        EventSeverity::Info => "good",
    };

    serde_json::json!({
        "type": "message",
        "attachments": [{
            "contentType": "application/vnd.microsoft.card.adaptive",
            "content": {
                "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                "type": "AdaptiveCard",
                "version": "1.4",
                "body": [
                    {
                        "type": "TextBlock",
                        "size": "Medium",
                        "weight": "Bolder",
                        "text": format!("Aether: {}", notification.title),
                        "color": color
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Severity", "value": format!("{}", notification.severity) },
                            { "title": "Category", "value": format!("{}", notification.category) },
                            { "title": "Workload", "value": notification.workload.as_deref().unwrap_or("N/A") }
                        ]
                    },
                    {
                        "type": "TextBlock",
                        "text": notification.message,
                        "wrap": true
                    }
                ]
            }
        }]
    })
    .to_string()
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
    /// Event severity (used by Slack formatter)
    pub severity: EventSeverity,
    /// Event category (used by Slack formatter)
    pub category: EventCategory,
    /// Workload name if applicable (used by Slack formatter)
    pub workload: Option<String>,
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
            event.timestamp.get(..19).unwrap_or(&event.timestamp)
        ));
    }

    output
}

/// Format event summary
pub fn format_event_summary(summary: &EventSummary) -> String {
    let mut output = String::new();
    output.push_str(&output::property_section(&[
        ("Event Summary", String::new()),
        ("Total", format!("{}", summary.total_events)),
        ("Unacknowledged", format!("{}", summary.unacknowledged)),
        (
            "Critical (unacked)",
            format!("{}", summary.critical_unacked),
        ),
    ]));

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

// ─── Alert Rule Evaluation ────────────────────────────────────────────

/// Snapshot of current system metrics for alert evaluation.
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    /// Per-workload uptime percentage (0.0 -- 100.0)
    pub sla_uptimes: HashMap<String, f64>,
    /// Per-workload restart counts
    pub restart_counts: HashMap<String, u32>,
    /// Per-workload health failure rate percentage (0.0 -- 100.0)
    pub error_rates: HashMap<String, f64>,
    /// Per-workload estimated monthly cost (USD)
    pub workload_monthly_costs: HashMap<String, f64>,
    /// Sum of per-workload monthly costs (USD)
    pub fleet_monthly_cost_usd: f64,
    /// Whether drift was detected on any workload
    pub drift_detected: bool,
    /// Whether any policy violation was found
    pub policy_violations: bool,
    /// Per-secret days until expiry
    pub secrets_expiring_days: HashMap<String, u32>,
}

impl SystemMetrics {
    /// Build metrics from workload state, health history, and optional cost estimates.
    pub fn collect(
        workloads: &[crate::state::WorkloadState],
        policy_config: &crate::config::PolicyConfig,
    ) -> Self {
        let mut metrics = Self::default();
        Self::collect_into(&mut metrics, workloads, policy_config);
        metrics
    }

    /// Populate an existing metrics snapshot.
    pub fn collect_into(
        metrics: &mut Self,
        workloads: &[crate::state::WorkloadState],
        policy_config: &crate::config::PolicyConfig,
    ) {
        *metrics = Self::default();
        let health_path = crate::health::HealthHistory::default_path();
        let history = crate::health::HealthHistory::load(&health_path).unwrap_or_default();
        let provider = crate::cost::default_chargeback_provider();

        for ws in workloads {
            let uptime = history.uptime_percent(&ws.name);
            if uptime > 0.0 {
                metrics.sla_uptimes.insert(ws.name.clone(), uptime);
                metrics
                    .error_rates
                    .insert(ws.name.clone(), history.failure_rate_percent(&ws.name));
            }
            metrics
                .restart_counts
                .insert(ws.name.clone(), history.restart_count(&ws.name));

            if let Ok(spec) = crate::spec::Workload::from_file(&ws.spec_path) {
                if let Ok(est) = crate::cost::estimate_cost_priced(&spec, provider) {
                    metrics
                        .workload_monthly_costs
                        .insert(ws.name.clone(), est.total_monthly);
                    metrics.fleet_monthly_cost_usd += est.total_monthly;
                }

                let detector = crate::drift::DriftDetector::new();
                let report = detector.detect(&spec, ws);
                if report.has_drift {
                    metrics.drift_detected = true;
                }
                if crate::policy::gate_deploy(&spec, policy_config).is_err() {
                    metrics.policy_violations = true;
                }
            }
        }

        let secrets_path = crate::secrets::SecretStore::default_path();
        if let Ok(store) = crate::secrets::SecretStore::load(&secrets_path) {
            for alert in store.audit_rotation() {
                let days_left = alert.max_age_days.saturating_sub(alert.age_days);
                metrics
                    .secrets_expiring_days
                    .entry(alert.secret.clone())
                    .and_modify(|d| *d = (*d).min(days_left))
                    .or_insert(days_left);
            }
        }
    }
}

impl EventBus {
    fn rule_condition_met(
        condition: &AlertCondition,
        metrics: &SystemMetrics,
        workload: Option<&str>,
    ) -> bool {
        match condition {
            AlertCondition::SlaUptimeBelow(threshold) => match workload {
                Some(wl) => metrics
                    .sla_uptimes
                    .get(wl)
                    .map(|u| *u < *threshold && *u > 0.0)
                    .unwrap_or(false),
                None => metrics
                    .sla_uptimes
                    .values()
                    .any(|u| *u < *threshold && *u > 0.0),
            },
            AlertCondition::ErrorRateAbove(threshold) => match workload {
                Some(wl) => metrics
                    .error_rates
                    .get(wl)
                    .map(|rate| *rate > *threshold)
                    .unwrap_or(false),
                None => metrics.error_rates.values().any(|rate| *rate > *threshold),
            },
            AlertCondition::CostExceeds(threshold) => match workload {
                Some(wl) => metrics
                    .workload_monthly_costs
                    .get(wl)
                    .map(|c| *c > *threshold)
                    .unwrap_or(false),
                None => {
                    metrics.fleet_monthly_cost_usd > *threshold
                        || metrics
                            .workload_monthly_costs
                            .values()
                            .any(|c| *c > *threshold)
                }
            },
            AlertCondition::ExcessiveRestarts(max) => match workload {
                Some(wl) => metrics
                    .restart_counts
                    .get(wl)
                    .map(|c| *c > *max)
                    .unwrap_or(false),
                None => metrics.restart_counts.values().any(|c| *c > *max),
            },
            AlertCondition::DriftDetected => metrics.drift_detected,
            AlertCondition::PolicyViolation => metrics.policy_violations,
            AlertCondition::SecretExpiring(days) => {
                metrics.secrets_expiring_days.values().any(|d| *d <= *days)
            }
        }
    }

    /// Evaluate all enabled alert rules against current system metrics.
    /// Emits events and triggers notifications for rules whose conditions are met.
    /// Returns the IDs of any events that were emitted.
    pub fn evaluate_rules(&mut self, metrics: &SystemMetrics) -> Vec<u64> {
        let now = crate::resources::now_rfc3339();
        let mut emitted_ids = Vec::new();

        for i in 0..self.rules.len() {
            if !self.rules[i].enabled {
                continue;
            }

            // Check cooldown
            if let Some(ref last) = self.rules[i].last_triggered {
                if let (Ok(last_dt), Ok(now_dt)) = (
                    chrono::DateTime::parse_from_rfc3339(last),
                    chrono::DateTime::parse_from_rfc3339(&now),
                ) {
                    let elapsed = (now_dt - last_dt).num_seconds() as u64;
                    if elapsed < self.rules[i].cooldown_seconds {
                        continue;
                    }
                }
            }

            let workload_name = self.rules[i].workload.clone();
            let triggered = Self::rule_condition_met(
                &self.rules[i].condition,
                metrics,
                workload_name.as_deref(),
            );

            if triggered {
                let msg = self.rules[i].message_template.clone();
                let severity = self.rules[i].severity.clone();
                let category = match &self.rules[i].condition {
                    AlertCondition::SlaUptimeBelow(_) => EventCategory::SlaViolation,
                    AlertCondition::ErrorRateAbove(_) => EventCategory::SystemAlert,
                    AlertCondition::CostExceeds(_) => EventCategory::CostAnomaly,
                    AlertCondition::ExcessiveRestarts(_) => EventCategory::HealthCheck,
                    AlertCondition::DriftDetected => EventCategory::DriftDetected,
                    AlertCondition::PolicyViolation => EventCategory::PolicyViolation,
                    AlertCondition::SecretExpiring(_) => EventCategory::SecretRotation,
                };

                let id = self.emit_simple(
                    severity,
                    category,
                    "alert-evaluator",
                    workload_name.as_deref(),
                    &msg,
                    &msg,
                );
                emitted_ids.push(id);
                self.rules[i].last_triggered = Some(now.clone());
            }
        }

        emitted_ids
    }
}

// ─── Webhook Retry Queue ──────────────────────────────────────────────

/// A pending webhook delivery awaiting retry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingWebhook {
    /// Serialized JSON payload to send
    pub payload_json: String,
    /// Target URL
    pub url: String,
    /// HTTP method (GET or POST)
    pub method: String,
    /// Number of delivery attempts so far
    pub attempts: u32,
    /// Maximum delivery attempts before discarding
    pub max_attempts: u32,
    /// Earliest time to retry (RFC 3339)
    pub next_attempt_at: String,
    /// When the webhook was first queued
    pub created_at: String,
}

/// Persistent retry queue for failed webhook deliveries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookQueue {
    pub pending: Vec<PendingWebhook>,
}

crate::impl_json_store!(WebhookQueue, "webhook_queue.json");

impl WebhookQueue {
    /// Queue a pre-formatted JSON payload for delivery. Attempts immediate
    /// delivery first; on failure, the payload is persisted for later retry.
    /// This is used by Slack and other channels that build their own JSON.
    pub fn enqueue_raw(payload_json: &str, url: &str, method: &str) {
        let now = crate::resources::now_rfc3339();

        // Try immediate delivery
        if Self::try_deliver(payload_json, url, method) {
            return;
        }

        // Failed — queue for retry
        let path = Self::default_path();
        let mut queue = Self::load(&path).unwrap_or_default();
        queue.pending.push(PendingWebhook {
            payload_json: payload_json.to_string(),
            url: url.to_string(),
            method: method.to_string(),
            attempts: 1,
            max_attempts: 5,
            next_attempt_at: Self::backoff_time(&now, 1),
            created_at: now,
        });
        let _ = queue.save(&path);
        tracing::info!("Webhook queued for retry ({} pending)", queue.pending.len());
    }

    /// Queue a webhook for delivery. Attempts immediate delivery first;
    /// on failure, the webhook is persisted for later retry.
    pub fn enqueue(payload: &WebhookPayload, url: &str, method: &str) {
        let payload_json = serde_json::to_string(payload).unwrap_or_default();
        let now = crate::resources::now_rfc3339();

        // Try immediate delivery
        if Self::try_deliver(&payload_json, url, method) {
            return;
        }

        // Failed — queue for retry
        let path = Self::default_path();
        let mut queue = Self::load(&path).unwrap_or_default();
        queue.pending.push(PendingWebhook {
            payload_json,
            url: url.to_string(),
            method: method.to_string(),
            attempts: 1,
            max_attempts: 5,
            next_attempt_at: Self::backoff_time(&now, 1),
            created_at: now,
        });
        let _ = queue.save(&path);
        tracing::info!("Webhook queued for retry ({} pending)", queue.pending.len());
    }

    /// Process all pending webhooks that are due for retry.
    /// Removes successfully delivered or exhausted entries.
    pub fn process_queue_once() {
        let path = Self::default_path();
        let mut queue = match Self::load(&path) {
            Ok(q) if !q.pending.is_empty() => q,
            _ => return,
        };

        let now = crate::resources::now_rfc3339();
        let mut changed = false;

        queue.pending.retain_mut(|entry| {
            // Not yet due for retry
            if entry.next_attempt_at > now {
                return true;
            }

            entry.attempts += 1;

            if Self::try_deliver(&entry.payload_json, &entry.url, &entry.method) {
                tracing::info!(
                    "Webhook delivered to {} on retry #{}",
                    entry.url,
                    entry.attempts
                );
                changed = true;
                return false; // remove from queue
            }

            if entry.attempts >= entry.max_attempts {
                tracing::warn!(
                    "Webhook to {} exhausted {} attempts, discarding",
                    entry.url,
                    entry.max_attempts
                );
                changed = true;
                return false; // discard
            }

            // Schedule next retry with exponential backoff
            entry.next_attempt_at = Self::backoff_time(&now, entry.attempts);
            changed = true;
            true // keep in queue
        });

        if changed {
            let _ = queue.save(&path);
        }
    }

    /// Attempt a single HTTP delivery. Returns true on success.
    fn try_deliver(payload_json: &str, url: &str, method: &str) -> bool {
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        let result = if method.eq_ignore_ascii_case("GET") {
            client.get(url).body(payload_json.to_string()).send()
        } else {
            client
                .post(url)
                .header("Content-Type", "application/json")
                .body(payload_json.to_string())
                .send()
        };

        match result {
            Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => true,
            Ok(resp) => {
                tracing::warn!("Webhook to {} returned status {}", url, resp.status());
                false
            }
            Err(e) => {
                tracing::warn!("Webhook delivery to {} failed: {}", url, e);
                false
            }
        }
    }

    /// Calculate the next retry time using exponential backoff.
    /// Base: 30s, doubled each attempt, capped at 30 minutes.
    fn backoff_time(now: &str, attempt: u32) -> String {
        let base_secs: u64 = 30;
        let backoff_secs = base_secs.saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));
        let capped = backoff_secs.min(1800); // cap at 30 minutes
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(now) {
            let next = dt + chrono::Duration::seconds(capped as i64);
            next.to_rfc3339()
        } else {
            now.to_string()
        }
    }
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

        assert_eq!(
            bus.events_by_category(&EventCategory::DriftDetected).len(),
            1
        );
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
        bus.emit_simple(
            EventSeverity::Info,
            EventCategory::Deployment,
            "test",
            None,
            "Deploy",
            "ok",
        );
        bus.emit_simple(
            EventSeverity::Warning,
            EventCategory::DriftDetected,
            "test",
            None,
            "Drift",
            "found",
        );
        bus.emit_simple(
            EventSeverity::Critical,
            EventCategory::SlaViolation,
            "test",
            None,
            "SLA",
            "violated",
        );

        let summary = bus.summary();
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.critical_unacked, 1);
    }

    #[test]
    fn test_prune() {
        let mut bus = EventBus::new();
        for i in 0..100 {
            bus.emit_simple(
                EventSeverity::Info,
                EventCategory::Deployment,
                "test",
                None,
                &format!("Event {}", i),
                "msg",
            );
        }
        assert_eq!(bus.events().len(), 100);
        bus.prune(50);
        assert_eq!(bus.events().len(), 50);
    }

    #[test]
    fn test_format_event_list() {
        let mut bus = EventBus::new();
        bus.emit_simple(
            EventSeverity::Warning,
            EventCategory::DriftDetected,
            "test",
            Some("app"),
            "Drift",
            "found",
        );
        let events: Vec<&Event> = bus.events().iter().collect();
        let output = format_event_list(&events, 10);
        assert!(output.contains("Drift"));
        assert!(output.contains("app"));
    }

    #[test]
    fn test_format_summary() {
        let mut bus = EventBus::new();
        bus.emit_simple(
            EventSeverity::Info,
            EventCategory::Deployment,
            "test",
            None,
            "Deploy",
            "ok",
        );
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
        bus.emit_simple(
            EventSeverity::Info,
            EventCategory::Deployment,
            "test",
            None,
            "Info",
            "ok",
        );
        // Warning event should not match
        bus.emit_simple(
            EventSeverity::Warning,
            EventCategory::DriftDetected,
            "test",
            None,
            "Warn",
            "drift",
        );
        // Error event should match
        bus.emit_simple(
            EventSeverity::Error,
            EventCategory::Deployment,
            "test",
            None,
            "Error",
            "fail",
        );
        // Critical event should match
        bus.emit_simple(
            EventSeverity::Critical,
            EventCategory::SlaViolation,
            "test",
            None,
            "Critical",
            "bad",
        );

        assert_eq!(bus.events().len(), 4);
    }

    // ── evaluate_rules() ────────────────────────────────────────────────

    fn make_bus_with_rule(condition: AlertCondition, cooldown: u64) -> EventBus {
        let mut bus = EventBus::new();
        bus.rules = vec![AlertRule {
            name: "test-rule".to_string(),
            enabled: true,
            condition,
            severity: EventSeverity::Warning,
            message_template: "Test alert fired".to_string(),
            cooldown_seconds: cooldown,
            last_triggered: None,
            workload: None,
        }];
        bus
    }

    #[test]
    fn test_evaluate_rules_sla_uptime_below() {
        let mut bus = make_bus_with_rule(AlertCondition::SlaUptimeBelow(99.0), 0);
        let mut metrics = SystemMetrics::default();
        metrics.sla_uptimes.insert("web".to_string(), 98.5);

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 1);
        assert_eq!(bus.events().len(), 1);
        assert!(bus.events()[0].message.contains("Test alert fired"));
    }

    #[test]
    fn test_evaluate_rules_sla_above_threshold_does_not_fire() {
        let mut bus = make_bus_with_rule(AlertCondition::SlaUptimeBelow(99.0), 0);
        let mut metrics = SystemMetrics::default();
        metrics.sla_uptimes.insert("web".to_string(), 99.5);

        let fired = bus.evaluate_rules(&metrics);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_evaluate_rules_sla_zero_uptime_does_not_fire() {
        let mut bus = make_bus_with_rule(AlertCondition::SlaUptimeBelow(99.0), 0);
        let mut metrics = SystemMetrics::default();
        // 0.0 uptime means no data yet, should not trigger
        metrics.sla_uptimes.insert("web".to_string(), 0.0);

        let fired = bus.evaluate_rules(&metrics);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_evaluate_rules_excessive_restarts() {
        let mut bus = make_bus_with_rule(AlertCondition::ExcessiveRestarts(3), 0);
        let mut metrics = SystemMetrics::default();
        metrics.restart_counts.insert("api".to_string(), 5);

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_evaluate_rules_restarts_at_threshold_does_not_fire() {
        let mut bus = make_bus_with_rule(AlertCondition::ExcessiveRestarts(5), 0);
        let mut metrics = SystemMetrics::default();
        metrics.restart_counts.insert("api".to_string(), 5); // equal, not above

        let fired = bus.evaluate_rules(&metrics);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_evaluate_rules_drift_detected() {
        let mut bus = make_bus_with_rule(AlertCondition::DriftDetected, 0);
        let metrics = SystemMetrics {
            drift_detected: true,
            ..Default::default()
        };

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_evaluate_rules_policy_violation() {
        let mut bus = make_bus_with_rule(AlertCondition::PolicyViolation, 0);
        let metrics = SystemMetrics {
            policy_violations: true,
            ..Default::default()
        };

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_evaluate_rules_secret_expiring() {
        let mut bus = make_bus_with_rule(AlertCondition::SecretExpiring(14), 0);
        let mut metrics = SystemMetrics::default();
        metrics
            .secrets_expiring_days
            .insert("db-creds".to_string(), 10);

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_evaluate_rules_workload_scoped_sla() {
        let mut bus = EventBus::new();
        bus.rules = vec![AlertRule {
            name: "web-sla".to_string(),
            enabled: true,
            condition: AlertCondition::SlaUptimeBelow(99.0),
            severity: EventSeverity::Critical,
            message_template: "web SLA breach".to_string(),
            cooldown_seconds: 0,
            last_triggered: None,
            workload: Some("web".to_string()),
        }];
        let mut metrics = SystemMetrics::default();
        metrics.sla_uptimes.insert("web".to_string(), 98.0);
        metrics.sla_uptimes.insert("api".to_string(), 50.0);

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 1);
        assert_eq!(bus.events()[0].workload.as_deref(), Some("web"));
    }

    #[test]
    fn test_evaluate_rules_workload_scoped_ignores_other_workloads() {
        let mut bus = EventBus::new();
        bus.rules = vec![AlertRule {
            name: "web-sla".to_string(),
            enabled: true,
            condition: AlertCondition::SlaUptimeBelow(99.0),
            severity: EventSeverity::Critical,
            message_template: "web SLA breach".to_string(),
            cooldown_seconds: 0,
            last_triggered: None,
            workload: Some("web".to_string()),
        }];
        let mut metrics = SystemMetrics::default();
        metrics.sla_uptimes.insert("api".to_string(), 50.0);

        let fired = bus.evaluate_rules(&metrics);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_evaluate_rules_disabled_rule_does_not_fire() {
        let mut bus = EventBus::new();
        bus.rules = vec![AlertRule {
            name: "disabled-rule".to_string(),
            enabled: false,
            condition: AlertCondition::DriftDetected,
            severity: EventSeverity::Warning,
            message_template: "Should not fire".to_string(),
            cooldown_seconds: 0,
            last_triggered: None,
            workload: None,
        }];
        let metrics = SystemMetrics {
            drift_detected: true,
            ..Default::default()
        };

        let fired = bus.evaluate_rules(&metrics);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_evaluate_rules_cooldown_prevents_repeated_firing() {
        let mut bus = make_bus_with_rule(AlertCondition::DriftDetected, 3600);
        let metrics = SystemMetrics {
            drift_detected: true,
            ..Default::default()
        };

        // First evaluation should fire
        let fired1 = bus.evaluate_rules(&metrics);
        assert_eq!(fired1.len(), 1);

        // Second evaluation within cooldown should not fire
        let fired2 = bus.evaluate_rules(&metrics);
        assert!(fired2.is_empty());
    }

    #[test]
    fn test_evaluate_rules_multiple_rules_fire_independently() {
        let mut bus = EventBus::new();
        bus.rules = vec![
            AlertRule {
                name: "drift-alert".to_string(),
                enabled: true,
                condition: AlertCondition::DriftDetected,
                severity: EventSeverity::Warning,
                message_template: "Drift alert".to_string(),
                cooldown_seconds: 0,
                last_triggered: None,
                workload: None,
            },
            AlertRule {
                name: "restart-alert".to_string(),
                enabled: true,
                condition: AlertCondition::ExcessiveRestarts(2),
                severity: EventSeverity::Error,
                message_template: "Restart alert".to_string(),
                cooldown_seconds: 0,
                last_triggered: None,
                workload: None,
            },
        ];
        let mut metrics = SystemMetrics {
            drift_detected: true,
            ..Default::default()
        };
        metrics.restart_counts.insert("svc".to_string(), 10);

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 2);
        assert_eq!(bus.events().len(), 2);
    }

    #[test]
    fn test_evaluate_rules_empty_metrics_fires_nothing() {
        let mut bus = EventBus::new(); // has default rules
        let metrics = SystemMetrics::default();

        let fired = bus.evaluate_rules(&metrics);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_evaluate_rules_error_rate_above() {
        let mut bus = make_bus_with_rule(AlertCondition::ErrorRateAbove(5.0), 0);
        let mut metrics = SystemMetrics::default();
        metrics.error_rates.insert("api".to_string(), 8.0);

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_evaluate_rules_cost_exceeds_fleet() {
        let mut bus = make_bus_with_rule(AlertCondition::CostExceeds(100.0), 0);
        let metrics = SystemMetrics {
            fleet_monthly_cost_usd: 250.0,
            ..Default::default()
        };

        let fired = bus.evaluate_rules(&metrics);
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_event_severity_from_str() {
        assert_eq!(
            "info".parse::<EventSeverity>().unwrap(),
            EventSeverity::Info
        );
        assert_eq!(
            "warning".parse::<EventSeverity>().unwrap(),
            EventSeverity::Warning
        );
        assert_eq!(
            "warn".parse::<EventSeverity>().unwrap(),
            EventSeverity::Warning
        );
        assert_eq!(
            "error".parse::<EventSeverity>().unwrap(),
            EventSeverity::Error
        );
        assert_eq!(
            "critical".parse::<EventSeverity>().unwrap(),
            EventSeverity::Critical
        );
        assert_eq!(
            "CRITICAL".parse::<EventSeverity>().unwrap(),
            EventSeverity::Critical
        );
        assert!("debug".parse::<EventSeverity>().is_err());
    }

    // ── Slack notification tests ────────────────────────────────────────

    #[test]
    fn test_slack_channel_type_display() {
        let ct = ChannelType::Slack {
            webhook_url: "https://hooks.slack.com/services/T00/B00/xxx".to_string(),
            channel: "ops-alerts".to_string(),
        };
        assert_eq!(format!("{}", ct), "slack:#ops-alerts");
    }

    #[test]
    fn test_slack_channel_serialization_roundtrip() {
        let channel = NotificationChannel {
            name: "slack-ops".to_string(),
            channel_type: ChannelType::Slack {
                webhook_url: "https://hooks.slack.com/services/T00/B00/xxx".to_string(),
                channel: "ops-alerts".to_string(),
            },
            enabled: true,
            min_severity: EventSeverity::Warning,
            categories: vec![],
        };
        let json = serde_json::to_string(&channel).unwrap();
        let parsed: NotificationChannel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "slack-ops");
        assert!(matches!(parsed.channel_type, ChannelType::Slack { .. }));
        if let ChannelType::Slack {
            webhook_url,
            channel,
        } = &parsed.channel_type
        {
            assert_eq!(webhook_url, "https://hooks.slack.com/services/T00/B00/xxx");
            assert_eq!(channel, "ops-alerts");
        }
    }

    #[test]
    fn test_format_slack_payload_structure() {
        let notification = NotificationPayload {
            channel_name: "slack-ops".to_string(),
            channel_type: ChannelType::Slack {
                webhook_url: "https://hooks.slack.com/test".to_string(),
                channel: "ops".to_string(),
            },
            event_id: 1,
            title: "SLA Violated".to_string(),
            message: "Uptime dropped below 99.9%".to_string(),
            severity: EventSeverity::Critical,
            category: EventCategory::SlaViolation,
            workload: Some("web-app".to_string()),
        };

        let payload_str = format_slack_payload(&notification);
        let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

        // Verify top-level structure
        assert!(payload["attachments"].is_array());
        let attachment = &payload["attachments"][0];
        assert_eq!(attachment["color"], "#dc3545"); // Critical = red

        // Verify blocks
        let blocks = &attachment["blocks"];
        assert!(blocks.is_array());
        assert_eq!(blocks.as_array().unwrap().len(), 3);

        // Header block
        assert_eq!(blocks[0]["type"], "header");
        let header_text = blocks[0]["text"]["text"].as_str().unwrap();
        assert!(header_text.contains("Aether"));

        // Section with fields
        assert_eq!(blocks[1]["type"], "section");
        let fields = blocks[1]["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 2);
        assert!(fields[0]["text"].as_str().unwrap().contains("CRITICAL"));
        assert!(fields[1]["text"].as_str().unwrap().contains("web-app"));

        // Message section
        assert_eq!(blocks[2]["type"], "section");
        assert!(blocks[2]["text"]["text"]
            .as_str()
            .unwrap()
            .contains("Uptime dropped"));
    }

    #[test]
    fn test_format_slack_payload_severity_colors() {
        let make_payload = |severity: EventSeverity| {
            let notification = NotificationPayload {
                channel_name: "test".to_string(),
                channel_type: ChannelType::Console,
                event_id: 1,
                title: "Test".to_string(),
                message: "msg".to_string(),
                severity,
                category: EventCategory::SystemAlert,
                workload: None,
            };
            let json_str = format_slack_payload(&notification);
            let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
            v["attachments"][0]["color"].as_str().unwrap().to_string()
        };

        assert_eq!(make_payload(EventSeverity::Critical), "#dc3545");
        assert_eq!(make_payload(EventSeverity::Error), "#fd7e14");
        assert_eq!(make_payload(EventSeverity::Warning), "#ffc107");
        assert_eq!(make_payload(EventSeverity::Info), "#28a745");
    }

    #[test]
    fn test_format_slack_payload_no_workload() {
        let notification = NotificationPayload {
            channel_name: "test".to_string(),
            channel_type: ChannelType::Console,
            event_id: 1,
            title: "Test".to_string(),
            message: "msg".to_string(),
            severity: EventSeverity::Info,
            category: EventCategory::Deployment,
            workload: None,
        };
        let payload_str = format_slack_payload(&notification);
        let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap();
        let workload_field = payload["attachments"][0]["blocks"][1]["fields"][1]["text"]
            .as_str()
            .unwrap();
        assert!(workload_field.contains("(none)"));
    }

    #[test]
    fn test_slack_channel_add_remove() {
        let mut bus = EventBus::new();
        let initial = bus.channels().len();

        bus.add_channel(NotificationChannel {
            name: "slack-ops".to_string(),
            channel_type: ChannelType::Slack {
                webhook_url: "https://hooks.slack.com/services/T00/B00/xxx".to_string(),
                channel: "ops-alerts".to_string(),
            },
            enabled: true,
            min_severity: EventSeverity::Warning,
            categories: vec![],
        });
        assert_eq!(bus.channels().len(), initial + 1);

        assert!(bus.remove_channel("slack-ops"));
        assert_eq!(bus.channels().len(), initial);
    }

    // ---------------------------------------------------------------
    // PagerDuty, Email, Teams channel tests
    // ---------------------------------------------------------------

    #[test]
    fn test_pagerduty_channel_type_display() {
        let ct = ChannelType::PagerDuty {
            routing_key: "R0123456789".to_string(),
        };
        assert_eq!(ct.to_string(), "pagerduty");
    }

    #[test]
    fn test_email_channel_type_display() {
        let ct = ChannelType::Email {
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            from: "aether@example.com".to_string(),
            to: vec!["ops@example.com".to_string()],
        };
        assert_eq!(ct.to_string(), "email:aether@example.com");
    }

    #[test]
    fn test_teams_channel_type_display() {
        let ct = ChannelType::Teams {
            webhook_url: "https://outlook.office.com/webhook/xxx".to_string(),
        };
        assert_eq!(ct.to_string(), "teams");
    }

    #[test]
    fn test_pagerduty_channel_serialization_roundtrip() {
        let ct = ChannelType::PagerDuty {
            routing_key: "R0123456789".to_string(),
        };
        let json = serde_json::to_string(&ct).unwrap();
        let parsed: ChannelType = serde_json::from_str(&json).unwrap();
        assert_eq!(ct, parsed);
    }

    #[test]
    fn test_email_channel_serialization_roundtrip() {
        let ct = ChannelType::Email {
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            from: "aether@example.com".to_string(),
            to: vec!["ops@example.com".to_string(), "dev@example.com".to_string()],
        };
        let json = serde_json::to_string(&ct).unwrap();
        let parsed: ChannelType = serde_json::from_str(&json).unwrap();
        assert_eq!(ct, parsed);
    }

    #[test]
    fn test_teams_channel_serialization_roundtrip() {
        let ct = ChannelType::Teams {
            webhook_url: "https://outlook.office.com/webhook/xxx".to_string(),
        };
        let json = serde_json::to_string(&ct).unwrap();
        let parsed: ChannelType = serde_json::from_str(&json).unwrap();
        assert_eq!(ct, parsed);
    }

    #[test]
    fn test_format_pagerduty_payload_structure() {
        let notification = NotificationPayload {
            channel_name: "pd".to_string(),
            channel_type: ChannelType::Console,
            event_id: 42,
            title: "High CPU".to_string(),
            message: "CPU at 95%".to_string(),
            severity: EventSeverity::Critical,
            category: EventCategory::HealthCheck,
            workload: Some("web-app".to_string()),
        };
        let payload = format_pagerduty_payload(&notification, "R_KEY");
        let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(parsed["routing_key"], "R_KEY");
        assert_eq!(parsed["event_action"], "trigger");
        assert_eq!(parsed["payload"]["severity"], "critical");
        assert!(parsed["payload"]["summary"]
            .as_str()
            .unwrap()
            .contains("High CPU"));
    }

    #[test]
    fn test_format_teams_payload_structure() {
        let notification = NotificationPayload {
            channel_name: "teams".to_string(),
            channel_type: ChannelType::Console,
            event_id: 7,
            title: "Drift Detected".to_string(),
            message: "Config drift on db-primary".to_string(),
            severity: EventSeverity::Warning,
            category: EventCategory::DriftDetected,
            workload: Some("db-primary".to_string()),
        };
        let payload = format_teams_payload(&notification);
        let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(parsed["type"], "message");
        assert!(parsed["attachments"][0]["content"]["body"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Drift Detected"));
    }

    #[test]
    fn test_pagerduty_channel_add_remove() {
        let mut bus = EventBus::new();
        let initial = bus.channels().len();

        bus.add_channel(NotificationChannel {
            name: "pd-ops".to_string(),
            channel_type: ChannelType::PagerDuty {
                routing_key: "R_KEY_123".to_string(),
            },
            enabled: true,
            min_severity: EventSeverity::Error,
            categories: vec![],
        });
        assert_eq!(bus.channels().len(), initial + 1);

        assert!(bus.remove_channel("pd-ops"));
        assert_eq!(bus.channels().len(), initial);
    }
}
