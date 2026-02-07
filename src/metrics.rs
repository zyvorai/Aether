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
}
