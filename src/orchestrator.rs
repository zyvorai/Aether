// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Health-aware orchestrator
//!
//! Coordinates workload lifecycle with health monitoring,
//! automatic recovery, rolling updates, and circuit breaking.

use crate::output;
use crate::runtime::RuntimeKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health status of a workload
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    #[default]
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
            HealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub workload: String,
    pub status: HealthStatus,
    pub checks: Vec<CheckResult>,
    pub timestamp: String,
    pub consecutive_failures: u32,
}

/// Individual check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub latency_ms: Option<f64>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Check interval in seconds
    pub interval_seconds: u32,
    /// Number of failures before marking unhealthy
    pub failure_threshold: u32,
    /// Number of successes before marking healthy
    pub success_threshold: u32,
    /// Timeout for each check in seconds
    pub timeout_seconds: u32,
    /// Enable auto-restart on failure
    pub auto_restart: bool,
    /// Enable circuit breaker
    pub circuit_breaker: bool,
    /// Max restarts before circuit opens
    pub max_restarts: u32,
    /// Circuit breaker cooldown in seconds
    pub cooldown_seconds: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 30,
            failure_threshold: 3,
            success_threshold: 2,
            timeout_seconds: 5,
            auto_restart: true,
            circuit_breaker: true,
            max_restarts: 5,
            cooldown_seconds: 300,
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CircuitState {
    /// Normal operation
    #[default]
    Closed,
    /// Failing, auto-restart disabled temporarily
    Open,
    /// Testing recovery
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "Half-Open"),
        }
    }
}

/// Managed workload state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedWorkload {
    pub name: String,
    pub runtime: RuntimeKind,
    pub health_config: HealthConfig,
    pub current_health: HealthStatus,
    pub circuit: CircuitState,
    pub restart_count: u32,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_check: Option<String>,
    pub last_restart: Option<String>,
    pub circuit_opened_at: Option<String>,
    pub history: Vec<HealthEvent>,
}

/// Health event for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthEvent {
    pub timestamp: String,
    pub event_type: HealthEventType,
    pub message: String,
}

/// Types of health events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthEventType {
    HealthCheckPassed,
    HealthCheckFailed,
    StatusChanged,
    AutoRestart,
    CircuitOpened,
    CircuitHalfOpen,
    CircuitClosed,
    ManualIntervention,
}

impl std::fmt::Display for HealthEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthEventType::HealthCheckPassed => write!(f, "CHECK_PASS"),
            HealthEventType::HealthCheckFailed => write!(f, "CHECK_FAIL"),
            HealthEventType::StatusChanged => write!(f, "STATUS_CHANGE"),
            HealthEventType::AutoRestart => write!(f, "AUTO_RESTART"),
            HealthEventType::CircuitOpened => write!(f, "CIRCUIT_OPEN"),
            HealthEventType::CircuitHalfOpen => write!(f, "CIRCUIT_HALF_OPEN"),
            HealthEventType::CircuitClosed => write!(f, "CIRCUIT_CLOSED"),
            HealthEventType::ManualIntervention => write!(f, "MANUAL"),
        }
    }
}

/// Rolling update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingUpdateConfig {
    pub max_unavailable: u32,
    pub max_surge: u32,
    pub health_check_delay_seconds: u32,
    pub rollback_on_failure: bool,
}

impl Default for RollingUpdateConfig {
    fn default() -> Self {
        Self {
            max_unavailable: 1,
            max_surge: 1,
            health_check_delay_seconds: 30,
            rollback_on_failure: true,
        }
    }
}

/// Rolling update status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingUpdateStatus {
    pub workload: String,
    pub phase: UpdatePhase,
    pub total_replicas: u32,
    pub updated_replicas: u32,
    pub available_replicas: u32,
    pub message: String,
}

/// Rolling update phase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpdatePhase {
    Pending,
    InProgress,
    Completed,
    RolledBack,
    Failed,
}

impl std::fmt::Display for UpdatePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdatePhase::Pending => write!(f, "Pending"),
            UpdatePhase::InProgress => write!(f, "In Progress"),
            UpdatePhase::Completed => write!(f, "Completed"),
            UpdatePhase::RolledBack => write!(f, "Rolled Back"),
            UpdatePhase::Failed => write!(f, "Failed"),
        }
    }
}

/// The orchestrator
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Orchestrator {
    workloads: HashMap<String, ManagedWorkload>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a workload for health management
    pub fn register(
        &mut self,
        name: &str,
        runtime: RuntimeKind,
        config: Option<HealthConfig>,
    ) -> &ManagedWorkload {
        let workload = ManagedWorkload {
            name: name.to_string(),
            runtime,
            health_config: config.unwrap_or_default(),
            current_health: HealthStatus::Unknown,
            circuit: CircuitState::Closed,
            restart_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_check: None,
            last_restart: None,
            circuit_opened_at: None,
            history: vec![HealthEvent {
                timestamp: crate::resources::now_rfc3339(),
                event_type: HealthEventType::StatusChanged,
                message: "Workload registered for health monitoring".to_string(),
            }],
        };
        self.workloads.insert(name.to_string(), workload);
        // SAFETY: we just inserted this key on the line above and hold &mut self,
        // so no concurrent removal is possible.
        self.workloads.get(name).unwrap_or_else(|| {
            unreachable!(
                "workload '{}' was just inserted but not found in HashMap",
                name
            )
        })
    }

    /// Unregister a workload
    pub fn unregister(&mut self, name: &str) -> Option<ManagedWorkload> {
        self.workloads.remove(name)
    }

    /// Process a health check result
    pub fn process_health_check(&mut self, check: HealthCheck) -> Vec<OrchestratorAction> {
        let mut actions = Vec::new();
        let now = crate::resources::now_rfc3339();

        let workload = match self.workloads.get_mut(&check.workload) {
            Some(w) => w,
            None => return actions,
        };

        workload.last_check = Some(now.clone());
        let previous_health = workload.current_health;

        match check.status {
            HealthStatus::Healthy => {
                workload.consecutive_failures = 0;
                workload.consecutive_successes += 1;

                if workload.consecutive_successes >= workload.health_config.success_threshold {
                    workload.current_health = HealthStatus::Healthy;

                    // Close circuit if half-open and recovery confirmed
                    if workload.circuit == CircuitState::HalfOpen {
                        workload.circuit = CircuitState::Closed;
                        workload.restart_count = 0;
                        workload.history.push(HealthEvent {
                            timestamp: now.clone(),
                            event_type: HealthEventType::CircuitClosed,
                            message: "Circuit closed - recovery confirmed".to_string(),
                        });
                        actions.push(OrchestratorAction::CircuitClosed {
                            workload: check.workload.clone(),
                        });
                    }
                }

                workload.history.push(HealthEvent {
                    timestamp: now.clone(),
                    event_type: HealthEventType::HealthCheckPassed,
                    message: format!(
                        "Health check passed ({} consecutive)",
                        workload.consecutive_successes
                    ),
                });
            }
            HealthStatus::Degraded => {
                workload.consecutive_successes = 0;
                workload.current_health = HealthStatus::Degraded;

                workload.history.push(HealthEvent {
                    timestamp: now.clone(),
                    event_type: HealthEventType::HealthCheckFailed,
                    message: "Workload degraded".to_string(),
                });

                actions.push(OrchestratorAction::Alert {
                    workload: check.workload.clone(),
                    message: "Workload health degraded".to_string(),
                });
            }
            HealthStatus::Unhealthy | HealthStatus::Unknown => {
                workload.consecutive_successes = 0;
                workload.consecutive_failures += 1;

                workload.history.push(HealthEvent {
                    timestamp: now.clone(),
                    event_type: HealthEventType::HealthCheckFailed,
                    message: format!(
                        "Health check failed ({} consecutive)",
                        workload.consecutive_failures
                    ),
                });

                if workload.consecutive_failures >= workload.health_config.failure_threshold {
                    workload.current_health = HealthStatus::Unhealthy;

                    // Handle based on circuit state
                    match workload.circuit {
                        CircuitState::Closed => {
                            if workload.health_config.auto_restart {
                                if workload.restart_count < workload.health_config.max_restarts {
                                    workload.restart_count += 1;
                                    workload.last_restart = Some(now.clone());
                                    workload.consecutive_failures = 0;

                                    workload.history.push(HealthEvent {
                                        timestamp: now.clone(),
                                        event_type: HealthEventType::AutoRestart,
                                        message: format!(
                                            "Auto-restart #{} triggered",
                                            workload.restart_count
                                        ),
                                    });

                                    actions.push(OrchestratorAction::Restart {
                                        workload: check.workload.clone(),
                                        runtime: workload.runtime,
                                        reason: "Health check failures exceeded threshold"
                                            .to_string(),
                                    });
                                } else if workload.health_config.circuit_breaker {
                                    // Open circuit
                                    workload.circuit = CircuitState::Open;
                                    workload.circuit_opened_at = Some(now.clone());

                                    workload.history.push(HealthEvent {
                                        timestamp: now.clone(),
                                        event_type: HealthEventType::CircuitOpened,
                                        message: format!(
                                            "Circuit breaker opened after {} restarts",
                                            workload.restart_count
                                        ),
                                    });

                                    actions.push(OrchestratorAction::CircuitOpened {
                                        workload: check.workload.clone(),
                                        reason: format!(
                                            "Max restarts ({}) exceeded",
                                            workload.health_config.max_restarts
                                        ),
                                    });
                                }
                            }
                        }
                        CircuitState::Open => {
                            // Check if cooldown has elapsed
                            if let Some(opened_at) = &workload.circuit_opened_at {
                                match chrono::DateTime::parse_from_rfc3339(opened_at) {
                                    Ok(opened) => {
                                        let elapsed =
                                            chrono::Utc::now() - opened.with_timezone(&chrono::Utc);
                                        if elapsed.num_seconds()
                                            >= workload.health_config.cooldown_seconds as i64
                                        {
                                            workload.circuit = CircuitState::HalfOpen;
                                            workload.restart_count = 0;
                                            workload.consecutive_failures = 0;

                                            workload.history.push(HealthEvent {
                                                timestamp: now.clone(),
                                                event_type: HealthEventType::CircuitHalfOpen,
                                                message: "Circuit half-open - attempting recovery"
                                                    .to_string(),
                                            });

                                            actions.push(OrchestratorAction::Restart {
                                                workload: check.workload.clone(),
                                                runtime: workload.runtime,
                                                reason: "Circuit half-open recovery attempt"
                                                    .to_string(),
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        // Corrupted timestamp: transition to HalfOpen to
                                        // allow automatic recovery rather than permanently
                                        // locking the circuit in Open state.
                                        tracing::error!(
                                            "Failed to parse circuit_opened_at '{}': {}; \
                                             forcing transition to HalfOpen for recovery",
                                            opened_at,
                                            e
                                        );
                                        workload.circuit = CircuitState::HalfOpen;
                                        workload.restart_count = 0;
                                        workload.consecutive_failures = 0;

                                        workload.history.push(HealthEvent {
                                            timestamp: now.clone(),
                                            event_type: HealthEventType::CircuitHalfOpen,
                                            message: format!(
                                                "Corrupted circuit_opened_at timestamp — forced HalfOpen for recovery: {}",
                                                e
                                            ),
                                        });

                                        actions.push(OrchestratorAction::Restart {
                                            workload: check.workload.clone(),
                                            runtime: workload.runtime,
                                            reason:
                                                "Circuit forced HalfOpen after corrupted timestamp"
                                                    .to_string(),
                                        });
                                    }
                                }
                            }
                        }
                        CircuitState::HalfOpen => {
                            // Failed during half-open, reopen circuit
                            workload.circuit = CircuitState::Open;
                            workload.circuit_opened_at = Some(now.clone());

                            workload.history.push(HealthEvent {
                                timestamp: now.clone(),
                                event_type: HealthEventType::CircuitOpened,
                                message: "Recovery failed - circuit reopened".to_string(),
                            });

                            actions.push(OrchestratorAction::CircuitOpened {
                                workload: check.workload.clone(),
                                reason: "Recovery attempt failed during half-open state"
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Emit status change event
        if previous_health != workload.current_health {
            workload.history.push(HealthEvent {
                timestamp: now,
                event_type: HealthEventType::StatusChanged,
                message: format!("{} -> {}", previous_health, workload.current_health),
            });
            actions.push(OrchestratorAction::StatusChanged {
                workload: check.workload.clone(),
                from: previous_health,
                to: workload.current_health,
            });
        }

        // Trim history — keep the most recent 100 events.
        // Use a single drain to avoid unbounded growth.
        let max_history = 100;
        if workload.history.len() > max_history {
            let excess = workload.history.len() - max_history;
            workload.history.drain(..excess);
        }

        actions
    }

    /// Manually reset circuit breaker
    pub fn reset_circuit(&mut self, name: &str) -> bool {
        if let Some(workload) = self.workloads.get_mut(name) {
            workload.circuit = CircuitState::Closed;
            workload.restart_count = 0;
            workload.consecutive_failures = 0;
            workload.circuit_opened_at = None;
            workload.history.push(HealthEvent {
                timestamp: crate::resources::now_rfc3339(),
                event_type: HealthEventType::ManualIntervention,
                message: "Circuit breaker manually reset".to_string(),
            });
            true
        } else {
            false
        }
    }

    /// Get workload health info
    pub fn get_workload(&self, name: &str) -> Option<&ManagedWorkload> {
        self.workloads.get(name)
    }

    /// List all managed workloads
    pub fn list_workloads(&self) -> Vec<WorkloadSummary> {
        self.workloads
            .values()
            .map(|w| WorkloadSummary {
                name: w.name.clone(),
                runtime: w.runtime,
                health: w.current_health,
                circuit: w.circuit,
                restart_count: w.restart_count,
                last_check: w.last_check.clone(),
            })
            .collect()
    }

    /// Get overall health summary
    pub fn health_summary(&self) -> HealthSummary {
        let total = self.workloads.len();
        let healthy = self
            .workloads
            .values()
            .filter(|w| w.current_health == HealthStatus::Healthy)
            .count();
        let degraded = self
            .workloads
            .values()
            .filter(|w| w.current_health == HealthStatus::Degraded)
            .count();
        let unhealthy = self
            .workloads
            .values()
            .filter(|w| w.current_health == HealthStatus::Unhealthy)
            .count();
        let unknown = self
            .workloads
            .values()
            .filter(|w| w.current_health == HealthStatus::Unknown)
            .count();
        let circuits_open = self
            .workloads
            .values()
            .filter(|w| w.circuit == CircuitState::Open)
            .count();

        HealthSummary {
            total_workloads: total,
            healthy,
            degraded,
            unhealthy,
            unknown,
            circuits_open,
        }
    }

    /// Simulate a rolling update
    pub fn rolling_update(
        &self,
        workload: &str,
        replicas: u32,
        config: Option<RollingUpdateConfig>,
    ) -> Vec<RollingUpdateStatus> {
        let config = config.unwrap_or_default();
        let mut statuses = Vec::new();

        let mut updated = 0;
        let mut available = replicas;

        // Simulate rolling update steps
        while updated < replicas {
            let batch = config.max_surge.min(replicas - updated);
            let unavailable = batch.min(config.max_unavailable);
            available = available.saturating_sub(unavailable);
            updated += batch;
            available += batch;

            let phase = if updated >= replicas {
                UpdatePhase::Completed
            } else {
                UpdatePhase::InProgress
            };

            statuses.push(RollingUpdateStatus {
                workload: workload.to_string(),
                phase,
                total_replicas: replicas,
                updated_replicas: updated,
                available_replicas: available.min(replicas),
                message: format!(
                    "Updated {}/{} replicas, {} available",
                    updated,
                    replicas,
                    available.min(replicas)
                ),
            });
        }

        statuses
    }

    /// Process health checks from a map of workload name → live HealthStatus.
    /// For each entry matching a registered workload, builds a HealthCheck
    /// and delegates to process_health_check().
    pub fn run_health_checks_from_statuses(
        &mut self,
        statuses: &HashMap<String, HealthStatus>,
    ) -> Vec<OrchestratorAction> {
        let mut all_actions = Vec::new();
        let now = crate::resources::now_rfc3339();

        // Collect registered names first to avoid borrow issues
        let registered: Vec<String> = self.workloads.keys().cloned().collect();

        for name in registered {
            if let Some(&status) = statuses.get(&name) {
                let check = HealthCheck {
                    workload: name.clone(),
                    status,
                    checks: vec![CheckResult {
                        name: "runtime-status".to_string(),
                        passed: status == HealthStatus::Healthy,
                        message: format!("Runtime reports: {}", status),
                        latency_ms: None,
                    }],
                    timestamp: now.clone(),
                    consecutive_failures: 0,
                };
                let actions = self.process_health_check(check);
                all_actions.extend(actions);
            }
        }

        all_actions
    }
}

crate::impl_json_store!(Orchestrator, "orchestrator.json");

/// Actions the orchestrator may request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorAction {
    Restart {
        workload: String,
        runtime: RuntimeKind,
        reason: String,
    },
    Alert {
        workload: String,
        message: String,
    },
    StatusChanged {
        workload: String,
        from: HealthStatus,
        to: HealthStatus,
    },
    CircuitOpened {
        workload: String,
        reason: String,
    },
    CircuitClosed {
        workload: String,
    },
}

impl std::fmt::Display for OrchestratorAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestratorAction::Restart {
                workload, reason, ..
            } => {
                write!(f, "RESTART '{}': {}", workload, reason)
            }
            OrchestratorAction::Alert { workload, message } => {
                write!(f, "ALERT '{}': {}", workload, message)
            }
            OrchestratorAction::StatusChanged { workload, from, to } => {
                write!(f, "STATUS '{}': {} -> {}", workload, from, to)
            }
            OrchestratorAction::CircuitOpened { workload, reason } => {
                write!(f, "CIRCUIT OPEN '{}': {}", workload, reason)
            }
            OrchestratorAction::CircuitClosed { workload } => {
                write!(f, "CIRCUIT CLOSED '{}'", workload)
            }
        }
    }
}

/// Workload summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSummary {
    pub name: String,
    pub runtime: RuntimeKind,
    pub health: HealthStatus,
    pub circuit: CircuitState,
    pub restart_count: u32,
    pub last_check: Option<String>,
}

/// Overall health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_workloads: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub unknown: usize,
    pub circuits_open: usize,
}

/// Format workload health list
pub fn format_workload_list(workloads: &[WorkloadSummary]) -> String {
    let mut output = String::new();
    output.push_str("Managed Workloads:\n\n");

    if workloads.is_empty() {
        output.push_str("  No workloads registered.\n");
        return output;
    }

    output.push_str(&format!(
        "  {:<20} {:<14} {:<12} {:<12} {:>8}\n",
        "Workload", "Runtime", "Health", "Circuit", "Restarts"
    ));
    output.push_str(&format!("  {}\n", "-".repeat(70)));

    for w in workloads {
        output.push_str(&format!(
            "  {:<20} {:<14} {:<12} {:<12} {:>8}\n",
            w.name,
            w.runtime.to_string(),
            w.health.to_string(),
            w.circuit.to_string(),
            w.restart_count,
        ));
    }

    output
}

/// Format health summary
pub fn format_health_summary(summary: &HealthSummary) -> String {
    let mut output = String::new();
    output.push_str(&output::property_section(&[
        ("Health Summary", String::new()),
        ("Total Workloads", format!("{}", summary.total_workloads)),
        ("Healthy", format!("{}", summary.healthy)),
        ("Degraded", format!("{}", summary.degraded)),
        ("Unhealthy", format!("{}", summary.unhealthy)),
        ("Unknown", format!("{}", summary.unknown)),
    ]));
    if summary.circuits_open > 0 {
        output.push_str(&format!(
            "\n  Circuits Open: {} (requires manual intervention)\n",
            summary.circuits_open
        ));
    }
    output
}

/// Format rolling update status
pub fn format_rolling_update(statuses: &[RollingUpdateStatus]) -> String {
    let mut output = String::new();
    if let Some(last) = statuses.last() {
        output.push_str(&format!("Rolling Update: {}\n\n", last.workload));
        for (i, status) in statuses.iter().enumerate() {
            output.push_str(&format!(
                "  Step {}: {} - {}\n",
                i + 1,
                status.phase,
                status.message
            ));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_check(workload: &str, status: HealthStatus) -> HealthCheck {
        HealthCheck {
            workload: workload.to_string(),
            status,
            checks: vec![CheckResult {
                name: "http".to_string(),
                passed: status == HealthStatus::Healthy,
                message: "HTTP check".to_string(),
                latency_ms: Some(50.0),
            }],
            timestamp: crate::resources::now_rfc3339(),
            consecutive_failures: 0,
        }
    }

    #[test]
    fn test_register_and_list() {
        let mut orch = Orchestrator::new();
        orch.register("web-app", RuntimeKind::Kubernetes, None);
        orch.register("worker", RuntimeKind::Podman, None);

        let list = orch.list_workloads();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_healthy_check_updates_status() {
        let mut orch = Orchestrator::new();
        orch.register("app", RuntimeKind::Kubernetes, None);

        // Send enough healthy checks to pass success_threshold (default 2)
        for _ in 0..3 {
            orch.process_health_check(make_check("app", HealthStatus::Healthy));
        }

        let w = orch.get_workload("app").unwrap();
        assert_eq!(w.current_health, HealthStatus::Healthy);
        assert_eq!(w.circuit, CircuitState::Closed);
    }

    #[test]
    fn test_unhealthy_triggers_restart() {
        let mut orch = Orchestrator::new();
        let config = HealthConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        orch.register("app", RuntimeKind::Kubernetes, Some(config));

        // Send failing checks to exceed threshold
        let mut restart_issued = false;
        for _ in 0..3 {
            let actions = orch.process_health_check(make_check("app", HealthStatus::Unhealthy));
            if actions
                .iter()
                .any(|a| matches!(a, OrchestratorAction::Restart { .. }))
            {
                restart_issued = true;
            }
        }

        assert!(restart_issued);
        let w = orch.get_workload("app").unwrap();
        assert!(w.restart_count > 0);
    }

    #[test]
    fn test_circuit_breaker_opens() {
        let mut orch = Orchestrator::new();
        let config = HealthConfig {
            failure_threshold: 1,
            max_restarts: 2,
            ..Default::default()
        };
        orch.register("app", RuntimeKind::Kubernetes, Some(config));

        let mut circuit_opened = false;
        // Keep failing until circuit opens
        for _ in 0..10 {
            let actions = orch.process_health_check(make_check("app", HealthStatus::Unhealthy));
            if actions
                .iter()
                .any(|a| matches!(a, OrchestratorAction::CircuitOpened { .. }))
            {
                circuit_opened = true;
                break;
            }
        }

        assert!(circuit_opened);
        let w = orch.get_workload("app").unwrap();
        assert_eq!(w.circuit, CircuitState::Open);
    }

    #[test]
    fn test_manual_circuit_reset() {
        let mut orch = Orchestrator::new();
        let config = HealthConfig {
            failure_threshold: 1,
            max_restarts: 1,
            ..Default::default()
        };
        orch.register("app", RuntimeKind::Kubernetes, Some(config));

        // Force circuit open
        for _ in 0..5 {
            orch.process_health_check(make_check("app", HealthStatus::Unhealthy));
        }

        assert!(orch.reset_circuit("app"));
        let w = orch.get_workload("app").unwrap();
        assert_eq!(w.circuit, CircuitState::Closed);
        assert_eq!(w.restart_count, 0);
    }

    #[test]
    fn test_health_summary() {
        let mut orch = Orchestrator::new();
        orch.register("healthy-app", RuntimeKind::Kubernetes, None);
        orch.register("unknown-app", RuntimeKind::Podman, None);

        // Make one healthy
        for _ in 0..3 {
            orch.process_health_check(make_check("healthy-app", HealthStatus::Healthy));
        }

        let summary = orch.health_summary();
        assert_eq!(summary.total_workloads, 2);
        assert_eq!(summary.healthy, 1);
        assert_eq!(summary.unknown, 1);
    }

    #[test]
    fn test_rolling_update() {
        let orch = Orchestrator::new();
        let statuses = orch.rolling_update("web-app", 4, None);
        assert!(!statuses.is_empty());
        assert_eq!(statuses.last().unwrap().phase, UpdatePhase::Completed);
        assert_eq!(statuses.last().unwrap().updated_replicas, 4);
    }

    #[test]
    fn test_degraded_status() {
        let mut orch = Orchestrator::new();
        orch.register("app", RuntimeKind::Kubernetes, None);

        let actions = orch.process_health_check(make_check("app", HealthStatus::Degraded));
        let w = orch.get_workload("app").unwrap();
        assert_eq!(w.current_health, HealthStatus::Degraded);
        assert!(actions
            .iter()
            .any(|a| matches!(a, OrchestratorAction::Alert { .. })));
    }

    #[test]
    fn test_format_workload_list() {
        let mut orch = Orchestrator::new();
        orch.register("web", RuntimeKind::Kubernetes, None);
        let list = orch.list_workloads();
        let output = format_workload_list(&list);
        assert!(output.contains("web"));
        assert!(output.contains("kubernetes"));
    }

    #[test]
    fn test_format_health_summary() {
        let orch = Orchestrator::new();
        let summary = orch.health_summary();
        let output = format_health_summary(&summary);
        assert!(output.contains("Health Summary"));
    }

    #[test]
    fn test_unregister() {
        let mut orch = Orchestrator::new();
        orch.register("app", RuntimeKind::Kubernetes, None);
        assert!(orch.unregister("app").is_some());
        assert!(orch.get_workload("app").is_none());
    }

    #[test]
    fn test_run_health_checks_basic() {
        let mut orch = Orchestrator::new();
        orch.register("web", RuntimeKind::Kubernetes, None);
        orch.register("worker", RuntimeKind::Podman, None);

        let mut statuses = HashMap::new();
        statuses.insert("web".to_string(), HealthStatus::Healthy);
        statuses.insert("worker".to_string(), HealthStatus::Healthy);

        let actions = orch.run_health_checks_from_statuses(&statuses);
        // After enough healthy checks, should see status changes
        // First round may or may not produce actions depending on success_threshold
        let _ = actions;

        // Send multiple rounds to exceed default success_threshold (2)
        for _ in 0..3 {
            orch.run_health_checks_from_statuses(&statuses);
        }

        let web = orch.get_workload("web").unwrap();
        assert_eq!(web.current_health, HealthStatus::Healthy);
        let worker = orch.get_workload("worker").unwrap();
        assert_eq!(worker.current_health, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_checks_ignores_unregistered() {
        let mut orch = Orchestrator::new();
        orch.register("web", RuntimeKind::Kubernetes, None);

        let mut statuses = HashMap::new();
        statuses.insert("web".to_string(), HealthStatus::Healthy);
        statuses.insert("unknown-svc".to_string(), HealthStatus::Unhealthy);

        let actions = orch.run_health_checks_from_statuses(&statuses);
        // Should not produce any action for unknown-svc
        assert!(!actions.iter().any(|a| match a {
            OrchestratorAction::Restart { workload, .. } => workload == "unknown-svc",
            OrchestratorAction::Alert { workload, .. } => workload == "unknown-svc",
            OrchestratorAction::StatusChanged { workload, .. } => workload == "unknown-svc",
            OrchestratorAction::CircuitOpened { workload, .. } => workload == "unknown-svc",
            OrchestratorAction::CircuitClosed { workload } => workload == "unknown-svc",
        }));
    }

    #[test]
    fn test_health_transitions() {
        let mut orch = Orchestrator::new();
        let config = HealthConfig {
            failure_threshold: 2,
            success_threshold: 2,
            ..Default::default()
        };
        orch.register("app", RuntimeKind::Kubernetes, Some(config));

        // Start healthy
        let mut statuses = HashMap::new();
        statuses.insert("app".to_string(), HealthStatus::Healthy);
        for _ in 0..3 {
            orch.run_health_checks_from_statuses(&statuses);
        }
        assert_eq!(
            orch.get_workload("app").unwrap().current_health,
            HealthStatus::Healthy
        );

        // Transition to unhealthy
        statuses.insert("app".to_string(), HealthStatus::Unhealthy);
        for _ in 0..3 {
            orch.run_health_checks_from_statuses(&statuses);
        }
        assert_eq!(
            orch.get_workload("app").unwrap().current_health,
            HealthStatus::Unhealthy
        );
    }
}
