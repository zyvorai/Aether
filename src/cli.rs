// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! CLI argument definitions for Aether

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aether")]
#[command(
    about = "Universal runtime control plane - One spec, three runtimes",
    long_about = "Aether deploys workloads across three runtimes (Podman, Kubernetes, KubeVirt)\nfrom a single YAML specification with AI-powered runtime selection, zero-downtime\nmigration, and built-in observability."
)]
#[command(version)]
#[command(styles = get_styles())]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,

    /// Workload specification file
    #[arg(short, long, default_value = "workload.yaml", global = true)]
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
    #[arg(long, global = true)]
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
    /// Validate workload specification (checks syntax, intent, and runtime compatibility)
    Validate,

    /// Build workload image (container + optional VM image for KubeVirt)
    Build,

    /// Run / deploy workload instance (auto-selects best runtime via intent engine)
    Run {
        /// Override runtime decision (podman, kube, kubevirt)
        #[arg(short, long)]
        runtime: Option<String>,
    },

    /// Stop running instance
    Stop {
        /// Workload name
        name: String,

        /// Also remove Service, Ingress, and HPA (Kubernetes only; keeps PVCs and Secrets)
        #[arg(long)]
        cascade: bool,
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

    /// Update a running workload with the current spec
    Update {
        /// Workload name
        name: String,

        /// Pin a specific image reference (e.g. registry/app:1.2.3). Overrides --tag.
        #[arg(long)]
        image: Option<String>,

        /// Pin a specific image tag (default: reuse the running image's tag)
        #[arg(long)]
        tag: Option<String>,
    },

    /// List all instances
    List,

    /// Migrate instance to different runtime (zero-downtime with rollback support)
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

        /// Log migration phases to stderr (or set AETHER_MIGRATION_TRACE=1)
        #[arg(long)]
        verbose_trace: bool,
    },

    /// Live-migrate a running KubeVirt VM to another node (requires kubevirt.liveMigration in the spec)
    LiveMigrate {
        /// Workload name
        name: String,

        /// Seconds to watch migration progress before returning (0 = fire-and-forget)
        #[arg(long, default_value = "120")]
        watch_timeout: u64,
    },

    /// Launch interactive TUI dashboard (k9s-style real-time monitoring)
    Tui,

    /// Ask Zyra — interactive AI ops assistant in the terminal
    #[command(visible_alias = "copilot")]
    Ask {
        /// Optional initial question (non-interactive single turn)
        message: Option<String>,
    },

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

    /// GitOps reconciliation
    GitOps {
        #[command(subcommand)]
        action: GitOpsAction,
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
        /// Snapshot version to rollback to (default: latest). Use --list to see available versions.
        #[arg(long)]
        version: Option<usize>,
        /// List available snapshots instead of rolling back
        #[arg(long)]
        list: bool,
    },

    /// Deploy all workloads from a directory
    Deploy {
        /// Directory containing YAML workload specs
        dir: PathBuf,

        /// Override runtime for all workloads (podman, kube, kubevirt)
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

    /// Copy files to/from a running workload pod (kubectl cp wrapper)
    Cp {
        /// Workload name
        name: String,

        /// Source path (local file or `pod:/path/in/container`)
        src: String,

        /// Destination path (local file or `pod:/path/in/container`)
        dest: String,

        /// Timeout in seconds (0 = no timeout)
        #[arg(short, long, default_value = "120")]
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

    /// Evaluate intent-based runtime recommendation
    Intent,

    /// Explain runtime placement decision for a workload spec
    Decide {
        /// Show per-runtime reasons and warnings for all runtimes
        #[arg(long)]
        explain: bool,
    },

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
        #[command(subcommand)]
        action: HealthAction,
    },

    /// Export workload as a Helm chart
    HelmExport {
        /// Output directory for the Helm chart
        #[arg(short, long, default_value = "helm-output")]
        output_dir: PathBuf,
        /// Chart version
        #[arg(long)]
        chart_version: Option<String>,
    },

    /// CycloneDX SBOM export and verify
    Sbom {
        #[command(subcommand)]
        action: SbomAction,
    },

    /// Edge site agent — register with control plane and drain reconcile queue
    EdgeAgent {
        /// Control plane API base URL
        #[arg(long)]
        control_plane: String,
        /// Edge site name
        #[arg(long)]
        site: String,
        /// Shared edge token (or AETHER_EDGE_TOKEN env)
        #[arg(long, env = "AETHER_EDGE_TOKEN")]
        token: Option<String>,
        /// Local kube context for apply
        #[arg(long)]
        kube_context: Option<String>,
        /// Print actions without applying
        #[arg(long)]
        dry_run: bool,
        /// Heartbeat interval seconds
        #[arg(long, default_value = "30")]
        interval_secs: u64,
    },

    /// Scale a deployed workload to a target replica count (Kubernetes)
    Scale {
        /// Workload name
        name: String,

        /// Target replica count (zero or greater)
        replicas: i32,
    },

    /// Restart a deployed workload (rolling restart on Kubernetes, stop+start otherwise)
    Restart {
        /// Workload name
        name: String,
    },

    /// Node maintenance operations (Kubernetes)
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },

    /// Atlas-backed storage: list volumes and manage snapshots
    Storage {
        #[command(subcommand)]
        action: StorageAction,
    },

    /// Manage saved cluster connections for discovery & assessment
    Connection {
        #[command(subcommand)]
        action: ConnectionAction,
    },

    /// Discover applications from a connected cluster into an inventory
    Discover {
        #[command(subcommand)]
        action: DiscoverAction,
    },

    /// Browse the discovered application inventory
    Inventory {
        #[command(subcommand)]
        action: InventoryAction,
    },

    /// Show discovered dependencies for an application
    Dependency {
        #[command(subcommand)]
        action: DependencyAction,
    },

    /// Assess an application's portability / migration readiness
    Assess {
        /// Connection name (source)
        #[arg(long)]
        connection: String,
        /// Application name or id
        app: String,
        /// Target connection to run a compatibility preflight against
        #[arg(long)]
        target: Option<String>,
    },

    /// Generate migration plans (aether.zyvor.dev/v1alpha1)
    Plan {
        #[command(subcommand)]
        action: PlanAction,
    },

    /// Produce a Cloud Exit Assessment report for a connection
    Report {
        /// Connection name
        connection: String,
        /// Output format: md, json, or text
        #[arg(long, default_value = "md")]
        format: String,
        /// Write to a file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Move an application from a source cluster to a target (stateless, blue-green)
    Move {
        #[command(subcommand)]
        action: MoveAction,
    },

    /// Forge GPU/AI infrastructure: capacity, placement, and cost
    Forge {
        #[command(subcommand)]
        action: ForgeAction,
    },
}

#[derive(Subcommand)]
pub(crate) enum ForgeAction {
    /// Cluster GPU stats (total/available/allocated, utilization)
    Stats,
    /// List GPU nodes
    Nodes,
    /// Recommend GPU placement for a workload
    Recommend {
        /// GPU type (e.g. a100, h100, any)
        #[arg(long, default_value = "any")]
        gpu_type: String,
        /// Number of GPUs required
        #[arg(long, default_value = "1")]
        gpus: u32,
        /// Model name to place (optional)
        #[arg(long)]
        model: Option<String>,
    },
    /// Show GPU cost breakdown
    Cost,
}

#[derive(Subcommand)]
pub(crate) enum MoveAction {
    /// Dry-run: show transformed manifests, image-mirror plan, and transform notes
    Plan {
        /// Application name or id
        app: String,
        /// Source connection (has the discovery snapshot)
        #[arg(long)]
        source: String,
        /// Target connection
        #[arg(long)]
        target: String,
        /// Target namespace (defaults to the app's namespace)
        #[arg(long)]
        namespace: Option<String>,
        /// Target image registry (mirror images here)
        #[arg(long)]
        registry: Option<String>,
    },
    /// Mirror images + shadow-deploy the app to the target (no external traffic)
    Start {
        app: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        registry: Option<String>,
    },
    /// Mark the Move cut over (perform the DNS/ingress change yourself)
    Cutover {
        /// Application name
        app: String,
    },
    /// Roll back a Move: delete the applied resources from the target
    Rollback {
        /// Application name
        app: String,
    },
    /// Show the status of a Move
    Status {
        /// Application name
        app: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum StorageAction {
    /// List Atlas volumes owned by Aether
    List,
    /// Show a summary of Atlas-backed storage
    Status,
    /// Snapshot a deployed workload's Atlas volume
    Snapshot {
        /// Workload name
        name: String,
        /// Snapshot name
        snapshot_name: String,
    },
    /// Clone a snapshot into a new independent volume
    Clone {
        /// Snapshot id
        snapshot_id: String,
        /// New volume name
        new_name: String,
    },
    /// Restore a snapshot into a point-in-time volume
    Restore {
        /// Snapshot id
        snapshot_id: String,
        /// New volume name
        new_name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum PlanAction {
    /// Generate a MigrationPlan YAML for an application from its assessment
    Create {
        /// Application name or id
        app: String,
        /// Source connection
        #[arg(long)]
        source: String,
        /// Target connection
        #[arg(long)]
        target: String,
        /// Override the recommended strategy (immediate|blue-green|rolling|canary)
        #[arg(long)]
        strategy: Option<String>,
        /// Write to a file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand)]
pub(crate) enum DiscoverAction {
    /// Run discovery for a connection and persist the inventory snapshot
    Start {
        /// Connection name
        #[arg(long)]
        connection: String,
        /// Limit to these namespaces (comma-separated; default: all)
        #[arg(long, value_delimiter = ',')]
        namespaces: Vec<String>,
    },
    /// Show the last discovery snapshot summary for a connection
    Status {
        /// Connection name
        connection: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum InventoryAction {
    /// List discovered applications
    Applications {
        /// Connection name
        #[arg(long)]
        connection: String,
    },
    /// Show one application in detail
    Show {
        /// Connection name
        #[arg(long)]
        connection: String,
        /// Application name or id
        app: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum DependencyAction {
    /// Show the dependency graph for an application
    Graph {
        /// Connection name
        #[arg(long)]
        connection: String,
        /// Application name or id
        app: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ConnectionAction {
    /// Add or update a saved connection
    Add {
        /// Connection name
        name: String,
        /// Source platform: eks, aks, gke, openshift, rancher, tanzu, kubernetes, k3s, rke2, podman, compose
        #[arg(long, default_value = "kubernetes")]
        kind: String,
        /// Kubeconfig context to use (defaults to current-context)
        #[arg(long, default_value = "")]
        context: String,
        /// Explicit kubeconfig file (defaults to ambient KUBECONFIG)
        #[arg(long)]
        kubeconfig: Option<std::path::PathBuf>,
    },
    /// List saved connections
    List,
    /// Remove a saved connection
    Remove {
        /// Connection name
        name: String,
    },
    /// Test reachability of a connection
    Test {
        /// Connection name
        name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum SbomAction {
    /// Export CycloneDX SBOM JSON
    Export {
        /// Output file (stdout when omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Verify SBOM file format
    Verify {
        /// Path to cyclonedx.json
        file: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum NodeAction {
    /// Mark a node unschedulable
    Cordon {
        /// Node name
        node: String,
    },
    /// Mark a node schedulable again
    Uncordon {
        /// Node name
        node: String,
    },
    /// Safely evict workloads from a node (kubectl drain)
    Drain {
        /// Node name
        node: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum HealthAction {
    /// Show health history for a workload
    Show {
        /// Workload name
        name: String,

        /// Show last N health records
        #[arg(short, long, default_value = "20")]
        last: usize,

        /// Show summary only
        #[arg(long)]
        summary: bool,
    },
    /// Collect health data for all deployed workloads
    Collect,
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
            Self::Update { .. } => "update",
            Self::List => "list",
            Self::Migrate { .. } => "migrate",
            Self::LiveMigrate { .. } => "live-migrate",
            Self::Tui => "tui",
            Self::Ask { .. } => "ask",
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
            Self::GitOps { .. } => "gitops",
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
            Self::Cp { .. } => "cp",
            Self::Watch { .. } => "watch",
            Self::Compare => "compare",
            Self::Decide { .. } => "decide",
            Self::Intent => "intent",
            Self::Init => "init",
            Self::Compose { .. } => "compose",
            Self::Plugin { .. } => "plugin",
            Self::Health { .. } => "health",
            Self::HelmExport { .. } => "helm-export",
            Self::Sbom { .. } => "sbom",
            Self::EdgeAgent { .. } => "edge-agent",
            Self::Scale { .. } => "scale",
            Self::Restart { .. } => "restart",
            Self::Node { .. } => "node",
            Self::Storage { .. } => "storage",
            Self::Connection { .. } => "connection",
            Self::Discover { .. } => "discover",
            Self::Inventory { .. } => "inventory",
            Self::Dependency { .. } => "dependency",
            Self::Assess { .. } => "assess",
            Self::Plan { .. } => "plan",
            Self::Report { .. } => "report",
            Self::Move { .. } => "move",
            Self::Forge { .. } => "forge",
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

#[derive(Debug, Subcommand)]
pub(crate) enum GitOpsAction {
    /// Initialize GitOps with a repository
    Init {
        /// Git repository URL
        #[arg(long)]
        repo: String,
        /// Branch to watch
        #[arg(long, default_value = "main")]
        branch: String,
    },
    /// Show GitOps sync status
    Status,
    /// Trigger a manual sync
    Sync,
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
