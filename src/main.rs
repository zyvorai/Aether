//! Orchestr8 CLI entrypoint

use anyhow::Result;
use clap::{Parser, Subcommand};
use orchestr8::{
    adapters::{KubernetesRuntime, PodmanRuntime},
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
        Commands::Migrate { name, target } => migrate_command(&name, &target).await,
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
        _ => {
            anyhow::bail!("Build not yet implemented for runtime: {}", runtime_kind);
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
        _ => {
            anyhow::bail!("Runtime not yet implemented: {}", runtime_kind);
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
        _ => {
            anyhow::bail!("Stop not yet implemented for runtime: {}", workload_state.runtime);
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
        _ => {
            anyhow::bail!("Status not yet implemented for runtime: {}", workload_state.runtime);
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
        _ => {
            anyhow::bail!("Logs not yet implemented for runtime: {}", workload_state.runtime);
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
        _ => {
            anyhow::bail!("Delete not yet implemented for runtime: {}", workload_state.runtime);
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

async fn migrate_command(_name: &str, _target: &str) -> Result<()> {
    println!("🔄 Migration not yet implemented");
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
