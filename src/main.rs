//! Orchestr8 CLI entrypoint

mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use orchestr8::state::StateStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cli::{Cli, Commands};

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
        Commands::Drift { .. } => "drift",
        Commands::PolicyCheck { .. } => "policy-check",
        Commands::Deps { .. } => "deps",
        Commands::Audit { .. } => "audit",
        Commands::Template { .. } => "template",
        Commands::Sla { .. } => "sla",
        Commands::Secrets { .. } => "secrets",
        Commands::Events { .. } => "events",
        Commands::Env { .. } => "env",
        Commands::Schedule { .. } => "schedule",
        Commands::Orchestrate { .. } => "orchestrate",
        Commands::Affinity { .. } => "affinity",
        Commands::Webhook { .. } => "webhook",
        Commands::Diff { .. } => "diff",
        Commands::Rollback { .. } => "rollback",
    };

    let result = match cli.command {
        Commands::Validate => commands::validate_command(&cli.spec).await,
        Commands::Build => commands::build_command(&cli.spec).await,
        Commands::Run { runtime } => commands::run_command(&cli.spec, runtime).await,
        Commands::Stop { name } => commands::stop_command(&name).await,
        Commands::Status { name } => commands::status_command(&name).await,
        Commands::Logs { name, follow } => commands::logs_command(&name, follow).await,
        Commands::Delete { name } => commands::delete_command(&name).await,
        Commands::List => commands::list_command().await,
        Commands::Migrate {
            name,
            target,
            strategy,
            no_validation,
            no_rollback,
        } => commands::migrate_command(&name, &target, &strategy, no_validation, no_rollback).await,
        Commands::Tui => commands::tui_command().await,
        Commands::Completions { shell } => {
            commands::completions_command(&shell)
        }
        Commands::Metrics => {
            commands::metrics_command().await;
            Ok(())
        }
        Commands::Backup { name, description } => {
            commands::backup_command(name.clone(), description.clone()).await
        }
        Commands::Restore { backup, merge } => {
            commands::restore_command(&backup, merge).await
        }
        Commands::ListBackups => {
            commands::list_backups_command().await
        }
        Commands::Cost { provider } => {
            commands::cost_command(&cli.spec, &provider).await
        }
        Commands::Serve { host, port } => {
            commands::serve_command(host, port).await
        }
        Commands::Recommend { runtime } => {
            commands::recommend_command(&cli.spec, runtime).await
        }
        Commands::Profile { name } => {
            commands::profile_command(&cli.spec, name).await
        }
        Commands::AnalyzeLogs { name } => {
            commands::analyze_logs_command(&name).await
        }
        Commands::MigrationAdvice { name, target } => {
            commands::migration_advice_command(&name, &target).await
        }
        Commands::ScalingAdvice => {
            commands::scaling_advice_command().await
        }
        Commands::Config { show, init } => {
            commands::config_command(show, init).await
        }
        Commands::Drift { name, reconcile } => {
            commands::drift_command(&name, reconcile).await
        }
        Commands::PolicyCheck { policy } => {
            commands::policy_check_command(&cli.spec, &policy).await
        }
        Commands::Deps { action } => {
            commands::deps_command(action).await
        }
        Commands::Audit { last, workload, summary } => {
            commands::audit_command(last, workload, summary).await
        }
        Commands::Template { name, workload_name, owner, project, registry, output, list } => {
            commands::template_command(&name, workload_name, &owner, &project, &registry, output, list).await
        }
        Commands::Sla { action } => {
            commands::sla_command(action).await
        }
        Commands::Secrets { action } => {
            commands::secrets_command(action).await
        }
        Commands::Events { last, severity, summary } => {
            commands::events_command(last, severity, summary).await
        }
        Commands::Env { action } => {
            commands::env_command(action).await
        }
        Commands::Schedule { action } => {
            commands::schedule_command(action).await
        }
        Commands::Orchestrate { action } => {
            commands::orchestrate_command(action).await
        }
        Commands::Affinity { action } => {
            commands::affinity_command(action).await
        }
        Commands::Webhook { action } => {
            commands::webhook_command(action).await
        }
        Commands::Diff { name } => {
            commands::diff_command(&name).await
        }
        Commands::Rollback { name } => {
            commands::rollback_command(&name).await
        }
    };

    // Record command execution time
    let duration = start.elapsed().as_secs_f64();
    orchestr8::metrics::record_command(command_name, duration);

    result
}
