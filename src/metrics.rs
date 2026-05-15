//! Prometheus metrics for Aether
//!
//! Tracks workload deployments, runtime distribution, and migration operations.

use std::sync::LazyLock;
use prometheus::{
    Counter, CounterVec, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry,
};

/// Global metrics registry
pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

// Workload metrics
/// Total number of workload build operations
pub static WORKLOAD_BUILDS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new("aether_workload_builds_total", "Total number of workload builds")
            .namespace("aether"),
        &["runtime", "status"],
    )
    .expect("metric can be created")
});

/// Total number of workload deployments
pub static WORKLOAD_DEPLOYMENTS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "aether_workload_deployments_total",
            "Total number of workload deployments",
        )
        .namespace("aether"),
        &["runtime", "status"],
    )
    .expect("metric can be created")
});

/// Currently running workloads
pub static WORKLOAD_RUNNING: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("aether_workload_running", "Number of currently running workloads")
            .namespace("aether"),
        &["runtime"],
    )
    .expect("metric can be created")
});

/// Workload state distribution
pub static WORKLOAD_STATE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("aether_workload_state", "Workload state distribution")
            .namespace("aether"),
        &["runtime", "state"],
    )
    .expect("metric can be created")
});

// Migration metrics
/// Total number of migrations
pub static MIGRATIONS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new("aether_migrations_total", "Total number of migrations")
            .namespace("aether"),
        &["source_runtime", "target_runtime", "strategy", "status"],
    )
    .expect("metric can be created")
});

/// Migration duration histogram
pub static MIGRATION_DURATION_SECONDS: LazyLock<HistogramVec> = LazyLock::new(|| {
    HistogramVec::new(
        HistogramOpts::new(
            "aether_migration_duration_seconds",
            "Migration duration in seconds",
        )
        .namespace("aether")
        .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0]),
        &["source_runtime", "target_runtime", "strategy"],
    )
    .expect("metric can be created")
});

/// Migration rollbacks
pub static MIGRATION_ROLLBACKS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "aether_migration_rollbacks_total",
            "Total number of migration rollbacks",
        )
        .namespace("aether"),
        &["source_runtime", "target_runtime", "strategy"],
    )
    .expect("metric can be created")
});

// Runtime metrics
/// Runtime availability
pub static RUNTIME_AVAILABLE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "aether_runtime_available",
            "Runtime availability (1=available, 0=unavailable)",
        )
        .namespace("aether"),
        &["runtime"],
    )
    .expect("metric can be created")
});

/// Runtime decision time
pub static RUNTIME_DECISION_SECONDS: LazyLock<Histogram> = LazyLock::new(|| {
    Histogram::with_opts(
        HistogramOpts::new(
            "aether_runtime_decision_seconds",
            "Runtime decision time in seconds",
        )
        .namespace("aether")
        .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
    )
    .expect("metric can be created")
});

// System metrics
/// Aether version info
pub static AETHER_INFO: LazyLock<Counter> = LazyLock::new(|| {
    Counter::with_opts(
        Opts::new("aether_build_info", "Aether version and build information")
            .namespace("aether")
            .const_label("version", env!("CARGO_PKG_VERSION")),
    )
    .expect("metric can be created")
});

/// Total CLI commands executed
pub static CLI_COMMANDS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new("aether_cli_commands_total", "Total CLI commands executed")
            .namespace("aether"),
        &["command"],
    )
    .expect("metric can be created")
});

/// Command execution duration
pub static COMMAND_DURATION_SECONDS: LazyLock<HistogramVec> = LazyLock::new(|| {
    HistogramVec::new(
        HistogramOpts::new(
            "aether_command_duration_seconds",
            "Command execution duration in seconds",
        )
        .namespace("aether")
        .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]),
        &["command"],
    )
    .expect("metric can be created")
});

// Scheduler metrics
/// Total scheduler placement decisions
pub static SCHEDULER_PLACEMENTS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "aether_scheduler_placements_total",
            "Total scheduler placement decisions",
        )
        .namespace("aether"),
        &["runtime", "strategy", "status"],
    )
    .expect("metric can be created")
});

// Orchestrator / health metrics
/// Health check results
pub static HEALTH_CHECKS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new("aether_health_checks_total", "Total health check results")
            .namespace("aether"),
        &["workload", "status"],
    )
    .expect("metric can be created")
});

/// Circuit breaker state changes
pub static CIRCUIT_BREAKER_EVENTS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "aether_circuit_breaker_events_total",
            "Circuit breaker state change events",
        )
        .namespace("aether"),
        &["workload", "state"],
    )
    .expect("metric can be created")
});

/// Auto-restarts triggered by orchestrator
pub static ORCHESTRATOR_RESTARTS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "aether_orchestrator_restarts_total",
            "Auto-restarts triggered by orchestrator",
        )
        .namespace("aether"),
        &["workload", "runtime"],
    )
    .expect("metric can be created")
});

// Secrets metrics
/// Secret operations
pub static SECRET_OPERATIONS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new("aether_secret_operations_total", "Secret management operations")
            .namespace("aether"),
        &["operation"],
    )
    .expect("metric can be created")
});

// Events metrics
/// Events emitted by category
pub static EVENTS_EMITTED_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new("aether_events_emitted_total", "Events emitted by category")
            .namespace("aether"),
        &["category", "severity"],
    )
    .expect("metric can be created")
});

// Environment metrics
/// Environment promotions
pub static ENV_PROMOTIONS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new("aether_env_promotions_total", "Environment promotion operations")
            .namespace("aether"),
        &["from_tier", "to_tier", "status"],
    )
    .expect("metric can be created")
});

// Affinity metrics
/// Affinity recommendations served
pub static AFFINITY_RECOMMENDATIONS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "aether_affinity_recommendations_total",
            "Affinity recommendations served",
        )
        .namespace("aether"),
        &["workload_class", "recommended_runtime"],
    )
    .expect("metric can be created")
});

/// HTTP requests handled by the embedded Axum API (low-cardinality labels).
pub static API_HTTP_REQUESTS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "aether_api_http_requests_total",
            "Total HTTP requests processed through the API middleware stack",
        )
        .namespace("aether"),
        &["method", "status"],
    )
    .expect("metric can be created")
});

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
    let _ = REGISTRY.register(Box::new(AETHER_INFO.clone()));
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
    let _ = REGISTRY.register(Box::new(API_HTTP_REQUESTS_TOTAL.clone()));

    tracing::debug!("Metrics initialized");
}

/// Get metrics in Prometheus text format
pub fn gather() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return format!("# Error encoding metrics: {}\n", e);
    }
    String::from_utf8(buffer).unwrap_or_else(|e| {
        tracing::error!("Metrics contain invalid UTF-8: {}", e);
        "# Error: metrics contained invalid UTF-8\n".to_string()
    })
}

/// Count one HTTP request after the response status is known.
pub fn record_api_http(method: &str, status: &str) {
    API_HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, status])
        .inc();
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
        assert!(output.contains("aether_build_info"));
        assert!(output.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_record_build() {
        init();
        record_build("podman", true);
        record_build("kubernetes", false);
        let output = gather();
        assert!(output.contains("aether_workload_builds_total"));
    }

    #[test]
    fn test_record_deployment() {
        init();
        record_deployment("podman", true);
        let output = gather();
        assert!(output.contains("aether_workload_deployments_total"));
        assert!(output.contains("aether_workload_running"));
    }

    #[test]
    fn test_record_migration() {
        init();
        record_migration("podman", "kubernetes", "blue-green", 45.2, true, false);
        let output = gather();
        assert!(output.contains("aether_migrations_total"));
        assert!(output.contains("aether_migration_duration_seconds"));
    }

    #[test]
    fn test_record_scheduler_placement() {
        init();
        record_scheduler_placement("kubernetes", "balanced", true);
        record_scheduler_placement("podman", "cost-optimized", false);
        let output = gather();
        assert!(output.contains("aether_scheduler_placements_total"));
    }

    #[test]
    fn test_record_health_check() {
        init();
        record_health_check("web-app", "healthy");
        record_health_check("worker", "unhealthy");
        let output = gather();
        assert!(output.contains("aether_health_checks_total"));
    }

    #[test]
    fn test_record_circuit_breaker() {
        init();
        record_circuit_breaker_event("web-app", "open");
        record_circuit_breaker_event("web-app", "closed");
        let output = gather();
        assert!(output.contains("aether_circuit_breaker_events_total"));
    }

    #[test]
    fn test_record_secret_operation() {
        init();
        record_secret_operation("create");
        record_secret_operation("rotate");
        let output = gather();
        assert!(output.contains("aether_secret_operations_total"));
    }

    #[test]
    fn test_record_event_emitted() {
        init();
        record_event_emitted("deployment", "info");
        record_event_emitted("sla_violation", "critical");
        let output = gather();
        assert!(output.contains("aether_events_emitted_total"));
    }

    #[test]
    fn test_record_env_promotion() {
        init();
        record_env_promotion("development", "staging", true);
        let output = gather();
        assert!(output.contains("aether_env_promotions_total"));
    }

    #[test]
    fn test_record_affinity_recommendation() {
        init();
        record_affinity_recommendation("web_service", "kubernetes");
        let output = gather();
        assert!(output.contains("aether_affinity_recommendations_total"));
    }
}
