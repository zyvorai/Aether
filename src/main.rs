//! Aether CLI entrypoint

mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use aether::state::StateStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing (suppress default fmt for non-verbose to keep output clean)
    let filter = if cli.verbose {
        tracing_subscriber::EnvFilter::new("debug")
    } else {
        tracing_subscriber::EnvFilter::new("warn")
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Set output modes
    aether::output::set_quiet(cli.quiet);
    let json_mode = cli.json || matches!(cli.output, cli::OutputFormat::Json);
    aether::output::set_json(json_mode);
    aether::output::set_yaml(matches!(cli.output, cli::OutputFormat::Yaml));
    aether::output::set_wide(matches!(cli.output, cli::OutputFormat::Wide));
    aether::output::set_yes(cli.yes);
    commands::set_skip_policy(cli.skip_policy);
    commands::set_namespace(cli.namespace.clone());

    // Initialize metrics
    aether::metrics::init();

    // Ensure state directory exists
    StateStore::ensure_state_dir()?;

    // Record command start time
    let start = std::time::Instant::now();
    let command_name = cli.command.name();

    let result = match cli.command {
        Commands::Validate => commands::validate_command(&cli.spec).await,
        Commands::Build => commands::build_command(&cli.spec).await,
        Commands::Run { runtime } => {
            if cli.dry_run {
                commands::dry_run_command(&cli.spec, runtime).await
            } else {
                commands::run_command(&cli.spec, runtime).await
            }
        }
        Commands::Stop { name } => {
            if cli.dry_run {
                aether::output::info(&format!("[dry-run] Would stop workload '{}'", name));
                return Ok(());
            }
            commands::stop_command(&name).await
        }
        Commands::Status { name } => commands::status_command(&name).await,
        Commands::Logs { name, follow } => commands::logs_command(&name, follow).await,
        Commands::Delete { name } => {
            if cli.dry_run {
                aether::output::info(&format!("[dry-run] Would delete workload '{}'", name));
                return Ok(());
            }
            commands::delete_command(&name).await
        }
        Commands::List => commands::list_command().await,
        Commands::Migrate {
            name,
            target,
            strategy,
            no_validation,
            no_rollback,
        } => {
            if cli.dry_run {
                aether::output::info(&format!(
                    "[dry-run] Would migrate '{}' to {} using {} strategy",
                    name, target, strategy
                ));
                return Ok(());
            }
            commands::migrate_command(&name, &target, &strategy, no_validation, no_rollback).await
        }
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
        Commands::Serve { host, port, tls_cert, tls_key } => {
            commands::serve_command(host, port, tls_cert, tls_key).await
        }
        Commands::Recommend { .. } => {
            commands::recommend_command(&cli.spec).await
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
        Commands::HelpAll => {
            aether::completions::show_help();
            Ok(())
        }
        Commands::Diff { name } => {
            commands::diff_command(&name).await
        }
        Commands::Rollback { name } => {
            commands::rollback_command(&name).await
        }
        Commands::Deploy { dir, runtime, fail_fast, dry_run } => {
            commands::deploy_command(&dir, runtime, fail_fast, dry_run).await
        }
        Commands::Exec { name, command, interactive, timeout } => {
            commands::exec_command(&name, &command, interactive, timeout).await
        }
        Commands::PortForward { name, ports, timeout } => {
            commands::port_forward_command(&name, &ports, timeout).await
        }
        Commands::Watch { runtime } => {
            commands::watch_command(&cli.spec, runtime).await
        }
        Commands::Compare => {
            commands::compare_command(&cli.spec).await
        }
        Commands::Init => {
            commands::init_command().await
        }
        Commands::Compose { action } => {
            commands::compose_command(action).await
        }
        Commands::Plugin { action } => {
            commands::plugin_command(action).await
        }
        Commands::Health { name, last, summary } => {
            commands::health_command(&name, last, summary).await
        }
    };

    // Record command execution time
    let duration = start.elapsed().as_secs_f64();
    aether::metrics::record_command(command_name, duration);

    // Show elapsed time for commands that take noticeable time
    if duration >= 0.1 && !cli.quiet && !cli.json {
        let elapsed = if duration < 1.0 {
            format!("{:.0}ms", duration * 1000.0)
        } else if duration < 60.0 {
            format!("{:.1}s", duration)
        } else {
            let mins = (duration / 60.0).floor() as u64;
            let secs = duration % 60.0;
            format!("{}m {:.0}s", mins, secs)
        };
        eprintln!(
            "\n{}",
            format!("  ⏱ {} | {}", elapsed, command_name)
                .truecolor(
                    aether::output::COLOR_MUTED.0,
                    aether::output::COLOR_MUTED.1,
                    aether::output::COLOR_MUTED.2,
                )
        );
    }

    // Styled error display — print our styled version, then return a
    // minimal error so the runtime doesn't re-print the full message.
    // Add contextual help suggestions to errors.
    let result = result.map_err(commands::suggest_on_error);
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            if cli.json {
                eprintln!(
                    r#"{{"status":"error","command":"{}","message":{}}}"#,
                    command_name,
                    serde_json::to_string(&e.to_string()).unwrap_or_else(|_| format!("\"{}\"", e))
                );
            } else if !cli.quiet {
                eprintln!(
                    "\n{} {} {}\n",
                    "Error:"
                        .truecolor(
                            aether::output::COLOR_ERROR.0,
                            aether::output::COLOR_ERROR.1,
                            aether::output::COLOR_ERROR.2,
                        )
                        .bold(),
                    e,
                    format!("({})", command_name).truecolor(
                        aether::output::COLOR_MUTED.0,
                        aether::output::COLOR_MUTED.1,
                        aether::output::COLOR_MUTED.2,
                    ),
                );
            }
            // Return a minimal error so the runtime doesn't re-print the full message
            Err(anyhow::anyhow!(""))
        }
    }
}
