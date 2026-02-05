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
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "orchestr8")]
#[command(about = "Universal runtime control plane - One spec, three runtimes", long_about = None)]
#[command(version)]
struct Cli {
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

    // Ensure state directory exists
    StateStore::ensure_state_dir()?;

    match cli.command {
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
    }
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
    let image = match runtime_kind {
        RuntimeKind::Podman => {
            let runtime = PodmanRuntime::new()?;
            runtime.build(&workload).await?
        }
        RuntimeKind::Kubernetes => {
            let runtime = KubernetesRuntime::new().await?;
            runtime.build(&workload).await?
        }
        RuntimeKind::KubeVirt => {
            let runtime = KubeVirtRuntime::new().await?;
            runtime.build(&workload).await?
        }
        RuntimeKind::Metal3 => {
            let runtime = Metal3Runtime::new().await?;
            runtime.build(&workload).await?
        }
    };

    println!("✅ Built image: {}", image.full_name());

    Ok(())
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
    let engine = MigrationEngine::new(StateStore::default_path());
    let result = engine.migrate(plan).await?;

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
