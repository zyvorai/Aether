//! Prometheus metrics for Orchestr8
//!
//! Tracks workload deployments, runtime distribution, and migration operations.

use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry,
};

lazy_static! {
    /// Global metrics registry
    pub static ref REGISTRY: Registry = Registry::new();

    // Workload metrics
    /// Total number of workload build operations
    pub static ref WORKLOAD_BUILDS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("orchestr8_workload_builds_total", "Total number of workload builds")
            .namespace("orchestr8"),
        &["runtime", "status"]
    )
    .expect("metric can be created");

    /// Total number of workload deployments
    pub static ref WORKLOAD_DEPLOYMENTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "orchestr8_workload_deployments_total",
            "Total number of workload deployments"
        )
        .namespace("orchestr8"),
        &["runtime", "status"]
    )
    .expect("metric can be created");

    /// Currently running workloads
    pub static ref WORKLOAD_RUNNING: GaugeVec = GaugeVec::new(
        Opts::new("orchestr8_workload_running", "Number of currently running workloads")
            .namespace("orchestr8"),
        &["runtime"]
    )
    .expect("metric can be created");

    /// Workload state distribution
    pub static ref WORKLOAD_STATE: GaugeVec = GaugeVec::new(
        Opts::new("orchestr8_workload_state", "Workload state distribution")
            .namespace("orchestr8"),
        &["runtime", "state"]
    )
    .expect("metric can be created");

    // Migration metrics
    /// Total number of migrations
    pub static ref MIGRATIONS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("orchestr8_migrations_total", "Total number of migrations")
            .namespace("orchestr8"),
        &["source_runtime", "target_runtime", "strategy", "status"]
    )
    .expect("metric can be created");

    /// Migration duration histogram
    pub static ref MIGRATION_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "orchestr8_migration_duration_seconds",
            "Migration duration in seconds"
        )
        .namespace("orchestr8")
        .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0]),
        &["source_runtime", "target_runtime", "strategy"]
    )
    .expect("metric can be created");

    /// Migration rollbacks
    pub static ref MIGRATION_ROLLBACKS_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "orchestr8_migration_rollbacks_total",
            "Total number of migration rollbacks"
        )
        .namespace("orchestr8"),
        &["source_runtime", "target_runtime", "strategy"]
    )
    .expect("metric can be created");

    // Runtime metrics
    /// Runtime availability
    pub static ref RUNTIME_AVAILABLE: GaugeVec = GaugeVec::new(
        Opts::new("orchestr8_runtime_available", "Runtime availability (1=available, 0=unavailable)")
            .namespace("orchestr8"),
        &["runtime"]
    )
    .expect("metric can be created");

    /// Runtime decision time
    pub static ref RUNTIME_DECISION_SECONDS: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "orchestr8_runtime_decision_seconds",
            "Runtime decision time in seconds"
        )
        .namespace("orchestr8")
        .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
    )
    .expect("metric can be created");

    // System metrics
    /// Orchestr8 version info
    pub static ref ORCHESTR8_INFO: Counter = Counter::with_opts(
        Opts::new("orchestr8_build_info", "Orchestr8 version and build information")
            .namespace("orchestr8")
            .const_label("version", env!("CARGO_PKG_VERSION"))
    )
    .expect("metric can be created");

    /// Total CLI commands executed
    pub static ref CLI_COMMANDS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("orchestr8_cli_commands_total", "Total CLI commands executed")
            .namespace("orchestr8"),
        &["command"]
    )
    .expect("metric can be created");

    /// Command execution duration
    pub static ref COMMAND_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "orchestr8_command_duration_seconds",
            "Command execution duration in seconds"
        )
        .namespace("orchestr8")
        .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]),
        &["command"]
    )
    .expect("metric can be created");

    // Scheduler metrics
    /// Total scheduler placement decisions
    pub static ref SCHEDULER_PLACEMENTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "orchestr8_scheduler_placements_total",
            "Total scheduler placement decisions"
        )
        .namespace("orchestr8"),
        &["runtime", "strategy", "status"]
    )
    .expect("metric can be created");

    // Orchestrator / health metrics
    /// Health check results
    pub static ref HEALTH_CHECKS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("orchestr8_health_checks_total", "Total health check results")
            .namespace("orchestr8"),
        &["workload", "status"]
    )
    .expect("metric can be created");

    /// Circuit breaker state changes
    pub static ref CIRCUIT_BREAKER_EVENTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "orchestr8_circuit_breaker_events_total",
            "Circuit breaker state change events"
        )
        .namespace("orchestr8"),
        &["workload", "state"]
    )
    .expect("metric can be created");

    /// Auto-restarts triggered by orchestrator
    pub static ref ORCHESTRATOR_RESTARTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "orchestr8_orchestrator_restarts_total",
            "Auto-restarts triggered by orchestrator"
        )
        .namespace("orchestr8"),
        &["workload", "runtime"]
    )
    .expect("metric can be created");

    // Secrets metrics
    /// Secret operations
    pub static ref SECRET_OPERATIONS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("orchestr8_secret_operations_total", "Secret management operations")
            .namespace("orchestr8"),
        &["operation"]
    )
    .expect("metric can be created");

    // Events metrics
    /// Events emitted by category
    pub static ref EVENTS_EMITTED_TOTAL: CounterVec = CounterVec::new(
        Opts::new("orchestr8_events_emitted_total", "Events emitted by category")
            .namespace("orchestr8"),
        &["category", "severity"]
    )
    .expect("metric can be created");

    // Environment metrics
    /// Environment promotions
    pub static ref ENV_PROMOTIONS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("orchestr8_env_promotions_total", "Environment promotion operations")
            .namespace("orchestr8"),
        &["from_tier", "to_tier", "status"]
    )
    .expect("metric can be created");

    // Affinity metrics
    /// Affinity recommendations served
    pub static ref AFFINITY_RECOMMENDATIONS_TOTAL: CounterVec = CounterVec::new(
        Opts::new(
            "orchestr8_affinity_recommendations_total",
            "Affinity recommendations served"
        )
        .namespace("orchestr8"),
        &["workload_class", "recommended_runtime"]
    )
    .expect("metric can be created");
}

/// Initialize metrics registry
pub fn init() {
    // Register all metrics (ignore errors if already registered)
    let _ = REGISTRY.register(Box::new(WORKLOAD_BUILDS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(WORKLOAD_DEPLOYMENTS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(WORKLOAD_RUNNING.clone()));
    let _ = REGISTRY.register(Box::new(WORKLOAD_STATE.clone()));
    let _ = REGISTRY.register(Box::new(MIGRATIONS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(MIGRATION_DURATION_SECONDS.clone()));
    let _ = REGISTRY.register(Box::new(MIGRATION_ROLLBACKS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(RUNTIME_AVAILABLE.clone()));
    let _ = REGISTRY.register(Box::new(RUNTIME_DECISION_SECONDS.clone()));
    let _ = REGISTRY.register(Box::new(ORCHESTR8_INFO.clone()));
    let _ = REGISTRY.register(Box::new(CLI_COMMANDS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(COMMAND_DURATION_SECONDS.clone()));
    let _ = REGISTRY.register(Box::new(SCHEDULER_PLACEMENTS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(HEALTH_CHECKS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(CIRCUIT_BREAKER_EVENTS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(ORCHESTRATOR_RESTARTS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(SECRET_OPERATIONS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(EVENTS_EMITTED_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(ENV_PROMOTIONS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(AFFINITY_RECOMMENDATIONS_TOTAL.clone()));

    tracing::debug!("Metrics initialized");
}

/// Get metrics in Prometheus text format
pub fn gather() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder
        .encode(&metric_families, &mut buffer)
        .expect("metrics encoded");
    String::from_utf8(buffer).expect("metrics are valid UTF-8")
}

/// Record a workload build
pub fn record_build(runtime: &str, success: bool) {
    let status = if success { "success" } else { "failure" };
    WORKLOAD_BUILDS_TOTAL
        .with_label_values(&[runtime, status])
        .inc();
}

/// Record a workload deployment
pub fn record_deployment(runtime: &str, success: bool) {
    let status = if success { "success" } else { "failure" };
    WORKLOAD_DEPLOYMENTS_TOTAL
        .with_label_values(&[runtime, status])
        .inc();

    if success {
        WORKLOAD_RUNNING.with_label_values(&[runtime]).inc();
    }
}

/// Record workload deletion
pub fn record_deletion(runtime: &str) {
    WORKLOAD_RUNNING.with_label_values(&[runtime]).dec();
}

/// Update workload state metrics from state store
pub fn update_workload_states(states: &[(String, String)]) {
    // Reset all states first
    WORKLOAD_STATE.reset();

    // Count states per runtime
    for (runtime, state) in states {
        WORKLOAD_STATE
            .with_label_values(&[runtime, state])
            .inc();
    }
}

/// Record a migration
pub fn record_migration(
    source_runtime: &str,
    target_runtime: &str,
    strategy: &str,
    duration_secs: f64,
    success: bool,
    rollback_performed: bool,
) {
    let status = if success { "success" } else { "failure" };

    MIGRATIONS_TOTAL
        .with_label_values(&[source_runtime, target_runtime, strategy, status])
        .inc();

    MIGRATION_DURATION_SECONDS
        .with_label_values(&[source_runtime, target_runtime, strategy])
        .observe(duration_secs);

    if rollback_performed {
        MIGRATION_ROLLBACKS_TOTAL
            .with_label_values(&[source_runtime, target_runtime, strategy])
            .inc();
    }
}

/// Record runtime availability check
pub fn record_runtime_availability(runtime: &str, available: bool) {
    let value = if available { 1.0 } else { 0.0 };
    RUNTIME_AVAILABLE
        .with_label_values(&[runtime])
        .set(value);
}

/// Record runtime decision time
pub fn record_runtime_decision(duration_secs: f64) {
    RUNTIME_DECISION_SECONDS.observe(duration_secs);
}

/// Record CLI command execution
pub fn record_command(command: &str, duration_secs: f64) {
    CLI_COMMANDS_TOTAL.with_label_values(&[command]).inc();
    COMMAND_DURATION_SECONDS
        .with_label_values(&[command])
        .observe(duration_secs);
}

/// Record a scheduler placement decision
pub fn record_scheduler_placement(runtime: &str, strategy: &str, success: bool) {
    let status = if success { "success" } else { "failure" };
    SCHEDULER_PLACEMENTS_TOTAL
        .with_label_values(&[runtime, strategy, status])
        .inc();
}

/// Record a health check result
pub fn record_health_check(workload: &str, status: &str) {
    HEALTH_CHECKS_TOTAL
        .with_label_values(&[workload, status])
        .inc();
}

/// Record a circuit breaker state change
pub fn record_circuit_breaker_event(workload: &str, state: &str) {
    CIRCUIT_BREAKER_EVENTS_TOTAL
        .with_label_values(&[workload, state])
        .inc();
}

/// Record an orchestrator-triggered restart
pub fn record_orchestrator_restart(workload: &str, runtime: &str) {
    ORCHESTRATOR_RESTARTS_TOTAL
        .with_label_values(&[workload, runtime])
        .inc();
}

/// Record a secret management operation
pub fn record_secret_operation(operation: &str) {
    SECRET_OPERATIONS_TOTAL
        .with_label_values(&[operation])
        .inc();
}

/// Record an event emission
pub fn record_event_emitted(category: &str, severity: &str) {
    EVENTS_EMITTED_TOTAL
        .with_label_values(&[category, severity])
        .inc();
}

/// Record an environment promotion
pub fn record_env_promotion(from_tier: &str, to_tier: &str, success: bool) {
    let status = if success { "success" } else { "failure" };
    ENV_PROMOTIONS_TOTAL
        .with_label_values(&[from_tier, to_tier, status])
        .inc();
}

/// Record an affinity recommendation
pub fn record_affinity_recommendation(workload_class: &str, recommended_runtime: &str) {
    AFFINITY_RECOMMENDATIONS_TOTAL
        .with_label_values(&[workload_class, recommended_runtime])
        .inc();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        init();
        let output = gather();
        assert!(output.contains("orchestr8_build_info"));
        assert!(output.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_record_build() {
        init();
        record_build("podman", true);
        record_build("kubernetes", false);
        let output = gather();
        assert!(output.contains("orchestr8_workload_builds_total"));
    }

    #[test]
    fn test_record_deployment() {
        init();
        record_deployment("podman", true);
        let output = gather();
        assert!(output.contains("orchestr8_workload_deployments_total"));
        assert!(output.contains("orchestr8_workload_running"));
    }

    #[test]
    fn test_record_migration() {
        init();
        record_migration("podman", "kubernetes", "blue-green", 45.2, true, false);
        let output = gather();
        assert!(output.contains("orchestr8_migrations_total"));
        assert!(output.contains("orchestr8_migration_duration_seconds"));
    }

    #[test]
    fn test_record_scheduler_placement() {
        init();
        record_scheduler_placement("kubernetes", "balanced", true);
        record_scheduler_placement("podman", "cost-optimized", false);
        let output = gather();
        assert!(output.contains("orchestr8_scheduler_placements_total"));
    }

    #[test]
    fn test_record_health_check() {
        init();
        record_health_check("web-app", "healthy");
        record_health_check("worker", "unhealthy");
        let output = gather();
        assert!(output.contains("orchestr8_health_checks_total"));
    }

    #[test]
    fn test_record_circuit_breaker() {
        init();
        record_circuit_breaker_event("web-app", "open");
        record_circuit_breaker_event("web-app", "closed");
        let output = gather();
        assert!(output.contains("orchestr8_circuit_breaker_events_total"));
    }

    #[test]
    fn test_record_secret_operation() {
        init();
        record_secret_operation("create");
        record_secret_operation("rotate");
        let output = gather();
        assert!(output.contains("orchestr8_secret_operations_total"));
    }

    #[test]
    fn test_record_event_emitted() {
        init();
        record_event_emitted("deployment", "info");
        record_event_emitted("sla_violation", "critical");
        let output = gather();
        assert!(output.contains("orchestr8_events_emitted_total"));
    }

    #[test]
    fn test_record_env_promotion() {
        init();
        record_env_promotion("development", "staging", true);
        let output = gather();
        assert!(output.contains("orchestr8_env_promotions_total"));
    }

    #[test]
    fn test_record_affinity_recommendation() {
        init();
        record_affinity_recommendation("web_service", "kubernetes");
        let output = gather();
        assert!(output.contains("orchestr8_affinity_recommendations_total"));
    }
}
