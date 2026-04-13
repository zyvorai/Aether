//! CLI argument definitions for Aether

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aether")]
#[command(
    about = "Universal runtime control plane - One spec, four runtimes",
    long_about = "Aether deploys workloads across four runtimes (Podman, Kubernetes, KubeVirt, Metal3)\nfrom a single YAML specification with AI-powered runtime selection, zero-downtime\nmigration, and built-in observability.",
)]
#[command(version)]
#[command(styles = get_styles())]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,

    /// Workload specification file
    #[arg(short, long, default_value = "workload.yaml")]
    pub(crate) spec: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    pub(crate) verbose: bool,

    /// Suppress all output except errors (machine-friendly)
    #[arg(short, long)]
    pub(crate) quiet: bool,

    /// Output results as JSON (machine-readable)
    #[arg(long, conflicts_with = "quiet")]
    pub(crate) json: bool,

    /// Output format: table (default), json, yaml, wide
    #[arg(short, long, default_value = "table")]
    pub(crate) output: OutputFormat,

    /// Skip confirmation prompts (for automation/CI)
    #[arg(short, long)]
    pub(crate) yes: bool,

    /// Show what would happen without executing (for mutating commands)
    #[arg(long)]
    pub(crate) dry_run: bool,

    /// Skip policy checks on deploy (use with caution)
    #[arg(long)]
    pub(crate) skip_policy: bool,

    /// Kubernetes namespace (overrides AETHER_NAMESPACE env var)
    #[arg(short = 'n', long, env = "AETHER_NAMESPACE")]
    pub(crate) namespace: Option<String>,
}

/// Output format for CLI results
#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub(crate) enum OutputFormat {
    /// Default table output
    #[default]
    Table,
    /// JSON machine-readable output
    Json,
    /// YAML output
    Yaml,
    /// Wide table with extra columns
    Wide,
}

/// Custom clap styles matching the orange vision-optimized theme
fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .header(
            clap::builder::styling::AnsiColor::BrightYellow.on_default()
                | clap::builder::styling::Effects::BOLD,
        )
        .usage(
            clap::builder::styling::AnsiColor::BrightYellow.on_default()
                | clap::builder::styling::Effects::BOLD,
        )
        .literal(
            clap::builder::styling::AnsiColor::Yellow.on_default()
                | clap::builder::styling::Effects::BOLD,
        )
        .placeholder(clap::builder::styling::AnsiColor::Cyan.on_default())
        .valid(clap::builder::styling::AnsiColor::Green.on_default())
        .invalid(clap::builder::styling::AnsiColor::Red.on_default())
        .error(
            clap::builder::styling::AnsiColor::Red.on_default()
                | clap::builder::styling::Effects::BOLD,
        )
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
        #[arg(short, long, default_value = "5090")]
        port: u16,

        /// TLS certificate file path (enables HTTPS)
        #[arg(long)]
        tls_cert: Option<std::path::PathBuf>,

        /// TLS private key file path (required with --tls-cert)
        #[arg(long)]
        tls_key: Option<std::path::PathBuf>,
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

    /// Show detailed command reference with examples
    #[command(name = "help-all")]
    HelpAll,

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

    /// Deploy all workloads from a directory
    Deploy {
        /// Directory containing YAML workload specs
        dir: PathBuf,

        /// Override runtime for all workloads (podman, kube, kubevirt, metal)
        #[arg(short, long)]
        runtime: Option<String>,

        /// Stop on first failure
        #[arg(long)]
        fail_fast: bool,

        /// Show deployment plan without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Execute a command inside a running workload
    Exec {
        /// Workload name
        name: String,

        /// Command to run (default: /bin/sh)
        #[arg(default_value = "/bin/sh")]
        command: String,

        /// Pass stdin to the container
        #[arg(short, long)]
        interactive: bool,

        /// Timeout in seconds (0 = no timeout)
        #[arg(short, long, default_value = "0")]
        timeout: u64,
    },

    /// Forward local ports to a running workload
    PortForward {
        /// Workload name
        name: String,

        /// Port mapping (local:remote, e.g. 8080:80)
        ports: String,

        /// Timeout in seconds (0 = no timeout, for scripting)
        #[arg(short, long, default_value = "0")]
        timeout: u64,
    },

    /// Watch spec file and auto-redeploy on changes
    Watch {
        /// Override runtime
        #[arg(short, long)]
        runtime: Option<String>,
    },

    /// Compare workload across runtimes (cost, capabilities, limitations)
    Compare,

    /// First-time setup wizard
    Init,

    /// Deploy workloads from a compose file
    Compose {
        #[command(subcommand)]
        action: ComposeAction,
    },

    /// Manage runtime plugins
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },

    /// View workload health history and uptime
    Health {
        /// Workload name
        name: String,

        /// Show last N health records
        #[arg(short, long, default_value = "20")]
        last: usize,

        /// Show summary only
        #[arg(long)]
        summary: bool,
    },
}

impl Commands {
    /// Return the kebab-case command name for metrics and error display.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Validate => "validate",
            Self::Build => "build",
            Self::Run { .. } => "run",
            Self::Stop { .. } => "stop",
            Self::Status { .. } => "status",
            Self::Logs { .. } => "logs",
            Self::Delete { .. } => "delete",
            Self::List => "list",
            Self::Migrate { .. } => "migrate",
            Self::Tui => "tui",
            Self::Completions { .. } => "completions",
            Self::Metrics => "metrics",
            Self::Backup { .. } => "backup",
            Self::Restore { .. } => "restore",
            Self::ListBackups => "list-backups",
            Self::Cost { .. } => "cost",
            Self::Serve { .. } => "serve",
            Self::Recommend { .. } => "recommend",
            Self::Profile { .. } => "profile",
            Self::AnalyzeLogs { .. } => "analyze-logs",
            Self::MigrationAdvice { .. } => "migration-advice",
            Self::ScalingAdvice => "scaling-advice",
            Self::Config { .. } => "config",
            Self::Drift { .. } => "drift",
            Self::PolicyCheck { .. } => "policy-check",
            Self::Deps { .. } => "deps",
            Self::Audit { .. } => "audit",
            Self::Template { .. } => "template",
            Self::Sla { .. } => "sla",
            Self::Secrets { .. } => "secrets",
            Self::Events { .. } => "events",
            Self::Env { .. } => "env",
            Self::Schedule { .. } => "schedule",
            Self::Orchestrate { .. } => "orchestrate",
            Self::Affinity { .. } => "affinity",
            Self::Webhook { .. } => "webhook",
            Self::HelpAll => "help-all",
            Self::Diff { .. } => "diff",
            Self::Rollback { .. } => "rollback",
            Self::Deploy { .. } => "deploy",
            Self::Exec { .. } => "exec",
            Self::PortForward { .. } => "port-forward",
            Self::Watch { .. } => "watch",
            Self::Compare => "compare",
            Self::Init => "init",
            Self::Compose { .. } => "compose",
            Self::Plugin { .. } => "plugin",
            Self::Health { .. } => "health",
        }
    }
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
    /// Show pending webhook deliveries in the retry queue
    Queue,
    /// Force-retry all queued webhooks now
    Flush,
}

#[derive(Subcommand)]
pub(crate) enum ComposeAction {
    /// Validate a compose file
    Validate {
        /// Path to compose file
        #[arg(default_value = "aether-compose.yaml")]
        file: PathBuf,
    },
    /// Deploy all workloads from a compose file
    Up {
        /// Path to compose file
        #[arg(default_value = "aether-compose.yaml")]
        file: PathBuf,

        /// Override runtime for all workloads
        #[arg(short, long)]
        runtime: Option<String>,

        /// Show plan without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Stop all workloads from a compose file
    Down {
        /// Path to compose file
        #[arg(default_value = "aether-compose.yaml")]
        file: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum PluginAction {
    /// List registered plugins
    List,
    /// Discover plugins from ~/.aether/plugins/
    Discover,
    /// Register a plugin from a manifest file
    Register {
        /// Path to plugin manifest JSON
        manifest: PathBuf,
    },
    /// Unregister a plugin
    Remove {
        /// Plugin name
        name: String,
    },
}
