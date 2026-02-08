//! Orchestr8 CLI entrypoint

use anyhow::Result;
use clap::{Parser, Subcommand};
use orchestr8::{
    adapters::{KubeVirtRuntime, KubernetesRuntime, Metal3Runtime, PodmanRuntime},
    engine::Engine,
    runtime::RuntimeKind,
    spec::Workload,
    state::StateStore,
    Runtime,
};
use std::path::{Path, PathBuf};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "orchestr8")]
#[command(about = "Universal runtime control plane - One spec, four runtimes", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Workload specification file
    #[arg(short, long, default_value = "workload.yaml")]
    spec: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        tracing_subscriber::EnvFilter::new("debug")
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize metrics
    orchestr8::metrics::init();

    // Ensure state directory exists
    StateStore::ensure_state_dir()?;

    // Record command start time
    let start = std::time::Instant::now();
    let command_name = match &cli.command {
        Commands::Validate => "validate",
        Commands::Build => "build",
        Commands::Run { .. } => "run",
        Commands::Stop { .. } => "stop",
        Commands::Status { .. } => "status",
        Commands::Logs { .. } => "logs",
        Commands::Delete { .. } => "delete",
        Commands::List => "list",
        Commands::Migrate { .. } => "migrate",
        Commands::Tui => "tui",
        Commands::Completions { .. } => "completions",
        Commands::Metrics => "metrics",
        Commands::Backup { .. } => "backup",
        Commands::Restore { .. } => "restore",
        Commands::ListBackups => "list-backups",
        Commands::Cost { .. } => "cost",
        Commands::Serve { .. } => "serve",
        Commands::Recommend { .. } => "recommend",
        Commands::Profile { .. } => "profile",
        Commands::AnalyzeLogs { .. } => "analyze-logs",
        Commands::MigrationAdvice { .. } => "migration-advice",
        Commands::ScalingAdvice => "scaling-advice",
        Commands::Config { .. } => "config",
    };

    let result = match cli.command {
        Commands::Validate => validate_command(&cli.spec).await,
        Commands::Build => build_command(&cli.spec).await,
        Commands::Run { runtime } => run_command(&cli.spec, runtime).await,
        Commands::Stop { name } => stop_command(&name).await,
        Commands::Status { name } => status_command(&name).await,
        Commands::Logs { name, follow } => logs_command(&name, follow).await,
        Commands::Delete { name } => delete_command(&name).await,
        Commands::List => list_command().await,
        Commands::Migrate {
            name,
            target,
            strategy,
            no_validation,
            no_rollback,
        } => migrate_command(&name, &target, &strategy, no_validation, no_rollback).await,
        Commands::Tui => tui_command().await,
        Commands::Completions { shell } => {
            completions_command(&shell);
            Ok(())
        }
        Commands::Metrics => {
            metrics_command().await;
            Ok(())
        }
        Commands::Backup { name, description } => {
            backup_command(name.clone(), description.clone()).await
        }
        Commands::Restore { backup, merge } => {
            restore_command(&backup, merge).await
        }
        Commands::ListBackups => {
            list_backups_command().await
        }
        Commands::Cost { provider } => {
            cost_command(&cli.spec, &provider).await
        }
        Commands::Serve { host, port } => {
            serve_command(host, port).await
        }
        Commands::Recommend { runtime } => {
            recommend_command(&cli.spec, runtime).await
        }
        Commands::Profile { name } => {
            profile_command(&cli.spec, name).await
        }
        Commands::AnalyzeLogs { name } => {
            analyze_logs_command(&name).await
        }
        Commands::MigrationAdvice { name, target } => {
            migration_advice_command(&name, &target).await
        }
        Commands::ScalingAdvice => {
            scaling_advice_command().await
        }
        Commands::Config { show, init } => {
            config_command(show, init).await
        }
    };

    // Record command execution time
    let duration = start.elapsed().as_secs_f64();
    orchestr8::metrics::record_command(command_name, duration);

    result
}

async fn validate_command(spec_path: &PathBuf) -> Result<()> {
    println!("🔍 Validating workload specification...");

    let workload = Workload::from_file(spec_path)?;
    println!("✅ Workload '{}' is valid", workload.metadata.name);
    println!("\n📋 Workload Details:");
    println!("  Name: {}", workload.metadata.name);
    println!("  Owner: {}", workload.metadata.owner);
    println!("  Project: {}", workload.metadata.project);
    println!("  CPU: {}", workload.requirements.cpu);
    println!("  Memory: {}", workload.requirements.memory);
    println!("  Storage: {}", workload.requirements.storage);
    println!("  Preferred Runtime: {:?}", workload.runtime.preferred);

    Ok(())
}

async fn build_command(spec_path: &PathBuf) -> Result<()> {
    println!("🔨 Building workload...");

    let workload = Workload::from_file(spec_path)?;
    let engine = Engine::new();
    let runtime_kind = engine.decide(&workload)?;

    println!("📦 Selected runtime: {}", runtime_kind);

    // Build based on runtime
    let result = match runtime_kind {
        RuntimeKind::Podman => {
            let runtime = PodmanRuntime::new()?;
            runtime.build(&workload).await
        }
        RuntimeKind::Kubernetes => {
            let runtime = KubernetesRuntime::new().await?;
            runtime.build(&workload).await
        }
        RuntimeKind::KubeVirt => {
            let runtime = KubeVirtRuntime::new().await?;
            runtime.build(&workload).await
        }
        RuntimeKind::Metal3 => {
            let runtime = Metal3Runtime::new().await?;
            runtime.build(&workload).await
        }
    };

    match result {
        Ok(image) => {
            println!("✅ Built image: {}", image.full_name());
            orchestr8::metrics::record_build(&runtime_kind.to_string(), true);
            Ok(())
        }
        Err(e) => {
            orchestr8::metrics::record_build(&runtime_kind.to_string(), false);
            Err(e)
        }
    }
}

async fn run_command(spec_path: &PathBuf, runtime_override: Option<String>) -> Result<()> {
    println!("🚀 Running workload...");

    let workload = Workload::from_file(spec_path)?;
    let engine = Engine::new();

    // Determine runtime
    let runtime_kind = if let Some(override_str) = runtime_override {
        match override_str.as_str() {
            "podman" | "container" => RuntimeKind::Podman,
            "kube" | "kubernetes" => RuntimeKind::Kubernetes,
            "kubevirt" | "vm" => RuntimeKind::KubeVirt,
            "metal" | "metal3" => RuntimeKind::Metal3,
            _ => anyhow::bail!("Unknown runtime: {}", override_str),
        }
    } else {
        engine.decide(&workload)?
    };

    println!("📦 Selected runtime: {}", runtime_kind);

    // Build and run based on runtime
    let (_image, instance) = match runtime_kind {
        RuntimeKind::Podman => {
            let runtime = PodmanRuntime::new()?;
            let image = runtime.build(&workload).await?;
            println!("✅ Built image: {}", image.full_name());
            let instance = runtime.run(&image, &workload).await?;
            (image, instance)
        }
        RuntimeKind::Kubernetes => {
            let runtime = KubernetesRuntime::new().await?;
            let image = runtime.build(&workload).await?;
            println!("✅ Image reference: {}", image.full_name());
            let instance = runtime.run(&image, &workload).await?;
            (image, instance)
        }
        RuntimeKind::KubeVirt => {
            let runtime = KubeVirtRuntime::new().await?;
            let image = runtime.build(&workload).await?;
            println!("✅ VM image reference: {}", image.full_name());
            let instance = runtime.run(&image, &workload).await?;
            (image, instance)
        }
        RuntimeKind::Metal3 => {
            let runtime = Metal3Runtime::new().await?;
            let image = runtime.build(&workload).await?;
            println!("✅ Bare metal image reference: {}", image.full_name());
            let instance = runtime.run(&image, &workload).await?;
            (image, instance)
        }
    };

    println!("✅ Started instance: {} ({})", instance.name, instance.id);

    // Save state
    let mut state = StateStore::load(&StateStore::default_path())?;
    state.upsert(
        workload.metadata.name.clone(),
        orchestr8::state::WorkloadState {
            name: workload.metadata.name.clone(),
            runtime: runtime_kind,
            instance,
            spec_path: spec_path.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    );
    state.save(&StateStore::default_path())?;

    // Record metrics
    orchestr8::metrics::record_deployment(&runtime_kind.to_string(), true);

    Ok(())
}

async fn stop_command(name: &str) -> Result<()> {
    println!("⏸️  Stopping workload '{}'...", name);

    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    // Stop based on runtime
    match workload_state.runtime {
        RuntimeKind::Podman => {
            let runtime = PodmanRuntime::new()?;
            runtime.stop(&workload_state.instance).await?;
        }
        RuntimeKind::Kubernetes => {
            let runtime = KubernetesRuntime::new().await?;
            runtime.stop(&workload_state.instance).await?;
        }
        RuntimeKind::KubeVirt => {
            let runtime = KubeVirtRuntime::new().await?;
            runtime.stop(&workload_state.instance).await?;
        }
        RuntimeKind::Metal3 => {
            let runtime = Metal3Runtime::new().await?;
            runtime.stop(&workload_state.instance).await?;
        }
    }

    println!("✅ Stopped instance: {}", name);

    Ok(())
}

async fn status_command(name: &str) -> Result<()> {
    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    // Get status based on runtime
    let status = match workload_state.runtime {
        RuntimeKind::Podman => {
            let runtime = PodmanRuntime::new()?;
            runtime.status(&workload_state.instance).await?
        }
        RuntimeKind::Kubernetes => {
            let runtime = KubernetesRuntime::new().await?;
            runtime.status(&workload_state.instance).await?
        }
        RuntimeKind::KubeVirt => {
            let runtime = KubeVirtRuntime::new().await?;
            runtime.status(&workload_state.instance).await?
        }
        RuntimeKind::Metal3 => {
            let runtime = Metal3Runtime::new().await?;
            runtime.status(&workload_state.instance).await?
        }
    };

    println!("📊 Status for '{}':", name);
    println!("  Runtime: {}", workload_state.runtime);
    println!("  State: {}", status.state);
    println!("  Ready: {}", status.ready);
    if let Some(msg) = status.message {
        println!("  Message: {}", msg);
    }
    if status.restart_count > 0 {
        println!("  Restarts: {}", status.restart_count);
    }

    Ok(())
}

async fn logs_command(name: &str, follow: bool) -> Result<()> {
    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    // Get logs based on runtime
    let logs = match workload_state.runtime {
        RuntimeKind::Podman => {
            let runtime = PodmanRuntime::new()?;
            runtime.logs(&workload_state.instance, follow).await?
        }
        RuntimeKind::Kubernetes => {
            let runtime = KubernetesRuntime::new().await?;
            runtime.logs(&workload_state.instance, follow).await?
        }
        RuntimeKind::KubeVirt => {
            let runtime = KubeVirtRuntime::new().await?;
            runtime.logs(&workload_state.instance, follow).await?
        }
        RuntimeKind::Metal3 => {
            let runtime = Metal3Runtime::new().await?;
            runtime.logs(&workload_state.instance, follow).await?
        }
    };

    println!("{}", logs);

    Ok(())
}

async fn delete_command(name: &str) -> Result<()> {
    println!("🗑️  Deleting workload '{}'...", name);

    let mut state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?
        .clone();

    // Delete based on runtime
    match workload_state.runtime {
        RuntimeKind::Podman => {
            let runtime = PodmanRuntime::new()?;
            runtime.delete(&workload_state.instance).await?;
        }
        RuntimeKind::Kubernetes => {
            let runtime = KubernetesRuntime::new().await?;
            runtime.delete(&workload_state.instance).await?;
        }
        RuntimeKind::KubeVirt => {
            let runtime = KubeVirtRuntime::new().await?;
            runtime.delete(&workload_state.instance).await?;
        }
        RuntimeKind::Metal3 => {
            let runtime = Metal3Runtime::new().await?;
            runtime.delete(&workload_state.instance).await?;
        }
    }

    // Remove from state
    state.remove(name);
    state.save(&StateStore::default_path())?;

    // Record metrics
    orchestr8::metrics::record_deletion(&workload_state.runtime.to_string());

    println!("✅ Deleted instance: {}", name);

    Ok(())
}

async fn list_command() -> Result<()> {
    let state = StateStore::load(&StateStore::default_path())?;
    let workloads = state.list();

    if workloads.is_empty() {
        println!("No workloads running");
        return Ok(());
    }

    println!("📋 Running workloads:");
    for w in workloads {
        println!(
            "  {} ({}) - {}",
            w.name,
            w.runtime,
            w.instance.id.chars().take(12).collect::<String>()
        );
    }

    Ok(())
}

async fn migrate_command(
    name: &str,
    target: &str,
    strategy_str: &str,
    no_validation: bool,
    no_rollback: bool,
) -> Result<()> {
    use orchestr8::migration::{MigrationEngine, MigrationPlan, MigrationStrategy};
    use std::time::Duration;

    println!("🔄 Migrating workload '{}'...", name);

    // Load current state
    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    let source_runtime = workload_state.runtime;

    // Parse target runtime
    let target_runtime = match target {
        "podman" | "container" => RuntimeKind::Podman,
        "kube" | "kubernetes" => RuntimeKind::Kubernetes,
        "kubevirt" | "vm" => RuntimeKind::KubeVirt,
        "metal" | "metal3" => RuntimeKind::Metal3,
        _ => anyhow::bail!("Unknown runtime: {}", target),
    };

    // Parse strategy
    let strategy = match strategy_str {
        "immediate" => MigrationStrategy::Immediate,
        "blue-green" => MigrationStrategy::BlueGreen,
        "rolling" => MigrationStrategy::Rolling,
        _ => anyhow::bail!("Unknown strategy: {}", strategy_str),
    };

    println!("📊 Migration Plan:");
    println!("  Workload: {}", name);
    println!("  Source: {}", source_runtime);
    println!("  Target: {}", target_runtime);
    println!("  Strategy: {:?}", strategy);

    // Create migration plan
    let plan = MigrationPlan {
        workload_name: name.to_string(),
        source_runtime,
        target_runtime,
        strategy,
        validation_delay: if no_validation {
            Duration::from_secs(5)
        } else {
            Duration::from_secs(30)
        },
        rollback_on_failure: !no_rollback,
    };

    // Execute migration
    let migration_start = std::time::Instant::now();
    let engine = MigrationEngine::new(StateStore::default_path());
    let result = engine.migrate(plan).await?;
    let migration_duration = migration_start.elapsed().as_secs_f64();

    // Record metrics
    orchestr8::metrics::record_migration(
        &source_runtime.to_string(),
        &target_runtime.to_string(),
        strategy_str,
        migration_duration,
        result.success,
        result.rollback_performed,
    );

    if result.success {
        println!("✅ Migration completed successfully!");
        if let Some(instance) = result.target_instance {
            println!("  New instance: {} ({})", instance.name, instance.id);
            println!("  Runtime: {}", target_runtime);
        }
    } else {
        println!("❌ Migration failed!");
        if let Some(error) = result.error {
            println!("  Error: {}", error);
        }
        if result.rollback_performed {
            println!("  Rollback: Performed successfully");
            if let Some(instance) = result.source_instance {
                println!("  Restored instance: {} ({})", instance.name, instance.id);
            }
        }
        anyhow::bail!("Migration failed");
    }

    Ok(())
}

async fn tui_command() -> Result<()> {
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use orchestr8::ui::App;
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Initial load
    app.refresh_workloads().await?;

    // Main loop
    let res = run_tui(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run_tui<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut orchestr8::ui::App,
) -> Result<()> {
    use orchestr8::ui::{render_dashboard, render_logs, Screen};

    loop {
        // Render
        terminal.draw(|f| match &app.screen {
            Screen::Dashboard => {
                render_dashboard(f, app);
            }
            Screen::Logs(name) => {
                render_logs(f, app, name);
            }
        })?;

        // Handle events
        orchestr8::ui::handle_events(app).await?;

        // Check if should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn completions_command(shell_str: &str) {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let shell = match shell_str.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" | "ps1" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => {
            eprintln!("Unknown shell: {}", shell_str);
            eprintln!();
            orchestr8::completions::list_shells();
            std::process::exit(1);
        }
    };

    let mut cmd = Cli::command();
    orchestr8::completions::generate_completions(shell, &mut cmd);
}

async fn metrics_command() {
    println!("# Orchestr8 Metrics");
    println!("# Updated: {}", chrono::Utc::now().to_rfc3339());
    println!();

    // Update workload state metrics from state store
    if let Ok(state) = StateStore::load(&StateStore::default_path()) {
        let workloads = state.list();
        let states: Vec<(String, String)> = workloads
            .iter()
            .map(|w| (w.runtime.to_string(), "running".to_string()))
            .collect();
        orchestr8::metrics::update_workload_states(&states);

        // Update running workload count
        let mut runtime_counts = std::collections::HashMap::new();
        for w in workloads {
            *runtime_counts.entry(w.runtime.to_string()).or_insert(0) += 1;
        }
    }

    // Gather and print metrics
    print!("{}", orchestr8::metrics::gather());
}

async fn backup_command(name: Option<String>, description: Option<String>) -> Result<()> {
    use orchestr8::backup::BackupManager;

    println!("💾 Creating backup...");

    // Load current state
    let state = StateStore::load(&StateStore::default_path())?;
    let workload_count = state.list().len();

    if workload_count == 0 {
        println!("⚠️  No workloads to backup");
        return Ok(());
    }

    // Create backup
    let manager = BackupManager::new(BackupManager::default_dir());
    let backup_path = manager.create_backup(&state, name.clone(), description.clone())?;

    println!("✅ Backup created: {}", backup_path.display());
    println!("   Workloads: {}", workload_count);
    if let Some(desc) = description {
        println!("   Description: {}", desc);
    }

    Ok(())
}

async fn restore_command(backup_path: &Path, merge: bool) -> Result<()> {
    use orchestr8::backup::Backup;

    println!("📦 Restoring from backup...");

    // Load backup
    let backup = Backup::load(backup_path)?;

    println!("   Backup created: {}", backup.metadata.created_at);
    println!("   Workloads: {}", backup.metadata.workload_count);
    println!("   Version: {}", backup.metadata.orchestr8_version);

    if let Some(desc) = &backup.metadata.description {
        println!("   Description: {}", desc);
    }

    // Restore or merge
    let state_path = StateStore::default_path();

    if merge {
        println!("\n🔀 Merging backup with existing state...");
        backup.merge(&state_path)?;
        println!("✅ Backup merged successfully");
    } else {
        println!("\n⚠️  This will replace your current state!");
        println!("   Press Enter to continue, or Ctrl+C to cancel...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        backup.restore(&state_path)?;
        println!("✅ Backup restored successfully");
    }

    Ok(())
}

async fn list_backups_command() -> Result<()> {
    use orchestr8::backup::BackupManager;

    let manager = BackupManager::new(BackupManager::default_dir());
    let backups = manager.list_backups()?;

    if backups.is_empty() {
        println!("No backups found");
        println!("Backup directory: {}", BackupManager::default_dir().display());
        return Ok(());
    }

    println!("📋 Available backups:\n");

    for backup_path in backups {
        match manager.get_backup_info(&backup_path) {
            Ok(info) => {
                println!("  📄 {}", backup_path.file_name().unwrap().to_string_lossy());
                println!("     Created: {}", info.created_at);
                println!("     Workloads: {}", info.workload_count);
                println!("     Version: {}", info.orchestr8_version);
                if let Some(desc) = info.description {
                    println!("     Description: {}", desc);
                }
                println!();
            }
            Err(e) => {
                println!("  ⚠️  {} (error: {})", backup_path.display(), e);
                println!();
            }
        }
    }

    println!("Backup directory: {}", BackupManager::default_dir().display());

    Ok(())
}

async fn cost_command(spec_path: &PathBuf, provider: &str) -> Result<()> {
    use orchestr8::cost::{estimate_cost, CloudProvider, CostComparison};

    println!("💰 Estimating costs...\n");

    let workload = Workload::from_file(spec_path)?;

    if provider == "all" {
        // Show comparison across all providers
        let comparison = CostComparison::for_workload(&workload)?;
        print!("{}", comparison.display());
    } else {
        // Show estimate for specific provider
        let cloud_provider = match provider.to_lowercase().as_str() {
            "aws" => CloudProvider::AWS,
            "azure" => CloudProvider::Azure,
            "gcp" => CloudProvider::GCP,
            "digitalocean" | "do" => CloudProvider::DigitalOcean,
            "linode" => CloudProvider::Linode,
            _ => {
                eprintln!("Unknown provider: {}", provider);
                eprintln!("Available: aws, azure, gcp, digitalocean, linode");
                std::process::exit(1);
            }
        };

        let estimate = estimate_cost(&workload, cloud_provider)?;

        println!("Workload: {}", workload.metadata.name);
        println!("Resources: {} CPU | {} RAM | {} Storage",
            workload.requirements.cpu,
            workload.requirements.memory,
            workload.requirements.storage);
        println!();
        println!("{}", estimate.display());
    }

    println!("\n💡 Note: Estimates are based on baseline pricing and may vary based on:");
    println!("   - Region selection");
    println!("   - Reserved vs. on-demand instances");
    println!("   - Volume discounts");
    println!("   - Additional services (load balancers, networking, etc.)");

    Ok(())
}

async fn serve_command(host: String, port: u16) -> Result<()> {
    use orchestr8::api::{ApiConfig, start_server};

    println!("🚀 Starting Orchestr8 API Server\n");

    let config = ApiConfig {
        host,
        port,
        state_path: StateStore::default_path(),
    };

    println!("📊 Dashboard URL: http://{}:{}", config.host, config.port);
    println!("🔌 API Endpoints:");
    println!("   GET  /health");
    println!("   GET  /api/workloads");
    println!("   POST /api/workloads");
    println!("   GET  /api/workloads/:name");
    println!("   DELETE /api/workloads/:name");
    println!("   GET  /api/workloads/:name/logs");
    println!("   POST /api/workloads/:name/stop");
    println!("   POST /api/cost");
    println!("   GET  /api/backups");
    println!("   POST /api/backups");
    println!();
    println!("Press Ctrl+C to stop the server\n");

    start_server(config).await
}

async fn recommend_command(spec_path: &PathBuf, _runtime_override: Option<String>) -> Result<()> {
    use orchestr8::ai::scoring::{format_scoring_report, ScoringEngine};
    use orchestr8::config::Config;

    println!("🤖 AI-Powered Runtime Recommendation\n");

    let config = Config::load();
    let workload = Workload::from_file(spec_path)?;
    let engine = ScoringEngine::new(config.engine);
    let result = engine.score(&workload);

    print!("{}", format_scoring_report(&result));

    Ok(())
}

async fn profile_command(spec_path: &PathBuf, name: Option<String>) -> Result<()> {
    use orchestr8::ai::profiler::{format_profile_report, Profiler};
    use orchestr8::config::Config;

    println!("🔍 Workload Profiler\n");

    let config = Config::load();
    let profiler = Profiler::new(config.profiler.waste_threshold);

    // If name is provided, look up runtime from state
    let (workload, runtime) = if let Some(ref workload_name) = name {
        let state = StateStore::load(&StateStore::default_path())?;
        let ws = state
            .get(workload_name)
            .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", workload_name))?;
        let workload = Workload::from_file(&ws.spec_path)?;
        (workload, Some(ws.runtime))
    } else {
        let workload = Workload::from_file(spec_path)?;
        (workload, None)
    };

    let profile = profiler.profile(&workload, runtime);
    print!("{}", format_profile_report(&profile));

    Ok(())
}

async fn analyze_logs_command(name: &str) -> Result<()> {
    use orchestr8::ai::analyzer::{format_analysis_report, LogAnalyzer};
    use orchestr8::config::Config;

    println!("📊 Log Analysis for '{}'\n", name);

    let config = Config::load();
    let analyzer = LogAnalyzer::new(config.analyzer);

    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    // Fetch logs
    let logs = match workload_state.runtime {
        RuntimeKind::Podman => {
            let runtime = PodmanRuntime::new()?;
            runtime.logs(&workload_state.instance, false).await?
        }
        RuntimeKind::Kubernetes => {
            let runtime = KubernetesRuntime::new().await?;
            runtime.logs(&workload_state.instance, false).await?
        }
        RuntimeKind::KubeVirt => {
            let runtime = KubeVirtRuntime::new().await?;
            runtime.logs(&workload_state.instance, false).await?
        }
        RuntimeKind::Metal3 => {
            let runtime = Metal3Runtime::new().await?;
            runtime.logs(&workload_state.instance, false).await?
        }
    };

    let analysis = analyzer.analyze(&logs);
    print!("{}", format_analysis_report(&analysis));

    Ok(())
}

async fn migration_advice_command(name: &str, target: &str) -> Result<()> {
    use orchestr8::ai::migration::{format_migration_advice, MigrationAdvisor};
    use orchestr8::config::Config;

    println!("🔄 Migration Advisor for '{}'\n", name);

    let config = Config::load();
    let advisor = MigrationAdvisor::new(config.migration);

    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    let target_runtime = match target {
        "podman" | "container" => RuntimeKind::Podman,
        "kube" | "kubernetes" => RuntimeKind::Kubernetes,
        "kubevirt" | "vm" => RuntimeKind::KubeVirt,
        "metal" | "metal3" => RuntimeKind::Metal3,
        _ => anyhow::bail!("Unknown runtime: {}", target),
    };

    let workload = Workload::from_file(&workload_state.spec_path)?;
    let advice = advisor.advise(&workload, workload_state.runtime, target_runtime);
    print!("{}", format_migration_advice(&advice));

    Ok(())
}

async fn scaling_advice_command() -> Result<()> {
    use orchestr8::ai::scaling::{format_scaling_report, ScalingEngine, TimeSeries};
    use orchestr8::config::Config;

    println!("📈 Predictive Scaling Advisor\n");

    let config = Config::load();
    let engine = ScalingEngine::new(config.scaling);

    // Generate sample metrics (in production this would come from Prometheus/metrics)
    let mut cpu_series = TimeSeries::new("cpu_utilization", "ratio");
    let mut mem_series = TimeSeries::new("memory_utilization", "ratio");

    let base_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        - 3600.0;

    // Simulate metrics from last hour
    for i in 0..60 {
        let t = base_time + (i as f64 * 60.0);
        cpu_series.add(t, 0.45 + (i as f64 * 0.005) + ((i as f64 * 0.1).sin() * 0.05));
        mem_series.add(t, 0.55 + (i as f64 * 0.002));
    }

    let rec = engine.recommend(&cpu_series, &mem_series, 3, 1, 10, 0.05);
    print!("{}", format_scaling_report(&rec));

    println!("\n💡 Note: Using simulated metrics. Connect to Prometheus for real data.");

    Ok(())
}

async fn config_command(show: bool, init: bool) -> Result<()> {
    use orchestr8::config::Config;

    if init {
        let config = Config::default();
        let path = Config::default_path();
        config.save_to(&path)?;
        println!("✅ Configuration initialized: {}", path.display());
        println!("\nEdit this file to customize Orchestr8 behavior.");
        return Ok(());
    }

    if show {
        let config = Config::load();
        let yaml = serde_yaml::to_string(&config)?;
        println!("📋 Current Configuration ({})\n", Config::default_path().display());
        println!("{}", yaml);
        return Ok(());
    }

    // Default: show summary
    let config = Config::load();
    let path = Config::default_path();
    let exists = path.exists();

    println!("⚙️  Orchestr8 Configuration\n");
    println!("Config file: {}", path.display());
    println!("Status: {}\n", if exists { "loaded" } else { "using defaults" });

    println!("Engine:");
    println!("  Scoring enabled: {}", config.engine.enable_scoring);
    println!("  Metal3 CPU threshold: {:.0} cores", config.engine.metal3_cpu_threshold);
    println!("  Metal3 memory threshold: {:.0}Gi", config.engine.metal3_memory_threshold_gi);
    println!("  Weights: cost={:.0}% perf={:.0}% rel={:.0}% avail={:.0}%",
        config.engine.scoring_weights.cost * 100.0,
        config.engine.scoring_weights.performance * 100.0,
        config.engine.scoring_weights.reliability * 100.0,
        config.engine.scoring_weights.availability * 100.0,
    );

    println!("\nMigration:");
    println!("  Adaptive timing: {}", config.migration.enable_adaptive_timing);
    println!("  Canary error threshold: {:.1}%", config.migration.canary_error_threshold * 100.0);
    println!("  Max retries: {}", config.migration.max_validation_retries);

    println!("\nScaling:");
    println!("  Predictive: {}", config.scaling.enable_predictive);
    println!("  Scale-up threshold: {:.0}%", config.scaling.scale_up_threshold * 100.0);
    println!("  Scale-down threshold: {:.0}%", config.scaling.scale_down_threshold * 100.0);
    println!("  Cost-aware: {}", config.scaling.cost_aware);

    println!("\nCost:");
    println!("  Include GPU: {}", config.cost.include_gpu);
    println!("  Include network: {}", config.cost.include_network);
    println!("  Include load balancer: {}", config.cost.include_load_balancer);

    if !exists {
        println!("\n💡 Run 'orchestr8 config --init' to create a config file");
    }

    Ok(())
}
