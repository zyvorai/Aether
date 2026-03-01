//! Command handler implementations for Orchestr8 CLI

use anyhow::Result;
use orchestr8::{
    engine::Engine,
    runtime::RuntimeKind,
    spec::Workload,
    state::StateStore,
};
use std::path::{Path, PathBuf};

use crate::cli::*;

pub(crate) async fn validate_command(spec_path: &PathBuf) -> Result<()> {
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

pub(crate) async fn build_command(spec_path: &PathBuf) -> Result<()> {
    println!("🔨 Building workload...");

    let workload = Workload::from_file(spec_path)?;
    let engine = Engine::new();
    let runtime_kind = engine.decide(&workload)?;

    println!("📦 Selected runtime: {}", runtime_kind);

    // Build based on runtime
    let rt = orchestr8::runtime::create_runtime(&runtime_kind).await?;
    let result = rt.build(&workload).await;

    match result {
        Ok(image) => {
            println!("✅ Built image: {}", image.full_name());
            orchestr8::metrics::record_build(&runtime_kind.to_string(), true);
            emit_event(
                orchestr8::events::EventSeverity::Info,
                orchestr8::events::EventCategory::Deployment,
                "cli",
                Some(&workload.metadata.name),
                "Build completed",
                &format!("Built image on {}", runtime_kind),
            );
            Ok(())
        }
        Err(e) => {
            orchestr8::metrics::record_build(&runtime_kind.to_string(), false);
            emit_event(
                orchestr8::events::EventSeverity::Error,
                orchestr8::events::EventCategory::Deployment,
                "cli",
                Some(&workload.metadata.name),
                "Build failed",
                &format!("Build failed on {}: {}", runtime_kind, e),
            );
            Err(e)
        }
    }
}

pub(crate) async fn run_command(spec_path: &PathBuf, runtime_override: Option<String>) -> Result<()> {
    println!("🚀 Running workload...");

    let workload = Workload::from_file(spec_path)?;
    let engine = Engine::new();

    // Determine runtime
    let runtime_kind = if let Some(override_str) = runtime_override {
        override_str.parse::<RuntimeKind>()?
    } else {
        engine.decide(&workload)?
    };

    println!("📦 Selected runtime: {}", runtime_kind);

    // Build and run based on runtime
    let rt = orchestr8::runtime::create_runtime(&runtime_kind).await?;
    let image = rt.build(&workload).await?;
    println!("✅ Image ready: {}", image.full_name());
    let instance = rt.run(&image, &workload).await?;

    println!("✅ Started instance: {} ({})", instance.name, instance.id);

    // Capture instance name before move
    let instance_name = instance.name.clone();

    // Save state (auto-snapshot existing workload if present)
    let mut state = StateStore::load(&StateStore::default_path())?;
    if let Some(existing) = state.get(&workload.metadata.name) {
        let snap_mgr = orchestr8::backup::SnapshotManager::new();
        if let Err(e) = snap_mgr.create_snapshot(existing) {
            tracing::warn!("Failed to create pre-deploy snapshot: {}", e);
        }
    }
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

    emit_event(
        orchestr8::events::EventSeverity::Info,
        orchestr8::events::EventCategory::Deployment,
        "cli",
        Some(&workload.metadata.name),
        "Workload deployed",
        &format!("Deployed on {} (instance: {})", runtime_kind, instance_name),
    );

    Ok(())
}

pub(crate) async fn stop_command(name: &str) -> Result<()> {
    println!("⏸️  Stopping workload '{}'...", name);

    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    // Stop based on runtime
    let rt = orchestr8::runtime::create_runtime(&workload_state.runtime).await?;
    rt.stop(&workload_state.instance).await?;

    println!("✅ Stopped instance: {}", name);

    emit_event(
        orchestr8::events::EventSeverity::Info,
        orchestr8::events::EventCategory::Deployment,
        "cli",
        Some(name),
        "Workload stopped",
        &format!("Stopped instance {}", name),
    );

    Ok(())
}

pub(crate) async fn status_command(name: &str) -> Result<()> {
    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    // Get status based on runtime
    let rt = orchestr8::runtime::create_runtime(&workload_state.runtime).await?;
    let status = rt.status(&workload_state.instance).await?;

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

pub(crate) async fn logs_command(name: &str, follow: bool) -> Result<()> {
    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    // Get logs based on runtime
    let rt = orchestr8::runtime::create_runtime(&workload_state.runtime).await?;
    let logs = rt.logs(&workload_state.instance, follow).await?;

    println!("{}", logs);

    Ok(())
}

pub(crate) async fn delete_command(name: &str) -> Result<()> {
    println!("🗑️  Deleting workload '{}'...", name);

    let mut state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?
        .clone();

    // Delete based on runtime
    let rt = orchestr8::runtime::create_runtime(&workload_state.runtime).await?;
    rt.delete(&workload_state.instance).await?;

    // Remove from state
    state.remove(name);
    state.save(&StateStore::default_path())?;

    // Record metrics
    orchestr8::metrics::record_deletion(&workload_state.runtime.to_string());

    emit_event(
        orchestr8::events::EventSeverity::Warning,
        orchestr8::events::EventCategory::Deployment,
        "cli",
        Some(name),
        "Workload deleted",
        &format!("Deleted instance from {}", workload_state.runtime),
    );

    println!("✅ Deleted instance: {}", name);

    Ok(())
}

pub(crate) async fn list_command() -> Result<()> {
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

pub(crate) async fn migrate_command(
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
    let target_runtime: RuntimeKind = target.parse()?;

    // Parse strategy
    let strategy: MigrationStrategy = strategy_str.parse()?;

    // Auto-snapshot before migration
    {
        let snap_mgr = orchestr8::backup::SnapshotManager::new();
        if let Err(e) = snap_mgr.create_snapshot(workload_state) {
            tracing::warn!("Failed to create pre-migrate snapshot: {}", e);
        }
    }

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
        emit_event(
            orchestr8::events::EventSeverity::Info,
            orchestr8::events::EventCategory::Migration,
            "cli",
            Some(name),
            "Migration completed",
            &format!("Migrated from {} to {} using {}", source_runtime, target_runtime, strategy_str),
        );
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
        emit_event(
            orchestr8::events::EventSeverity::Error,
            orchestr8::events::EventCategory::Migration,
            "cli",
            Some(name),
            "Migration failed",
            &format!("Failed migrating from {} to {}", source_runtime, target_runtime),
        );
        anyhow::bail!("Migration failed");
    }

    Ok(())
}

pub(crate) async fn tui_command() -> Result<()> {
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

pub(crate) fn completions_command(shell_str: &str) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::Shell;

    let shell = match shell_str.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" | "ps1" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => {
            orchestr8::completions::list_shells();
            anyhow::bail!("Unknown shell: '{}'. See list above.", shell_str);
        }
    };

    let mut cmd = Cli::command();
    orchestr8::completions::generate_completions(shell, &mut cmd);
    Ok(())
}

pub(crate) async fn metrics_command() {
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

pub(crate) async fn backup_command(name: Option<String>, description: Option<String>) -> Result<()> {
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

pub(crate) async fn restore_command(backup_path: &Path, merge: bool) -> Result<()> {
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

pub(crate) async fn list_backups_command() -> Result<()> {
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
                let fname = backup_path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| backup_path.display().to_string());
                println!("  📄 {}", fname);
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

pub(crate) async fn cost_command(spec_path: &PathBuf, provider: &str) -> Result<()> {
    use orchestr8::cost::{estimate_cost, CloudProvider, CostComparison};

    println!("💰 Estimating costs...\n");

    let workload = Workload::from_file(spec_path)?;

    if provider == "all" {
        // Show comparison across all providers
        let comparison = CostComparison::for_workload(&workload)?;
        print!("{}", comparison.display());
    } else {
        // Show estimate for specific provider
        let cloud_provider: CloudProvider = provider.parse()?;

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

pub(crate) async fn serve_command(host: String, port: u16) -> Result<()> {
    use orchestr8::api::{ApiConfig, start_server};

    println!("🚀 Starting Orchestr8 API Server\n");

    let config = ApiConfig {
        host,
        port,
        state_path: StateStore::default_path(),
    };

    println!("📊 Dashboard URL: http://{}:{}", config.host, config.port);
    println!("🔌 API Endpoints:");
    println!();
    println!("   Workloads:");
    println!("     GET    /health");
    println!("     GET    /api/workloads");
    println!("     POST   /api/workloads");
    println!("     GET    /api/workloads/:name");
    println!("     DELETE /api/workloads/:name");
    println!("     GET    /api/workloads/:name/logs");
    println!("     POST   /api/workloads/:name/start");
    println!("     POST   /api/workloads/:name/stop");
    println!();
    println!("   Cost & Backups:");
    println!("     POST   /api/cost");
    println!("     GET    /api/backups");
    println!("     POST   /api/backups");
    println!();
    println!("   AI Intelligence:");
    println!("     POST   /api/ai/recommend");
    println!("     GET    /api/ai/profile/:name");
    println!("     GET    /api/ai/analyze/:name");
    println!("     GET    /api/affinity/:class");
    println!();
    println!("   Operations:");
    println!("     GET    /api/drift/:name");
    println!("     POST   /api/policy/check");
    println!("     GET    /api/dependencies");
    println!("     POST   /api/dependencies");
    println!("     GET    /api/audit");
    println!("     GET    /api/sla/:workload");
    println!();
    println!("   Templates:");
    println!("     GET    /api/templates");
    println!("     POST   /api/templates/:name");
    println!();
    println!("   Advanced:");
    println!("     GET    /api/secrets");
    println!("     GET    /api/events");
    println!("     GET    /api/events/summary");
    println!("     GET    /api/environments");
    println!("     GET    /api/scheduler/utilization");
    println!("     GET    /api/scheduler/optimize");
    println!("     GET    /api/orchestrator/status");
    println!("     GET    /api/orchestrator/summary");
    println!();
    println!("Press Ctrl+C to stop the server\n");

    start_server(config).await
}

pub(crate) async fn recommend_command(spec_path: &PathBuf, _runtime_override: Option<String>) -> Result<()> {
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

pub(crate) async fn profile_command(spec_path: &PathBuf, name: Option<String>) -> Result<()> {
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

pub(crate) async fn analyze_logs_command(name: &str) -> Result<()> {
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
    let rt = orchestr8::runtime::create_runtime(&workload_state.runtime).await?;
    let logs = rt.logs(&workload_state.instance, false).await?;

    let analysis = analyzer.analyze(&logs);
    print!("{}", format_analysis_report(&analysis));

    Ok(())
}

pub(crate) async fn migration_advice_command(name: &str, target: &str) -> Result<()> {
    use orchestr8::ai::migration::{format_migration_advice, MigrationAdvisor};
    use orchestr8::config::Config;

    println!("🔄 Migration Advisor for '{}'\n", name);

    let config = Config::load();
    let advisor = MigrationAdvisor::new(config.migration);

    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    let target_runtime: RuntimeKind = target.parse()?;

    let workload = Workload::from_file(&workload_state.spec_path)?;
    let advice = advisor.advise(&workload, workload_state.runtime, target_runtime);
    print!("{}", format_migration_advice(&advice));

    Ok(())
}

pub(crate) async fn scaling_advice_command() -> Result<()> {
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
        .unwrap_or_default()
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

pub(crate) async fn config_command(show: bool, init: bool) -> Result<()> {
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

pub(crate) async fn drift_command(name: &str, reconcile: bool) -> Result<()> {
    use orchestr8::drift::{format_drift_report, DriftDetector};

    println!("🔍 Checking drift for '{}'\n", name);

    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    let spec = Workload::from_file(&workload_state.spec_path)?;
    let detector = DriftDetector::new();
    let report = detector.detect(&spec, workload_state);

    print!("{}", format_drift_report(&report));

    if reconcile && report.has_drift {
        println!("\n🔧 Reconciliation Actions:");
        for action in &report.reconciliation_plan {
            println!("  - [{}] {} {}", action.action_type, action.description,
                if action.requires_restart { "(requires restart)" } else { "" });
        }
        println!("\n💡 To apply these changes, re-run the workload with: orchestr8 run");
    }

    Ok(())
}

pub(crate) async fn policy_check_command(spec_path: &PathBuf, policy_name: &str) -> Result<()> {
    use orchestr8::policy::{format_policy_report, PolicyEngine};

    println!("📋 Policy Check\n");

    let workload = Workload::from_file(spec_path)?;

    let engine = match policy_name {
        "production" => PolicyEngine::production(),
        "development" => PolicyEngine::development(),
        _ => {
            // Try loading from file
            let path = std::path::Path::new(policy_name);
            if path.exists() {
                PolicyEngine::load(path)?
            } else {
                anyhow::bail!("Unknown policy set: {}. Use 'production', 'development', or a file path.", policy_name);
            }
        }
    };

    let result = engine.evaluate(&workload);
    print!("{}", format_policy_report(&result));

    if !result.passed {
        anyhow::bail!("Policy check failed with {} violation(s)", result.violations.len());
    }

    Ok(())
}

pub(crate) async fn deps_command(action: DepsAction) -> Result<()> {
    use orchestr8::dependencies::{format_dependency_report, DependencyGraph};

    let graph_path = DependencyGraph::default_path();
    let mut graph = DependencyGraph::load(&graph_path)?;

    match action {
        DepsAction::Add { workload, dependency } => {
            graph.add_dependency(&workload, &dependency);
            graph.save(&graph_path)?;
            println!("✅ Added dependency: {} -> {}", workload, dependency);

            let issues = graph.validate();
            if !issues.is_empty() {
                println!("\n⚠️  Warnings:");
                for issue in &issues {
                    println!("  - {}", issue);
                }
            }
        }
        DepsAction::Remove { workload, dependency } => {
            graph.remove_dependency(&workload, &dependency);
            graph.save(&graph_path)?;
            println!("✅ Removed dependency: {} -> {}", workload, dependency);
        }
        DepsAction::Show => {
            print!("{}", format_dependency_report(&graph));
        }
        DepsAction::Impact { workload } => {
            let impact = graph.impact_analysis(&workload);
            println!("Impact Analysis for '{}':\n", workload);
            println!("  Severity: {}", impact.severity);
            println!("  Affected workloads: {}", impact.cascade_count);
            if !impact.affected_workloads.is_empty() {
                println!("\n  Cascade:");
                for affected in &impact.affected_workloads {
                    println!("    - {}", affected);
                }
            }
        }
        DepsAction::Order => {
            match graph.startup_order() {
                Ok(order) => {
                    println!("Startup Order:\n");
                    for (i, name) in order.iter().enumerate() {
                        println!("  {}. {}", i + 1, name);
                    }
                }
                Err(e) => {
                    println!("❌ {}", e);
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn audit_command(last: usize, workload: Option<String>, summary: bool) -> Result<()> {
    use orchestr8::audit::{format_audit_report, AuditLog};

    let audit_path = AuditLog::default_path();
    let log = AuditLog::load(&audit_path)?;

    if summary {
        let s = log.summary();
        println!("📊 Audit Summary\n");
        println!("Total events: {}", s.total_events);
        println!("Successes: {}", s.successes);
        println!("Failures: {}", s.failures);
        println!("Unique workloads: {}", s.unique_workloads);

        if !s.events_by_action.is_empty() {
            println!("\nBy Action:");
            let mut actions: Vec<_> = s.events_by_action.iter().collect();
            actions.sort_by(|a, b| b.1.cmp(a.1));
            for (action, count) in actions {
                println!("  {}: {}", action, count);
            }
        }
        return Ok(());
    }

    if let Some(ref name) = workload {
        let events = log.events_for(name);
        if events.is_empty() {
            println!("No audit events for '{}'", name);
        } else {
            println!("📋 Audit Events for '{}'\n", name);
            for event in events {
                println!(
                    "  [{}] {} {} - {}",
                    event.timestamp.chars().take(19).collect::<String>(),
                    event.result,
                    event.action,
                    event.message,
                );
            }
        }
        return Ok(());
    }

    print!("{}", format_audit_report(&log, last));

    Ok(())
}

pub(crate) async fn template_command(
    name: &str,
    workload_name: Option<String>,
    owner: &str,
    project: &str,
    registry: &str,
    output: Option<PathBuf>,
    list: bool,
) -> Result<()> {
    use orchestr8::templates::{self, TemplateKind, TemplateParams};

    if list || name == "list" {
        print!("{}", templates::format_template_list());
        return Ok(());
    }

    let kind: TemplateKind = name.parse()?;

    let wl_name = workload_name.unwrap_or_else(|| format!("my-{}", name));

    let params = TemplateParams {
        name: wl_name.clone(),
        owner: owner.to_string(),
        project: project.to_string(),
        registry: registry.to_string(),
        ..Default::default()
    };

    let spec = templates::generate(&kind, &params);
    let yaml = serde_yaml::to_string(&spec)?;

    if let Some(path) = output {
        std::fs::write(&path, &yaml)?;
        println!("✅ Generated {} template: {}", name, path.display());
        println!("   Workload name: {}", wl_name);
    } else {
        println!("# Generated from '{}' template\n", name);
        println!("{}", yaml);
    }

    Ok(())
}

pub(crate) async fn sla_command(action: SlaAction) -> Result<()> {
    use orchestr8::sla::{format_sla_report, SlaEngine, SlaObservation, SlaTarget};

    // Persist SLA targets via a simple JSON file
    let sla_path = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".orchestr8/sla.json")
    };

    let load_engine = || -> Result<SlaEngine> {
        if sla_path.exists() {
            let content = std::fs::read_to_string(&sla_path)?;
            let targets: Vec<SlaTarget> = serde_json::from_str(&content)?;
            let mut engine = SlaEngine::new();
            for target in targets {
                engine.add_target(target);
            }
            Ok(engine)
        } else {
            Ok(SlaEngine::new())
        }
    };

    let save_engine = |engine: &SlaEngine| -> Result<()> {
        if let Some(parent) = sla_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let targets = engine.list_targets();
        let content = serde_json::to_string_pretty(&targets)?;
        std::fs::write(&sla_path, content)?;
        Ok(())
    };

    match action {
        SlaAction::Add { workload, tier } => {
            let mut engine = load_engine()?;
            let target = match tier.as_str() {
                "standard" => SlaTarget::standard(&workload),
                "high-availability" | "ha" => SlaTarget::high_availability(&workload),
                "best-effort" | "be" => SlaTarget::best_effort(&workload),
                _ => anyhow::bail!("Unknown SLA tier: {}. Use standard, high-availability, or best-effort.", tier),
            };
            println!("✅ Added SLA target for '{}' ({})", workload, tier);
            println!("   Uptime target: {:.2}%", target.uptime_target_pct);
            if let Some(lat) = target.max_latency_ms {
                println!("   Max latency: {:.0}ms", lat);
            }
            if let Some(err) = target.max_error_rate_pct {
                println!("   Max error rate: {:.2}%", err);
            }
            engine.add_target(target);
            save_engine(&engine)?;
        }
        SlaAction::Check {
            workload,
            uptime,
            latency,
            error_rate,
            restarts,
        } => {
            let engine = load_engine()?;
            let observation = SlaObservation {
                uptime_pct: uptime,
                avg_latency_ms: latency,
                error_rate_pct: error_rate,
                restarts,
                observation_period: "24h".to_string(),
            };

            match engine.evaluate(&workload, &observation) {
                Some(report) => {
                    print!("{}", format_sla_report(&report));
                }
                None => {
                    println!("❌ No SLA target found for '{}'. Add one with: orchestr8 sla add {}", workload, workload);
                }
            }
        }
        SlaAction::List => {
            let engine = load_engine()?;
            let targets = engine.list_targets();
            if targets.is_empty() {
                println!("No SLA targets defined.");
                println!("Add one with: orchestr8 sla add <workload> --tier standard");
            } else {
                println!("📋 SLA Targets\n");
                for target in targets {
                    println!("  {} - {:.2}% uptime", target.workload, target.uptime_target_pct);
                    if let Some(lat) = target.max_latency_ms {
                        println!("    Max latency: {:.0}ms", lat);
                    }
                    if let Some(err) = target.max_error_rate_pct {
                        println!("    Max error rate: {:.2}%", err);
                    }
                    if let Some(restarts) = target.max_restarts_per_day {
                        println!("    Max restarts/day: {}", restarts);
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn secrets_command(action: SecretsAction) -> Result<()> {
    use orchestr8::secrets::{format_secrets_list, SecretStore};

    let path = SecretStore::default_path();
    let mut store = SecretStore::load(&path)?;

    match action {
        SecretsAction::Create { name, namespace } => {
            store.create_secret(&name, &namespace);
            store.save(&path)?;
            orchestr8::metrics::record_secret_operation("create");
            emit_event(
                orchestr8::events::EventSeverity::Info,
                orchestr8::events::EventCategory::SecretRotation,
                "cli",
                None,
                "Secret created",
                &format!("Created secret '{}' in namespace '{}'", name, namespace),
            );
            println!("✅ Created secret '{}' in namespace '{}'", name, namespace);
        }
        SecretsAction::Set { secret, key, value } => {
            store.set(&secret, &key, &value)?;
            store.save(&path)?;
            orchestr8::metrics::record_secret_operation("set");
            emit_event(
                orchestr8::events::EventSeverity::Info,
                orchestr8::events::EventCategory::SecretRotation,
                "cli",
                None,
                "Secret updated",
                &format!("Set key '{}' in secret '{}'", key, secret),
            );
            println!("✅ Set key '{}' in secret '{}'", key, secret);
        }
        SecretsAction::Get { secret, key } => {
            let value = store.get(&secret, &key)?;
            orchestr8::metrics::record_secret_operation("get");
            println!("{}", value);
        }
        SecretsAction::List => {
            let summaries = store.list();
            print!("{}", format_secrets_list(&summaries));
        }
        SecretsAction::Audit => {
            let alerts = store.audit_rotation();
            if alerts.is_empty() {
                println!("✅ All secrets are within rotation policy limits.");
            } else {
                println!("⚠️  Rotation Alerts:\n");
                for alert in &alerts {
                    println!("  [{}] {}", alert.severity, alert.message);
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn events_command(last: usize, severity: Option<String>, summary: bool) -> Result<()> {
    use orchestr8::events::{format_event_list, format_event_summary, EventBus, EventSeverity};

    let path = EventBus::default_path();
    let bus = EventBus::load(&path)?;

    if summary {
        let s = bus.summary();
        print!("{}", format_event_summary(&s));
        return Ok(());
    }

    let events = if let Some(sev) = severity {
        let min_severity: EventSeverity = sev.parse()?;
        bus.events_by_severity(&min_severity)
    } else {
        bus.last_n(last)
    };

    print!("{}", format_event_list(&events, last));

    Ok(())
}

pub(crate) async fn env_command(action: EnvAction) -> Result<()> {
    use orchestr8::environments::{EnvTier, EnvironmentManager, PromotionRequest, PromotionStrategy, format_env_list};

    let path = EnvironmentManager::default_path();
    let mut manager = EnvironmentManager::load(&path)?;

    match action {
        EnvAction::Create { name, tier } => {
            let env_tier: EnvTier = tier.parse().expect("EnvTier::from_str is infallible");
            manager.create_env(&name, env_tier);
            manager.save(&path)?;
            println!("✅ Created environment '{}' ({})", name, tier);
        }
        EnvAction::List => {
            let envs = manager.list_envs();
            print!("{}", format_env_list(&envs));
        }
        EnvAction::Promote { workload, from, to } => {
            let request = PromotionRequest {
                workload: workload.clone(),
                from_env: from.clone(),
                to_env: to.clone(),
                strategy: PromotionStrategy::TierAdjusted,
                require_approval: false,
            };
            let result = manager.promote(&request)?;
            manager.save(&path)?;
            orchestr8::metrics::record_env_promotion(&from, &to, true);
            emit_event(
                orchestr8::events::EventSeverity::Info,
                orchestr8::events::EventCategory::Deployment,
                "cli",
                Some(&workload),
                "Environment promotion",
                &format!("Promoted from {} to {}", from, to),
            );
            println!("✅ Promoted '{}' from '{}' to '{}'", workload, from, to);
            if !result.changes.is_empty() {
                println!("   Changes:");
                for change in &result.changes {
                    println!("     - {}: {} -> {} ({})", change.field, change.from_value, change.to_value, change.reason);
                }
            }
            if !result.warnings.is_empty() {
                println!("   Warnings:");
                for warn in &result.warnings {
                    println!("     - {}", warn);
                }
            }
        }
        EnvAction::Parity { env1, env2 } => {
            // Parity needs a workload name - check all workloads in env1
            let env = manager.get_env(&env1)
                .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", env1))?;
            let workload_names: Vec<String> = env.workloads.keys().cloned().collect();

            if workload_names.is_empty() {
                println!("No workloads in environment '{}' to compare.", env1);
                return Ok(());
            }

            println!("Environment Parity: {} vs {}\n", env1, env2);
            for wl_name in &workload_names {
                match manager.check_parity(&env1, &env2, wl_name) {
                    Ok(report) => {
                        if report.in_sync {
                            println!("  ✅ '{}': In parity", wl_name);
                        } else {
                            println!("  ⚠️  '{}': Differences found", wl_name);
                            for diff in &report.diffs {
                                println!("      [{}] {}: {} vs {}", diff.severity, diff.field, diff.env_a_value, diff.env_b_value);
                            }
                        }
                    }
                    Err(e) => {
                        println!("  ❌ '{}': {}", wl_name, e);
                    }
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn schedule_command(action: ScheduleAction) -> Result<()> {
    use orchestr8::scheduler::{
        format_schedule_decision, format_utilization, ScheduleRequest, ScheduleStrategy,
        Scheduler, Priority,
    };

    let path = Scheduler::default_path();
    let mut scheduler = Scheduler::load(&path)?;

    match action {
        ScheduleAction::Place {
            name,
            cpu,
            memory,
            strategy,
            prefer,
        } => {
            let sched_strategy: ScheduleStrategy = strategy.parse()?;
            scheduler.set_strategy(sched_strategy);

            // Load affinity scores to inform scheduling decisions
            if let Ok(affinity_engine) = orchestr8::ai::affinity::AffinityEngine::load(
                &orchestr8::ai::affinity::AffinityEngine::default_path(),
            ) {
                use orchestr8::ai::affinity::WorkloadClass;
                let class = WorkloadClass::Microservice;
                let scores = affinity_engine.recommend(&class);
                let affinity_map: std::collections::HashMap<RuntimeKind, f64> = scores
                    .into_iter()
                    .map(|s| (s.runtime, s.composite_score))
                    .collect();
                scheduler.set_affinity_scores(affinity_map);
            }

            let preferred_runtime = prefer.as_deref().and_then(|p| p.parse::<RuntimeKind>().ok());

            let request = ScheduleRequest {
                workload_name: name,
                cpu_required: cpu,
                memory_required_mb: memory,
                storage_required_mb: 10240,
                gpu_required: 0,
                preferred_runtime,
                constraints: vec![],
                priority: Priority::Normal,
            };

            match scheduler.schedule(&request) {
                Ok(decision) => {
                    print!("{}", format_schedule_decision(&decision));
                    scheduler.save(&path)?;
                    orchestr8::metrics::record_scheduler_placement(
                        &decision.selected_runtime.to_string(),
                        &strategy,
                        true,
                    );
                }
                Err(e) => {
                    orchestr8::metrics::record_scheduler_placement("none", &strategy, false);
                    println!("❌ {}", e);
                }
            }
        }
        ScheduleAction::Utilization => {
            let utils = scheduler.utilization_summary();
            print!("{}", format_utilization(&utils));
        }
        ScheduleAction::Optimize => {
            let suggestions = scheduler.optimize();
            if suggestions.is_empty() {
                println!("✅ No optimization suggestions. All runtimes look good.");
            } else {
                println!("Optimization Suggestions:\n");
                for s in &suggestions {
                    print!("  [{}] {}", s.category, s.message);
                    if let Some(saving) = s.potential_saving {
                        print!(" (potential saving: ${:.2}/day)", saving);
                    }
                    println!();
                }
            }
        }
        ScheduleAction::Placements => {
            let placements = scheduler.placements();
            if placements.is_empty() {
                println!("No workloads placed.");
            } else {
                println!("Current Placements:\n");
                for p in placements {
                    println!(
                        "  {} -> {} ({:.0} CPU, {} MB)",
                        p.workload_name, p.runtime, p.cpu_reserved, p.memory_reserved_mb
                    );
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn orchestrate_command(action: OrchestrateAction) -> Result<()> {
    use orchestr8::orchestrator::{
        format_health_summary, format_rolling_update, format_workload_list, Orchestrator,
    };

    let path = Orchestrator::default_path();
    let mut orch = Orchestrator::load(&path)?;

    match action {
        OrchestrateAction::Register { name, runtime } => {
            let rt: RuntimeKind = match runtime.parse() {
                Ok(rt) => rt,
                Err(e) => anyhow::bail!("{}", e),
            };
            orch.register(&name, rt, None);
            orch.save(&path)?;
            println!("✅ Registered '{}' for health monitoring ({})", name, runtime);
        }
        OrchestrateAction::Status => {
            let list = orch.list_workloads();
            print!("{}", format_workload_list(&list));
        }
        OrchestrateAction::Summary => {
            let summary = orch.health_summary();
            print!("{}", format_health_summary(&summary));
        }
        OrchestrateAction::RollingUpdate { name, replicas } => {
            let statuses = orch.rolling_update(&name, replicas, None);
            print!("{}", format_rolling_update(&statuses));
        }
        OrchestrateAction::ResetCircuit { name } => {
            if orch.reset_circuit(&name) {
                orch.save(&path)?;
                println!("✅ Circuit breaker reset for '{}'", name);
            } else {
                println!("❌ Workload '{}' not found", name);
            }
        }
        OrchestrateAction::HealthCheck => {
            let statuses = collect_health_statuses(&orch, &StateStore::load(&StateStore::default_path())?).await;
            let actions = orch.run_health_checks_from_statuses(&statuses);
            orch.save(&path)?;

            if actions.is_empty() {
                println!("Health check complete. No actions required.");
            } else {
                println!("Health check actions:");
                for action in &actions {
                    println!("  - {:?}", action);
                }
            }
        }
        OrchestrateAction::Watch { interval } => {
            println!("Watching health every {} seconds (Ctrl+C to stop)\n", interval);

            loop {
                let statuses = collect_health_statuses(&orch, &StateStore::load(&StateStore::default_path())?).await;
                let actions = orch.run_health_checks_from_statuses(&statuses);
                orch.save(&path)?;

                let now = chrono::Utc::now().format("%H:%M:%S");
                if actions.is_empty() {
                    println!("[{}] Health OK ({} workloads checked)", now, statuses.len());
                } else {
                    println!("[{}] {} action(s):", now, actions.len());
                    for action in &actions {
                        println!("  - {:?}", action);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            }
        }
    }

    Ok(())
}

pub(crate) async fn affinity_command(action: AffinityAction) -> Result<()> {
    use orchestr8::ai::affinity::{format_affinity_report, AffinityEngine, WorkloadClass};

    let path = AffinityEngine::default_path();
    let engine = AffinityEngine::load(&path)?;

    match action {
        AffinityAction::Recommend { class } => {
            let wl_class: WorkloadClass = class.parse()?;
            let scores = engine.recommend(&wl_class);
            if let Some(top) = scores.first() {
                orchestr8::metrics::record_affinity_recommendation(&class, &top.runtime.to_string());
            }
            print!("{}", format_affinity_report(&wl_class, &scores));
        }
        AffinityAction::Matrix => {
            let matrix = engine.compatibility_matrix();
            println!("Compatibility Matrix:\n");
            let mut entries: Vec<_> = matrix.iter().collect();
            entries.sort_by_key(|((class, rt), _)| (format!("{}", class), format!("{}", rt)));
            for ((class, rt), entry) in &entries {
                let compat = if entry.compatible { "✓" } else { "✗" };
                println!(
                    "  {} {} + {} (score: {:.0}%, deployments: {})",
                    compat,
                    class,
                    rt,
                    entry.score * 100.0,
                    entry.deployments,
                );
            }
        }
        AffinityAction::Stats => {
            let stats = engine.stats();
            println!("Affinity Learning Stats:\n");
            println!("  Total outcomes: {}", stats.total_outcomes);
            println!("  Successes: {}", stats.successes);
            println!("  Failures: {}", stats.failures);
            println!("  Known incompatibilities: {}", stats.incompatibilities);

            if !stats.by_runtime.is_empty() {
                println!("\n  By Runtime:");
                for (rt, count) in &stats.by_runtime {
                    println!("    {}: {}", rt, count);
                }
            }
            if !stats.by_class.is_empty() {
                println!("\n  By Workload Class:");
                for (class, count) in &stats.by_class {
                    println!("    {}: {}", class, count);
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn rollback_command(name: &str) -> Result<()> {
    use orchestr8::backup::{Backup, SnapshotManager};
    use orchestr8::runtime::create_runtime;

    println!("Rolling back workload '{}'...\n", name);

    // Find latest snapshot
    let snap_mgr = SnapshotManager::new();
    let snapshot_path = snap_mgr
        .latest_snapshot(name)?
        .ok_or_else(|| anyhow::anyhow!("No snapshot found for workload '{}'", name))?;

    println!("  Found snapshot: {}", snapshot_path.display());

    // Load snapshot
    let backup = Backup::load(&snapshot_path)?;
    let snapshot_ws = backup
        .workloads
        .first()
        .ok_or_else(|| anyhow::anyhow!("Snapshot is empty"))?;

    // Stop current instance (best-effort)
    let state = StateStore::load(&StateStore::default_path())?;
    if let Some(current) = state.get(name) {
        match create_runtime(&current.runtime).await {
            Ok(runtime) => {
                if let Err(e) = runtime.stop(&current.instance).await {
                    tracing::warn!("Failed to stop current instance: {}", e);
                }
            }
            Err(e) => tracing::warn!("Failed to create runtime for stop: {}", e),
        }
    }

    // Re-deploy from snapshot
    let runtime = create_runtime(&snapshot_ws.runtime).await?;
    let spec = Workload::from_file(&snapshot_ws.spec_path)?;
    let image = runtime.build(&spec).await?;
    let instance = runtime.run(&image, &spec).await?;

    println!("  Re-deployed instance: {} ({})", instance.name, instance.id);

    // Update state
    let mut state = StateStore::load(&StateStore::default_path())?;
    state.upsert(
        name.to_string(),
        orchestr8::state::WorkloadState {
            name: name.to_string(),
            runtime: snapshot_ws.runtime,
            instance,
            spec_path: snapshot_ws.spec_path.clone(),
            created_at: snapshot_ws.created_at.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    );
    state.save(&StateStore::default_path())?;

    emit_event(
        orchestr8::events::EventSeverity::Warning,
        orchestr8::events::EventCategory::Deployment,
        "cli",
        Some(name),
        "Workload rolled back",
        &format!("Rolled back to snapshot {}", snapshot_path.display()),
    );

    println!("\nRollback complete for '{}'", name);

    Ok(())
}

/// Collect live HealthStatus for all managed workloads, grouping by runtime
/// to avoid creating duplicate runtime clients (N+1).
async fn collect_health_statuses(
    orch: &orchestr8::orchestrator::Orchestrator,
    state: &StateStore,
) -> std::collections::HashMap<String, orchestr8::orchestrator::HealthStatus> {
    use orchestr8::orchestrator::HealthStatus;
    use orchestr8::runtime::{create_runtime, InstanceState};

    let managed = orch.list_workloads();
    let mut statuses = std::collections::HashMap::new();

    // Group managed workloads by runtime
    let mut by_runtime: std::collections::HashMap<RuntimeKind, Vec<_>> =
        std::collections::HashMap::new();
    for mw in &managed {
        if let Some(ws) = state.get(&mw.name) {
            by_runtime.entry(ws.runtime).or_default().push((mw.name.clone(), ws.instance.clone()));
        }
    }

    for (kind, workloads) in by_runtime {
        match create_runtime(&kind).await {
            Ok(runtime) => {
                for (name, instance) in workloads {
                    let hs = match runtime.status(&instance).await {
                        Ok(status) => match status.state {
                            InstanceState::Running if status.ready => HealthStatus::Healthy,
                            InstanceState::Running => HealthStatus::Degraded,
                            InstanceState::Failed => HealthStatus::Unhealthy,
                            _ => HealthStatus::Unknown,
                        },
                        Err(e) => {
                            tracing::warn!("Failed to get status for {}: {}", name, e);
                            HealthStatus::Unknown
                        }
                    };
                    statuses.insert(name, hs);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create {} runtime: {}", kind, e);
                for (name, _) in workloads {
                    statuses.insert(name, HealthStatus::Unknown);
                }
            }
        }
    }

    statuses
}

pub(crate) async fn diff_command(name: &str) -> Result<()> {
    use orchestr8::drift::{format_live_diff, DiffRow, LiveDiffReport};
    use orchestr8::runtime::create_runtime;

    println!("Comparing spec vs stored vs live state for '{}'\n", name);

    let state = StateStore::load(&StateStore::default_path())?;
    let workload_state = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?;

    // Load spec
    let spec = Workload::from_file(&workload_state.spec_path)?;

    // Query live status
    let runtime = create_runtime(&workload_state.runtime).await?;
    let live_status = runtime.status(&workload_state.instance).await?;

    // Build diff rows
    let spec_runtime = format!("{:?}", spec.runtime.preferred);
    let stored_runtime = workload_state.runtime.to_string();
    let live_runtime = stored_runtime.clone(); // runtime doesn't change live

    let spec_image = format!("{}/{}:latest", spec.build.registry, spec.metadata.name);
    let stored_image = workload_state.instance.image.clone();

    let rows = vec![
        DiffRow {
            field: "runtime".to_string(),
            spec_value: spec_runtime.clone(),
            stored_value: stored_runtime.clone(),
            live_value: live_runtime.clone(),
            matches: spec_runtime == stored_runtime,
        },
        DiffRow {
            field: "image".to_string(),
            spec_value: spec_image.clone(),
            stored_value: stored_image.clone(),
            live_value: stored_image.clone(),
            matches: spec_image == stored_image,
        },
        DiffRow {
            field: "state".to_string(),
            spec_value: "running".to_string(),
            stored_value: "-".to_string(),
            live_value: format!("{}", live_status.state),
            matches: live_status.state == orchestr8::runtime::InstanceState::Running,
        },
        DiffRow {
            field: "ready".to_string(),
            spec_value: "true".to_string(),
            stored_value: "-".to_string(),
            live_value: format!("{}", live_status.ready),
            matches: live_status.ready,
        },
        DiffRow {
            field: "restarts".to_string(),
            spec_value: "0".to_string(),
            stored_value: "-".to_string(),
            live_value: format!("{}", live_status.restart_count),
            matches: live_status.restart_count == 0,
        },
        DiffRow {
            field: "cpu".to_string(),
            spec_value: spec.requirements.cpu.clone(),
            stored_value: "-".to_string(),
            live_value: "-".to_string(),
            matches: true,
        },
        DiffRow {
            field: "memory".to_string(),
            spec_value: spec.requirements.memory.clone(),
            stored_value: "-".to_string(),
            live_value: "-".to_string(),
            matches: true,
        },
    ];

    let has_differences = rows.iter().any(|r| !r.matches);

    let report = LiveDiffReport {
        workload_name: name.to_string(),
        rows,
        has_differences,
    };

    print!("{}", format_live_diff(&report));

    Ok(())
}

pub(crate) async fn webhook_command(action: WebhookAction) -> Result<()> {
    use orchestr8::events::{
        ChannelType, EventBus, EventCategory, EventSeverity, NotificationChannel,
    };

    let path = EventBus::default_path();
    let mut bus = EventBus::load(&path)?;

    match action {
        WebhookAction::Add {
            name,
            url,
            method,
            severity,
        } => {
            let min_severity: EventSeverity = severity.parse()?;

            let channel = NotificationChannel {
                name: name.clone(),
                channel_type: ChannelType::Webhook {
                    url: url.clone(),
                    method: method.clone(),
                },
                enabled: true,
                min_severity,
                categories: vec![],
            };

            bus.add_channel(channel);
            bus.save(&path)?;

            println!("Added webhook channel '{}' -> {} ({})", name, url, method);
        }
        WebhookAction::Remove { name } => {
            if bus.remove_channel(&name) {
                bus.save(&path)?;
                println!("Removed webhook channel '{}'", name);
            } else {
                println!("Channel '{}' not found", name);
            }
        }
        WebhookAction::List => {
            let channels = bus.channels();
            if channels.is_empty() {
                println!("No notification channels configured.");
                return Ok(());
            }

            println!("Notification Channels:\n");
            println!(
                "  {:<20} {:<30} {:<10} {:<10}",
                "Name", "Type", "Enabled", "Min Severity"
            );
            println!("  {}", "-".repeat(74));

            for ch in channels {
                let type_str = format!("{}", ch.channel_type);
                let enabled_str = if ch.enabled { "yes" } else { "no" };
                println!(
                    "  {:<20} {:<30} {:<10} {:<10}",
                    ch.name,
                    type_str,
                    enabled_str,
                    format!("{}", ch.min_severity),
                );
            }
        }
        WebhookAction::Test { name } => {
            let channel_exists = bus.channels().iter().any(|c| c.name == name);
            if !channel_exists {
                anyhow::bail!("Channel '{}' not found", name);
            }

            bus.emit_simple(
                EventSeverity::Info,
                EventCategory::SystemAlert,
                "webhook-test",
                None,
                "Webhook Test",
                &format!("Test notification for channel '{}'", name),
            );
            bus.save(&path)?;
            println!("Test notification sent to channel '{}'", name);
        }
    }

    Ok(())
}

/// Emit an event and save to the event bus (best-effort, errors are logged).
pub(crate) fn emit_event(
    severity: orchestr8::events::EventSeverity,
    category: orchestr8::events::EventCategory,
    source: &str,
    workload: Option<&str>,
    title: &str,
    message: &str,
) {
    let path = orchestr8::events::EventBus::default_path();
    match orchestr8::events::EventBus::load(&path) {
        Ok(mut bus) => {
            bus.emit_simple(severity, category, source, workload, title, message);
            let _ = bus.save(&path);
        }
        Err(e) => {
            tracing::warn!("Failed to emit event: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestr8::runtime::RuntimeKind;
    use orchestr8::spec::*;
    use std::collections::HashMap;

    // ---------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------

    fn make_valid_workload() -> Workload {
        Workload {
            api_version: "orchestr8/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test-app".to_string(),
                owner: "test-team".to_string(),
                project: "demo".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/test".to_string(),
                build_args: HashMap::new(),
            },
            requirements: ResourceRequirements {
                cpu: "2".to_string(),
                memory: "4Gi".to_string(),
                storage: "20Gi".to_string(),
                gpu: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Container, RuntimeType::Kube],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
        }
    }

    fn write_valid_spec(dir: &std::path::Path) -> PathBuf {
        let spec = make_valid_workload();
        let yaml = serde_yaml::to_string(&spec).unwrap();
        let path = dir.join("workload.yaml");
        std::fs::write(&path, yaml).unwrap();
        path
    }

    // ---------------------------------------------------------------
    // validate_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_validate_command_valid_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = validate_command(&path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_command_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.yaml");
        std::fs::write(&path, "not: valid: yaml: [[[").unwrap();
        let result = validate_command(&path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_command_missing_file() {
        let path = PathBuf::from("/tmp/nonexistent_orchestr8_test.yaml");
        let result = validate_command(&path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_command_invalid_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.yaml");
        let mut spec = make_valid_workload();
        spec.api_version = "orchestr8/v99".to_string();
        let yaml = serde_yaml::to_string(&spec).unwrap();
        std::fs::write(&path, yaml).unwrap();
        let result = validate_command(&path).await;
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // cost_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_cost_command_all_providers() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = cost_command(&path, "all").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cost_command_aws() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = cost_command(&path, "aws").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cost_command_azure() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = cost_command(&path, "azure").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cost_command_gcp() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = cost_command(&path, "gcp").await;
        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------
    // recommend_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_recommend_command() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = recommend_command(&path, None).await;
        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------
    // profile_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_profile_command_from_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = profile_command(&path, None).await;
        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------
    // policy_check_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_policy_check_command_development_pass() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = policy_check_command(&path, "development").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_policy_check_command_invalid_spec() {
        let path = PathBuf::from("/tmp/nonexistent_orchestr8_policy.yaml");
        let result = policy_check_command(&path, "production").await;
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // scaling_advice_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_scaling_advice_command() {
        let result = scaling_advice_command().await;
        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------
    // template_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_template_command_list() {
        let result = template_command(
            "list", None, "team", "default", "ghcr.io/org", None, true,
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_command_web_app_stdout() {
        let result = template_command(
            "web-app", Some("my-web".to_string()), "team", "demo", "ghcr.io/org", None, false,
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_command_rest_api_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("api.yaml");
        let result = template_command(
            "rest-api", Some("my-api".to_string()), "team", "demo", "ghcr.io/org",
            Some(out.clone()), false,
        ).await;
        assert!(result.is_ok());
        assert!(out.exists());

        // Verify the generated file is a valid workload spec
        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: Workload = serde_yaml::from_str(&content).unwrap();
        assert_eq!(parsed.metadata.name, "my-api");
    }

    #[tokio::test]
    async fn test_template_command_database() {
        let result = template_command(
            "database", None, "team", "demo", "ghcr.io/org", None, false,
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_command_unknown() {
        let result = template_command(
            "unknown-kind", None, "team", "demo", "ghcr.io/org", None, false,
        ).await;
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // list_command (empty state)
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_list_command_empty_state() {
        // Use a temp dir to avoid touching real state
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        // Create an empty state file
        let store = orchestr8::state::StateStore::new();
        store.save(&state_path).unwrap();

        // list_command uses StateStore::default_path() so we can't easily
        // redirect it. Instead, test the underlying StateStore directly.
        let loaded = orchestr8::state::StateStore::load(&state_path).unwrap();
        assert!(loaded.list().is_empty());
    }

    // ---------------------------------------------------------------
    // metrics_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_metrics_gather_not_empty() {
        orchestr8::metrics::init();
        let output = orchestr8::metrics::gather();
        // Metrics output should contain at least the HELP/TYPE headers
        assert!(!output.is_empty());
    }

    // ---------------------------------------------------------------
    // config_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_config_show() {
        // The show branch serializes Config to YAML and prints it
        let config = orchestr8::config::Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("engine"));
        assert!(yaml.contains("migration"));
        assert!(yaml.contains("scaling"));
    }

    #[tokio::test]
    async fn test_config_init_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        let config = orchestr8::config::Config::default();
        config.save_to(&path).unwrap();
        assert!(path.exists());

        let loaded = orchestr8::config::Config::load_from(&path);
        assert_eq!(loaded.engine.enable_scoring, config.engine.enable_scoring);
    }

    #[tokio::test]
    async fn test_config_load_nonexistent_returns_default() {
        let path = PathBuf::from("/tmp/nonexistent_orchestr8_config.yaml");
        let config = orchestr8::config::Config::load_from(&path);
        let default = orchestr8::config::Config::default();
        assert_eq!(config.engine.enable_scoring, default.engine.enable_scoring);
    }

    // ---------------------------------------------------------------
    // audit (underlying AuditLog)
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_audit_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.json");
        let log = orchestr8::audit::AuditLog::new();
        log.save(&path).unwrap();

        let loaded = orchestr8::audit::AuditLog::load(&path).unwrap();
        let summary = loaded.summary();
        assert_eq!(summary.total_events, 0);
    }

    #[tokio::test]
    async fn test_audit_record_and_summary() {
        use orchestr8::audit::{ActionResult, AuditAction, AuditLog};

        let mut log = AuditLog::new();
        log.record(
            AuditAction::Deploy,
            "web-app",
            Some("podman"),
            ActionResult::Success,
            "Deployed successfully",
            None,
        );
        log.record(
            AuditAction::Build,
            "api-svc",
            Some("kubernetes"),
            ActionResult::Failure,
            "Build failed",
            None,
        );

        let summary = log.summary();
        assert_eq!(summary.total_events, 2);
        assert_eq!(summary.successes, 1);
        assert_eq!(summary.failures, 1);
    }

    // ---------------------------------------------------------------
    // emit_event (underlying EventBus)
    // ---------------------------------------------------------------

    #[test]
    fn test_event_bus_emit_and_summary() {
        use orchestr8::events::{EventBus, EventCategory, EventSeverity};

        let mut bus = EventBus::new();
        bus.emit_simple(
            EventSeverity::Info,
            EventCategory::Deployment,
            "test",
            Some("my-app"),
            "Test event",
            "Testing event emission",
        );
        bus.emit_simple(
            EventSeverity::Warning,
            EventCategory::DriftDetected,
            "test",
            Some("my-app"),
            "Drift",
            "Config drift detected",
        );

        let summary = bus.summary();
        assert_eq!(summary.total_events, 2);
    }

    #[test]
    fn test_event_bus_save_and_load() {
        use orchestr8::events::{EventBus, EventCategory, EventSeverity};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.json");

        let mut bus = EventBus::new();
        bus.emit_simple(
            EventSeverity::Error,
            EventCategory::Deployment,
            "cli",
            Some("svc"),
            "Deploy failed",
            "Timeout",
        );
        bus.save(&path).unwrap();

        let loaded = EventBus::load(&path).unwrap();
        assert_eq!(loaded.summary().total_events, 1);
    }

    // ---------------------------------------------------------------
    // StateStore round-trip
    // ---------------------------------------------------------------

    #[test]
    fn test_state_store_upsert_get_remove() {
        use orchestr8::runtime::Instance;

        let mut store = orchestr8::state::StateStore::new();

        let ws = orchestr8::state::WorkloadState {
            name: "web".to_string(),
            runtime: RuntimeKind::Podman,
            instance: Instance {
                id: "abc123".to_string(),
                name: "web".to_string(),
                runtime: RuntimeKind::Podman,
                image: "ghcr.io/org/web:latest".to_string(),
                created_at: "2025-01-01T00:00:00Z".to_string(),
            },
            spec_path: PathBuf::from("workload.yaml"),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };

        store.upsert("web".to_string(), ws);
        assert_eq!(store.list().len(), 1);
        assert!(store.get("web").is_some());

        store.remove("web");
        assert!(store.get("web").is_none());
        assert!(store.list().is_empty());
    }

    #[test]
    fn test_state_store_save_and_load() {
        use orchestr8::runtime::Instance;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        let mut store = orchestr8::state::StateStore::new();
        store.upsert(
            "svc".to_string(),
            orchestr8::state::WorkloadState {
                name: "svc".to_string(),
                runtime: RuntimeKind::Kubernetes,
                instance: Instance {
                    id: "pod-xyz".to_string(),
                    name: "svc".to_string(),
                    runtime: RuntimeKind::Kubernetes,
                    image: "ghcr.io/org/svc:v1".to_string(),
                    created_at: "2025-01-01T00:00:00Z".to_string(),
                },
                spec_path: PathBuf::from("svc.yaml"),
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
            },
        );
        store.save(&path).unwrap();

        let loaded = orchestr8::state::StateStore::load(&path).unwrap();
        assert_eq!(loaded.list().len(), 1);
        assert_eq!(loaded.get("svc").unwrap().name, "svc");
    }

    // ---------------------------------------------------------------
    // Template generation for each kind
    // ---------------------------------------------------------------

    #[test]
    fn test_template_generate_all_kinds() {
        use orchestr8::templates::{generate, TemplateKind, TemplateParams};

        let kinds = vec![
            TemplateKind::WebApp,
            TemplateKind::RestApi,
            TemplateKind::Database,
            TemplateKind::Cache,
            TemplateKind::Worker,
            TemplateKind::CronJob,
            TemplateKind::MlTraining,
            TemplateKind::Microservice,
        ];

        let params = TemplateParams::default();

        for kind in kinds {
            let spec = generate(&kind, &params);
            assert_eq!(spec.api_version, "orchestr8/v1");
            assert_eq!(spec.kind, "Workload");
            assert!(spec.validate().is_ok(), "template {:?} produced invalid spec", kind);
        }
    }

    // ---------------------------------------------------------------
    // Cost estimation (library-level)
    // ---------------------------------------------------------------

    #[test]
    fn test_cost_estimate_all_providers() {
        let spec = make_valid_workload();
        let estimates = orchestr8::cost::estimate_all_providers(&spec).unwrap();
        assert_eq!(estimates.len(), 5); // AWS, Azure, GCP, DigitalOcean, Linode
        for est in &estimates {
            assert!(est.total_monthly > 0.0);
        }
    }

    // ---------------------------------------------------------------
    // Policy evaluation (library-level)
    // ---------------------------------------------------------------

    #[test]
    fn test_policy_production_evaluates() {
        let engine = orchestr8::policy::PolicyEngine::production();
        let spec = make_valid_workload();
        let result = engine.evaluate(&spec);
        // Result should have at least some rules evaluated
        assert!(result.policies_evaluated > 0);
    }

    #[test]
    fn test_policy_development_evaluates() {
        let engine = orchestr8::policy::PolicyEngine::development();
        let spec = make_valid_workload();
        let result = engine.evaluate(&spec);
        assert!(result.policies_evaluated > 0);
    }

    // ---------------------------------------------------------------
    // Scoring engine (library-level)
    // ---------------------------------------------------------------

    #[test]
    fn test_scoring_engine_recommends_runtime() {
        let engine = orchestr8::ai::scoring::ScoringEngine::with_defaults();
        let spec = make_valid_workload();
        let result = engine.score(&spec);
        // Should recommend some runtime
        assert!(!result.scores.is_empty());
        assert!(result.confidence > 0.0);
    }

    // ---------------------------------------------------------------
    // Profiler (library-level)
    // ---------------------------------------------------------------

    #[test]
    fn test_profiler_profiles_workload() {
        let profiler = orchestr8::ai::profiler::Profiler::with_defaults();
        let spec = make_valid_workload();
        let profile = profiler.profile(&spec, None);
        assert_eq!(profile.name, "test-app");
        assert!(profile.optimization_score >= 0.0 && profile.optimization_score <= 100.0);
    }

    #[test]
    fn test_profiler_with_runtime() {
        let profiler = orchestr8::ai::profiler::Profiler::with_defaults();
        let spec = make_valid_workload();
        let profile = profiler.profile(&spec, Some(RuntimeKind::Podman));
        assert_eq!(profile.name, "test-app");
    }

    // ---------------------------------------------------------------
    // Scaling engine (library-level)
    // ---------------------------------------------------------------

    #[test]
    fn test_scaling_engine_recommend() {
        use orchestr8::ai::scaling::{ScalingEngine, TimeSeries};

        let engine = ScalingEngine::with_defaults();

        let mut cpu = TimeSeries::new("cpu", "ratio");
        let mut mem = TimeSeries::new("mem", "ratio");

        let base = 1_700_000_000.0;
        for i in 0..60 {
            let t = base + (i as f64 * 60.0);
            cpu.add(t, 0.3 + (i as f64 * 0.005));
            mem.add(t, 0.4 + (i as f64 * 0.002));
        }

        let rec = engine.recommend(&cpu, &mem, 3, 1, 10, 0.05);
        assert!(rec.current_replicas == 3);
        assert!(rec.recommended_replicas >= 1);
        assert!(rec.confidence >= 0.0);
    }

    // ---------------------------------------------------------------
    // Diff report formatting
    // ---------------------------------------------------------------

    #[test]
    fn test_diff_report_formatting() {
        use orchestr8::drift::{format_live_diff, DiffRow, LiveDiffReport};

        let report = LiveDiffReport {
            workload_name: "test-app".to_string(),
            rows: vec![
                DiffRow {
                    field: "runtime".to_string(),
                    spec_value: "Auto".to_string(),
                    stored_value: "podman".to_string(),
                    live_value: "podman".to_string(),
                    matches: false,
                },
                DiffRow {
                    field: "state".to_string(),
                    spec_value: "running".to_string(),
                    stored_value: "-".to_string(),
                    live_value: "running".to_string(),
                    matches: true,
                },
            ],
            has_differences: true,
        };

        let output = format_live_diff(&report);
        assert!(output.contains("test-app"));
        assert!(output.contains("runtime"));
        assert!(output.contains("1 difference(s) found"));
    }

    // ---------------------------------------------------------------
    // Webhook command (event bus level)
    // ---------------------------------------------------------------

    #[test]
    fn test_webhook_channel_add_remove_list() {
        use orchestr8::events::{
            ChannelType, EventBus, EventSeverity, NotificationChannel,
        };

        let mut bus = EventBus::new();
        let initial = bus.channels().len();

        // Add a webhook channel
        bus.add_channel(NotificationChannel {
            name: "slack-ops".to_string(),
            channel_type: ChannelType::Webhook {
                url: "https://hooks.slack.com/services/test".to_string(),
                method: "POST".to_string(),
            },
            enabled: true,
            min_severity: EventSeverity::Warning,
            categories: vec![],
        });

        assert_eq!(bus.channels().len(), initial + 1);

        // Verify channel properties
        let ch = bus.channels().iter().find(|c| c.name == "slack-ops").unwrap();
        assert!(ch.enabled);
        assert_eq!(ch.min_severity, EventSeverity::Warning);

        // Remove channel
        assert!(bus.remove_channel("slack-ops"));
        assert_eq!(bus.channels().len(), initial);

        // Remove non-existent channel
        assert!(!bus.remove_channel("nonexistent"));
    }
}
