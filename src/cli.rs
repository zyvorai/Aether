//! CLI argument definitions for Orchestr8

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "orchestr8")]
#[command(about = "Universal runtime control plane - One spec, four runtimes", long_about = None)]
#[command(version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,

    /// Workload specification file
    #[arg(short, long, default_value = "workload.yaml")]
    pub(crate) spec: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    pub(crate) verbose: bool,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Validate workload specification
    Validate,

    /// Build workload image
    Build,

    /// Run workload instance
    Run {
        /// Override runtime decision (podman, kube, kubevirt, metal)
        #[arg(short, long)]
        runtime: Option<String>,
    },

    /// Stop running instance
    Stop {
        /// Workload name
        name: String,
    },

    /// Get instance status
    Status {
        /// Workload name
        name: String,
    },

    /// View instance logs
    Logs {
        /// Workload name
        name: String,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },

    /// Delete instance
    Delete {
        /// Workload name
        name: String,
    },

    /// List all instances
    List,

    /// Migrate instance to different runtime
    Migrate {
        /// Workload name
        name: String,

        /// Target runtime
        target: String,

        /// Migration strategy
        #[arg(short, long, default_value = "blue-green")]
        strategy: String,

        /// Skip validation delay
        #[arg(long)]
        no_validation: bool,

        /// Disable rollback on failure
        #[arg(long)]
        no_rollback: bool,
    },

    /// Launch interactive TUI dashboard
    Tui,

    /// Generate shell completions
    Completions {
        /// Shell type (bash, zsh, fish, powershell, elvish)
        shell: String,
    },

    /// Export Prometheus metrics
    Metrics,

    /// Backup workload state
    Backup {
        /// Backup name (optional, auto-generated if not provided)
        #[arg(short, long)]
        name: Option<String>,

        /// Backup description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Restore workload state from backup
    Restore {
        /// Path to backup file
        backup: PathBuf,

        /// Merge with existing state instead of replacing
        #[arg(short, long)]
        merge: bool,
    },

    /// List available backups
    ListBackups,

    /// Estimate workload costs
    Cost {
        /// Cloud provider (aws, azure, gcp, digitalocean, linode, or "all")
        #[arg(short, long, default_value = "all")]
        provider: String,
    },

    /// Start API server and web dashboard
    Serve {
        /// Server host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Server port
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// AI-powered runtime recommendation with scoring
    Recommend {
        /// Override runtime to evaluate (optional)
        #[arg(short, long)]
        runtime: Option<String>,
    },

    /// Profile workload and show optimization recommendations
    Profile {
        /// Workload name (for deployed workloads)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Analyze logs for anomalies and patterns
    AnalyzeLogs {
        /// Workload name
        name: String,
    },

    /// Get AI migration advice
    MigrationAdvice {
        /// Workload name
        name: String,

        /// Target runtime
        target: String,
    },

    /// Show predictive scaling recommendation
    ScalingAdvice,

    /// Show or update configuration
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Initialize default configuration file
        #[arg(long)]
        init: bool,
    },

    /// Detect configuration drift from spec
    Drift {
        /// Workload name
        name: String,

        /// Auto-reconcile detected drift
        #[arg(long)]
        reconcile: bool,
    },

    /// Check workload against policies
    PolicyCheck {
        /// Policy set: production, development, or custom file path
        #[arg(short, long, default_value = "production")]
        policy: String,
    },

    /// Manage workload dependencies
    Deps {
        #[command(subcommand)]
        action: DepsAction,
    },

    /// View or manage audit trail
    Audit {
        /// Show last N events
        #[arg(short, long, default_value = "20")]
        last: usize,

        /// Filter by workload name
        #[arg(short, long)]
        workload: Option<String>,

        /// Show summary only
        #[arg(long)]
        summary: bool,
    },

    /// Generate workload from template
    Template {
        /// Template name (web-app, rest-api, database, cache, worker, cron-job, ml-training, microservice)
        name: String,

        /// Workload name
        #[arg(long)]
        workload_name: Option<String>,

        /// Owner
        #[arg(long, default_value = "team")]
        owner: String,

        /// Project
        #[arg(long, default_value = "default")]
        project: String,

        /// Container registry
        #[arg(long, default_value = "ghcr.io/org")]
        registry: String,

        /// Output file (default: stdout as YAML)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// List available templates
        #[arg(long)]
        list: bool,
    },

    /// SLA compliance monitoring
    Sla {
        #[command(subcommand)]
        action: SlaAction,
    },

    /// Manage secrets
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },

    /// View and manage events
    Events {
        /// Show last N events
        #[arg(short, long, default_value = "20")]
        last: usize,

        /// Filter by severity (info, warning, error, critical)
        #[arg(long)]
        severity: Option<String>,

        /// Show summary only
        #[arg(long)]
        summary: bool,
    },

    /// Manage deployment environments
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },

    /// Workload scheduling and optimization
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },

    /// Health-aware orchestration
    Orchestrate {
        #[command(subcommand)]
        action: OrchestrateAction,
    },

    /// Runtime affinity learning
    Affinity {
        #[command(subcommand)]
        action: AffinityAction,
    },

    /// Manage webhook notification channels
    Webhook {
        #[command(subcommand)]
        action: WebhookAction,
    },

    /// Compare spec vs stored vs live workload state
    Diff {
        /// Workload name
        name: String,
    },

    /// Rollback a workload to its latest snapshot
    Rollback {
        /// Workload name
        name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum DepsAction {
    /// Add a dependency
    Add {
        /// Workload that has the dependency
        workload: String,
        /// Workload it depends on
        dependency: String,
    },
    /// Remove a dependency
    Remove {
        /// Workload
        workload: String,
        /// Dependency to remove
        dependency: String,
    },
    /// Show dependency graph
    Show,
    /// Show impact of stopping a workload
    Impact {
        /// Workload name
        workload: String,
    },
    /// Show startup order
    Order,
}

#[derive(Subcommand)]
pub(crate) enum SlaAction {
    /// Add an SLA target for a workload
    Add {
        /// Workload name
        workload: String,
        /// SLA tier: standard, high-availability, best-effort
        #[arg(long, default_value = "standard")]
        tier: String,
    },
    /// Check SLA compliance
    Check {
        /// Workload name
        workload: String,
        /// Uptime percentage observed
        #[arg(long)]
        uptime: f64,
        /// Average latency in ms
        #[arg(long, default_value = "100")]
        latency: f64,
        /// Error rate percentage
        #[arg(long, default_value = "0.1")]
        error_rate: f64,
        /// Number of restarts
        #[arg(long, default_value = "0")]
        restarts: u32,
    },
    /// List all SLA targets
    List,
}

#[derive(Subcommand)]
pub(crate) enum SecretsAction {
    /// Create a new secret
    Create {
        /// Secret name
        name: String,
        /// Namespace
        #[arg(long, default_value = "default")]
        namespace: String,
    },
    /// Set a key-value pair in a secret
    Set {
        /// Secret name
        secret: String,
        /// Key name
        key: String,
        /// Value
        value: String,
    },
    /// Get a value from a secret
    Get {
        /// Secret name
        secret: String,
        /// Key name
        key: String,
    },
    /// List all secrets
    List,
    /// Check rotation status
    Audit,
}

#[derive(Subcommand)]
pub(crate) enum EnvAction {
    /// Create a new environment
    Create {
        /// Environment name
        name: String,
        /// Tier: development, staging, production
        #[arg(long, default_value = "development")]
        tier: String,
    },
    /// List environments
    List,
    /// Promote a workload between environments
    Promote {
        /// Workload name
        workload: String,
        /// Source environment
        from: String,
        /// Target environment
        to: String,
    },
    /// Check parity between environments
    Parity {
        /// First environment
        env1: String,
        /// Second environment
        env2: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ScheduleAction {
    /// Schedule a workload
    Place {
        /// Workload name
        name: String,
        /// CPU required (cores)
        #[arg(long, default_value = "2")]
        cpu: f64,
        /// Memory required (MB)
        #[arg(long, default_value = "2048")]
        memory: u64,
        /// Scheduling strategy: balanced, cost, performance, bin-packing
        #[arg(long, default_value = "balanced")]
        strategy: String,
        /// Preferred runtime
        #[arg(long)]
        prefer: Option<String>,
    },
    /// Show runtime utilization
    Utilization,
    /// Get optimization suggestions
    Optimize,
    /// Show current placements
    Placements,
}

#[derive(Subcommand)]
pub(crate) enum OrchestrateAction {
    /// Register a workload for health monitoring
    Register {
        /// Workload name
        name: String,
        /// Runtime
        #[arg(long, default_value = "kubernetes")]
        runtime: String,
    },
    /// Show health status of all workloads
    Status,
    /// Show health summary
    Summary,
    /// Simulate a rolling update
    RollingUpdate {
        /// Workload name
        name: String,
        /// Number of replicas
        #[arg(long, default_value = "3")]
        replicas: u32,
    },
    /// Reset circuit breaker for a workload
    ResetCircuit {
        /// Workload name
        name: String,
    },
    /// Run a single round of health checks against live runtimes
    HealthCheck,
    /// Continuously monitor health at a regular interval
    Watch {
        /// Check interval in seconds
        #[arg(long, default_value = "30")]
        interval: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum AffinityAction {
    /// Show runtime recommendations for a workload class
    Recommend {
        /// Workload class: web-service, api-backend, database, cache, batch-job, ml-training, worker, microservice
        class: String,
    },
    /// Show compatibility matrix
    Matrix,
    /// Show learning statistics
    Stats,
}

#[derive(Subcommand)]
pub(crate) enum WebhookAction {
    /// Add a webhook notification channel
    Add {
        /// Channel name
        name: String,
        /// Webhook URL
        url: String,
        /// HTTP method (POST or GET)
        #[arg(long, default_value = "POST")]
        method: String,
        /// Minimum severity to trigger (info, warning, error, critical)
        #[arg(long, default_value = "warning")]
        severity: String,
    },
    /// Remove a webhook notification channel
    Remove {
        /// Channel name
        name: String,
    },
    /// List all notification channels
    List,
    /// Send a test notification to a channel
    Test {
        /// Channel name
        name: String,
    },
}
