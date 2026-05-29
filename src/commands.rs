// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Command handler implementations for Aether CLI

use anyhow::{Context, Result};
use aether::{
    engine::Engine,
    output,
    runtime::RuntimeKind,
    spec::Workload,
    state::StateStore,
};
use std::path::{Path, PathBuf};

use crate::cli::*;

/// CLI command context — holds session-scoped settings that were previously
/// global statics.  Initialized once per CLI invocation in `main()` via
/// `set_skip_policy()` / `set_namespace()`.  These are deliberately NOT
/// passed through every function signature because the CLI is single-threaded
/// (one command per process), and threading a context through 50+ handlers
/// adds complexity without benefit.  The API server has its own request-scoped
/// state in `AppState`.
mod ctx {
    use std::sync::atomic::{AtomicBool, Ordering};

    static SKIP_POLICY: AtomicBool = AtomicBool::new(false);
    static NAMESPACE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

    pub(crate) fn set_skip_policy(enabled: bool) {
        SKIP_POLICY.store(enabled, Ordering::Relaxed);
    }

    pub(crate) fn skip_policy() -> bool {
        SKIP_POLICY.load(Ordering::Relaxed)
    }

    pub(crate) fn set_namespace(ns: Option<String>) {
        let _ = NAMESPACE.set(ns);
    }

    pub(crate) fn namespace() -> Option<&'static str> {
        NAMESPACE.get().and_then(|n| n.as_deref())
    }
}

pub(crate) fn set_skip_policy(enabled: bool) {
    ctx::set_skip_policy(enabled);
}

pub(crate) fn set_namespace(ns: Option<String>) {
    ctx::set_namespace(ns);
}

fn get_namespace() -> Option<&'static str> {
    ctx::namespace()
}

/// Load the state store and look up a workload by name, returning a borrowed
/// reference tied to the returned store. Avoids repeating the 3-line
/// `StateStore::load` + `state.get` + `ok_or_else` pattern everywhere.
fn load_workload_state(name: &str) -> Result<(StateStore, String)> {
    let state = StateStore::load(&StateStore::default_path())?;
    if state.get(name).is_none() {
        anyhow::bail!("Workload '{}' not found.\nHint: Run `aether list` to see deployed workloads.", name);
    }
    Ok((state, name.to_string()))
}

/// Convenience: load state and get a cloned WorkloadState, avoiding the
/// `state.get(name).unwrap()` pattern after `load_workload_state`.
fn get_workload_state(state: &StateStore, name: &str) -> Result<aether::state::WorkloadState> {
    state.get(name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' disappeared from state", name))
}

/// Load a workload's state and create a runtime client in one step.
/// Used by commands that need to interact with the actual runtime
/// (stop, status, logs, delete, analyze-logs, diff).
async fn load_state_and_runtime(
    name: &str,
) -> Result<(
    StateStore,
    aether::state::WorkloadState,
    Box<dyn aether::Runtime>,
)> {
    let state = StateStore::load(&StateStore::default_path())?;
    let ws = state
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", name))?
        .clone();
    let rt = aether::runtime::create_runtime_ns(&ws.runtime, get_namespace()).await?;
    Ok((state, ws, rt))
}

pub(crate) async fn validate_command(spec_path: &PathBuf) -> Result<()> {
    let sp = output::spinner("Validating workload specification...");

    let workload = match Workload::from_file(spec_path) {
        Ok(w) => {
            output::spinner_success(&sp, "Workload specification is valid");
            w
        }
        Err(e) => {
            output::spinner_fail(&sp, "Validation failed");
            return Err(e);
        }
    };

    output::success(&format!("Workload '{}' is valid", workload.metadata.name));

    println!(
        "{}",
        output::property_table(&[
            ("Name", workload.metadata.name.clone()),
            ("Owner", workload.metadata.owner.clone()),
            ("Project", workload.metadata.project.clone()),
            ("CPU", workload.requirements.cpu.clone()),
            ("Memory", workload.requirements.memory.clone()),
            ("Storage", workload.requirements.storage.clone()),
            (
                "Preferred Runtime",
                format!("{:?}", workload.runtime.preferred),
            ),
        ])
    );

    Ok(())
}

pub(crate) async fn build_command(spec_path: &PathBuf) -> Result<()> {
    let workload = Workload::from_file(spec_path)?;
    let engine = Engine::new();
    let runtime_kind = engine.decide(&workload)?;

    output::info(&format!(
        "Selected runtime: {}",
        output::runtime_display(&runtime_kind)
    ));

    // Create runtime before spinner so failure doesn't leave orphan spinner
    let rt = aether::runtime::create_runtime_ns(&runtime_kind, get_namespace()).await?;

    let sp = output::spinner("Building workload...");
    let result = rt.build(&workload).await;

    match result {
        Ok(image) => {
            output::spinner_success(&sp, "Build completed");
            output::success(&format!("Built image: {}", image.full_name()));
            aether::metrics::record_build(&runtime_kind.to_string(), true);
            emit_event(
                aether::events::EventSeverity::Info,
                aether::events::EventCategory::Deployment,
                "cli",
                Some(&workload.metadata.name),
                "Build completed",
                &format!("Built image on {}", runtime_kind),
            );
            Ok(())
        }
        Err(e) => {
            output::spinner_fail(&sp, "Build failed");
            output::error(&format!("Build failed: {}", e));
            aether::metrics::record_build(&runtime_kind.to_string(), false);
            emit_event(
                aether::events::EventSeverity::Error,
                aether::events::EventCategory::Deployment,
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
    let workload = Workload::from_file(spec_path)?;
    deploy_workload_interactive(&workload, spec_path, runtime_override.as_deref()).await
}

/// Deploy a single workload: decide runtime, build, run, persist state, record metrics.
/// Extracted from run_command so deploy_command can reuse it per-workload.
/// When `event_batch` is provided, events accumulate in memory instead of hitting disk.
/// When `interactive` is true, shows the runtime selector menu for manual choice.
async fn deploy_single_workload(
    workload: &Workload,
    spec_path: &Path,
    runtime_override: Option<&str>,
    event_batch: Option<&mut EventBatch>,
) -> Result<()> {
    deploy_workload_inner(workload, spec_path, runtime_override, event_batch, false).await
}

async fn deploy_workload_interactive(
    workload: &Workload,
    spec_path: &Path,
    runtime_override: Option<&str>,
) -> Result<()> {
    deploy_workload_inner(workload, spec_path, runtime_override, None, true).await
}

async fn deploy_workload_inner(
    workload: &Workload,
    spec_path: &Path,
    runtime_override: Option<&str>,
    event_batch: Option<&mut EventBatch>,
    interactive: bool,
) -> Result<()> {
    // Policy gate: evaluate workload against configured policies
    let config = aether::config::Config::load();
    if config.policy.enforce_on_deploy && !ctx::skip_policy() {
        aether::policy::gate_deploy(workload, &config.policy)?;
    }

    let engine = Engine::new();

    // Determine runtime — use ScoringEngine when intent is present
    let runtime_kind = if let Some(override_str) = runtime_override {
        override_str.parse::<RuntimeKind>()?
    } else if let Some(intent) = &workload.intent {
        // Intent-driven: use scoring engine for intent-aware selection
        let scoring = aether::ai::scoring::ScoringEngine::new(config.engine.clone());
        let result = scoring.score(workload);
        output::info(&format!(
            "Intent goal: {:?} — recommended: {} (confidence: {:.0}%)",
            intent.goal,
            result.recommended,
            result.confidence * 100.0
        ));
        result.recommended
    } else if interactive {
        // Interactive selection: show menu
        let ai_recommended = engine.decide(workload)?;
        let selected = output::select_runtime(&[
            ("🐳", "Podman", "Local container — fast, simple, single-host"),
            ("☸️", "Kubernetes", "Cluster orchestration — HA, services, scaling"),
            ("🖥️", "KubeVirt", "Virtual machines — GPU passthrough, isolation"),
            ("🖧", "Metal3", "Bare metal — maximum performance, BMC provisioning"),
        ]);
        match selected {
            Some(0) => RuntimeKind::Podman,
            Some(1) => RuntimeKind::Kubernetes,
            Some(2) => RuntimeKind::KubeVirt,
            Some(3) => RuntimeKind::Metal3,
            _ => ai_recommended,
        }
    } else {
        engine.decide(workload)?
    };

    output::info(&format!(
        "Selected runtime: {}",
        output::runtime_display(&runtime_kind)
    ));

    // Build and run based on runtime
    let rt = aether::runtime::create_runtime_ns(&runtime_kind, get_namespace()).await?;
    let sp = output::spinner("Building and deploying workload...");
    let image = rt.build(workload).await?;

    sp.set_message(format!("Image ready: {}. Starting instance...", image.full_name()));

    let instance = rt.run(&image, workload).await?;
    output::spinner_success(&sp, "Deployment completed");

    output::success(&format!(
        "Started instance: {} ({})",
        instance.name, instance.id
    ));

    // Capture instance name before move
    let instance_name = instance.name.clone();

    // Save state (auto-snapshot existing workload if present)
    let mut state = StateStore::load(&StateStore::default_path())?;
    if let Some(existing) = state.get(&workload.metadata.name) {
        let snap_mgr = aether::backup::SnapshotManager::new();
        if let Err(e) = snap_mgr.create_snapshot(existing) {
            tracing::warn!("Failed to create pre-deploy snapshot: {}", e);
        }
    }
    state.upsert(
        workload.metadata.name.clone(),
        aether::state::WorkloadState::new(
            workload.metadata.name.clone(),
            runtime_kind,
            instance,
            spec_path.to_path_buf(),
        ),
    );
    state.save(&StateStore::default_path())?;

    // Record metrics
    aether::metrics::record_deployment(&runtime_kind.to_string(), true);

    // Emit event — use batch if provided, otherwise single disk round-trip
    let sev = aether::events::EventSeverity::Info;
    let cat = aether::events::EventCategory::Deployment;
    let msg = format!("Deployed on {} (instance: {})", runtime_kind, instance_name);
    if let Some(batch) = event_batch {
        batch.emit(sev, cat, "cli", Some(&workload.metadata.name), "Workload deployed", &msg);
    } else {
        emit_event(sev, cat, "cli", Some(&workload.metadata.name), "Workload deployed", &msg);
    }

    let _ = aether::intelligence::record::record_deployment_outcome(
        workload,
        runtime_kind,
        true,
        None,
    );

    Ok(())
}

pub(crate) async fn stop_command(name: &str, cascade: bool) -> Result<()> {
    let (_state, ws, rt) = load_state_and_runtime(name).await?;

    let sp = output::spinner(&format!("Stopping workload '{}'...", name));
    if cascade && ws.runtime == aether::runtime::RuntimeKind::Kubernetes {
        let ns = get_namespace()
            .map(|s| s.to_string())
            .unwrap_or_else(|| std::env::var("AETHER_NAMESPACE").unwrap_or_else(|_| "default".to_string()));
        let kube = aether::adapters::KubernetesRuntime::with_namespace(ns).await?;
        kube.stop_cascade(&ws.instance).await?;
    } else {
        if cascade && ws.runtime != aether::runtime::RuntimeKind::Kubernetes {
            output::warning("--cascade is only supported for Kubernetes workloads; stopping controller only");
        }
        rt.stop(&ws.instance).await?;
    }

    output::spinner_success(&sp, &format!("Stopped '{}'", name));
    output::success(&format!("Stopped instance: {}", name));

    emit_event(
        aether::events::EventSeverity::Info,
        aether::events::EventCategory::Deployment,
        "cli",
        Some(name),
        "Workload stopped",
        &format!("Stopped instance {}", name),
    );

    Ok(())
}

pub(crate) async fn status_command(name: &str) -> Result<()> {
    let (_state, ws, rt) = load_state_and_runtime(name).await?;

    let sp = output::spinner(&format!("Fetching status for '{}'...", name));
    let status = rt.status(&ws.instance).await?;

    output::spinner_success(&sp, "Status retrieved");

    // Record health check (side-effect: status queries feed the health timeline)
    if !output::is_quiet() {
        let health_path = aether::health::HealthHistory::default_path();
        let mut history = aether::health::HealthHistory::load(&health_path)
            .unwrap_or_default();
        history.record(aether::health::HealthRecord {
            timestamp: aether::resources::now_rfc3339(),
            workload: name.to_string(),
            runtime: ws.runtime,
            state: status.state.clone(),
            ready: status.ready,
            restart_count: status.restart_count,
            latency_ms: None,
        });
        let _ = history.save(&health_path);
    }

    // JSON/YAML output modes
    if output::is_json() {
        let val = serde_json::json!({
            "workload": name,
            "runtime": ws.runtime.to_string(),
            "state": format!("{}", status.state),
            "ready": status.ready,
            "restart_count": status.restart_count,
            "message": status.message,
            "instance_id": ws.instance.id,
            "image": ws.instance.image,
        });
        println!("{}", serde_json::to_string_pretty(&val)?);
        return Ok(());
    }
    if output::is_yaml() {
        let val = serde_json::json!({
            "workload": name,
            "runtime": ws.runtime.to_string(),
            "state": format!("{}", status.state),
            "ready": status.ready,
            "restart_count": status.restart_count,
        });
        println!("{}", serde_yaml::to_string(&val)?);
        return Ok(());
    }

    output::section_with_icon("📊", &format!("Status for '{}'", name));

    let ready_str = if status.ready {
        "● Yes".to_string()
    } else {
        "○ No".to_string()
    };

    let mut pairs: Vec<(&str, String)> = vec![
        ("Runtime", output::runtime_display(&ws.runtime)),
        ("State", format!("{}", status.state)),
        ("Ready", ready_str),
    ];

    if let Some(ref msg) = status.message {
        pairs.push(("Message", msg.clone()));
    }
    if status.restart_count > 0 {
        pairs.push(("Restarts", format!("{}", status.restart_count)));
    }

    // Show health history summary if available
    {
        let health_path = aether::health::HealthHistory::default_path();
        if let Ok(history) = aether::health::HealthHistory::load(&health_path) {
            let summary = history.summary(name);
            if summary.total_checks > 1 {
                pairs.push(("Uptime", format!("{:.1}% ({}/{} checks)",
                    summary.uptime_percent, summary.ready_checks, summary.total_checks)));
            }
        }
    }

    println!("{}", output::property_table(&pairs));

    Ok(())
}

pub(crate) async fn logs_command(name: &str, follow: bool) -> Result<()> {
    let (_state, ws, rt) = load_state_and_runtime(name).await?;

    let sp = output::spinner(&format!("Fetching logs for '{}'...", name));
    let logs = rt.logs(&ws.instance, follow).await?;
    output::spinner_success(&sp, &format!("Logs for '{}' ({})", name, ws.runtime));

    // Color-code each log line by level
    for line in logs.lines() {
        println!("{}", output::colorize_log_line(line));
    }

    Ok(())
}

pub(crate) async fn delete_command(name: &str) -> Result<()> {
    // Confirm destructive action
    if !output::confirm(&format!("Delete workload '{}'?", name)) {
        output::muted("Cancelled.");
        return Ok(());
    }

    let (mut state, ws, rt) = load_state_and_runtime(name).await?;

    let sp = output::spinner(&format!("Deleting workload '{}'...", name));
    rt.delete(&ws.instance).await?;

    // Remove from state
    state.remove(name);
    state.save(&StateStore::default_path())?;

    // Cascade: clean up subsystem stores
    cascade_delete(name);

    // Record metrics
    aether::metrics::record_deletion(&ws.runtime.to_string());

    emit_event(
        aether::events::EventSeverity::Warning,
        aether::events::EventCategory::Deployment,
        "cli",
        Some(name),
        "Workload deleted",
        &format!("Deleted instance from {}", ws.runtime),
    );

    output::spinner_success(&sp, &format!("Deleted '{}'", name));
    output::success(&format!("Deleted instance: {}", name));

    Ok(())
}

/// Clean up orphaned data across all subsystem stores when a workload is deleted.
/// Best-effort: errors are logged but do not block the deletion.
fn cascade_delete(name: &str) {
    // 1. Remove from orchestrator
    let orch_path = aether::orchestrator::Orchestrator::default_path();
    if let Ok(mut orch) = aether::orchestrator::Orchestrator::load(&orch_path) {
        if orch.unregister(name).is_some() {
            let _ = orch.save(&orch_path);
            tracing::debug!("Cascade: removed '{}' from orchestrator", name);
        }
    }

    // 2. Remove from dependency graph
    let deps_path = aether::dependencies::DependencyGraph::default_path();
    if let Ok(mut deps) = aether::dependencies::DependencyGraph::load(&deps_path) {
        deps.remove_workload(name);
        let _ = deps.save(&deps_path);
        tracing::debug!("Cascade: removed '{}' from dependency graph", name);
    }

    // 3. Remove from scheduler placements
    let sched_path = aether::scheduler::Scheduler::default_path();
    if let Ok(mut sched) = aether::scheduler::Scheduler::load(&sched_path) {
        sched.release(name);
        let _ = sched.save(&sched_path);
        tracing::debug!("Cascade: released '{}' from scheduler", name);
    }

    // 4. Prune health records for this workload
    let health_path = aether::health::HealthHistory::default_path();
    if let Ok(mut history) = aether::health::HealthHistory::load(&health_path) {
        let before = history.records.len();
        history.records.retain(|r| r.workload != name);
        if history.records.len() < before {
            let _ = history.save(&health_path);
            tracing::debug!("Cascade: pruned {} health records for '{}'", before - history.records.len(), name);
        }
    }
}

pub(crate) async fn update_command(name: &str, spec_path: &PathBuf) -> Result<()> {
    let (mut state, ws, rt) = load_state_and_runtime(name).await?;
    let workload = Workload::from_file(spec_path)?;

    output::section_with_icon("🔄", "Updating Workload");
    let sp = output::spinner("Updating workload...");

    let image = aether::runtime::Image {
        name: workload.metadata.name.clone(),
        tag: "latest".to_string(),
        digest: None,
        runtime: ws.runtime,
    };

    let new_instance = rt.update(&ws.instance, &image, &workload).await?;
    output::spinner_success(&sp, "Workload updated");

    state.upsert(name.to_string(), aether::state::WorkloadState::new(
        name.to_string(), ws.runtime, new_instance, spec_path.clone(),
    ));
    state.save(&StateStore::default_path())?;

    output::success(&format!("Workload '{}' updated successfully", name));
    Ok(())
}

pub(crate) async fn list_command() -> Result<()> {
    let state = StateStore::load(&StateStore::default_path())?;
    let workloads = state.list();

    if workloads.is_empty() {
        if output::is_json() {
            println!("[]");
        } else if output::is_yaml() {
            println!("workloads: []");
        } else {
            output::muted("No workloads running");
        }
        return Ok(());
    }

    // JSON output
    if output::is_json() {
        let list: Vec<serde_json::Value> = workloads.iter().map(|w| {
            serde_json::json!({
                "name": w.name,
                "runtime": w.runtime.to_string(),
                "instance_id": w.instance.id,
                "image": w.instance.image,
                "spec_path": w.spec_path.display().to_string(),
                "created_at": w.created_at,
                "updated_at": w.updated_at,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&list)?);
        return Ok(());
    }

    // YAML output
    if output::is_yaml() {
        let list: Vec<serde_json::Value> = workloads.iter().map(|w| {
            serde_json::json!({
                "name": w.name,
                "runtime": w.runtime.to_string(),
                "instance_id": w.instance.id,
                "image": w.instance.image,
                "created_at": w.created_at,
            })
        }).collect();
        println!("{}", serde_yaml::to_string(&list)?);
        return Ok(());
    }

    output::section_with_icon("📋", "Running Workloads");

    // Wide mode: show extra columns
    if output::is_wide() {
        let rows: Vec<Vec<String>> = workloads
            .iter()
            .map(|w| {
                vec![
                    w.name.clone(),
                    output::runtime_display(&w.runtime),
                    w.instance.id.chars().take(12).collect::<String>(),
                    w.instance.image.clone(),
                    w.spec_path.display().to_string(),
                    w.created_at.chars().take(19).collect::<String>(),
                ]
            })
            .collect();
        println!("{}", output::table(
            &["Name", "Runtime", "Instance ID", "Image", "Spec", "Created"],
            rows,
        ));
    } else {
        let rows: Vec<Vec<String>> = workloads
            .iter()
            .map(|w| {
                vec![
                    w.name.clone(),
                    output::runtime_display(&w.runtime),
                    w.instance.id.chars().take(12).collect::<String>(),
                ]
            })
            .collect();
        println!("{}", output::table(&["Name", "Runtime", "Instance ID"], rows));
    }

    Ok(())
}

pub(crate) async fn migrate_command(
    name: &str,
    target: &str,
    strategy_str: &str,
    no_validation: bool,
    no_rollback: bool,
    verbose_trace: bool,
) -> Result<()> {
    use aether::migration::{MigrationEngine, MigrationPlan, MigrationStrategy};
    use std::time::Duration;

    // Load current state
    let (state, _) = load_workload_state(name)?;
    let workload_state = get_workload_state(&state, name)?;

    let source_runtime = workload_state.runtime;

    // Parse target runtime
    let target_runtime: RuntimeKind = target.parse()?;

    // Parse strategy
    let strategy: MigrationStrategy = strategy_str.parse()?;

    // Auto-snapshot before migration
    {
        let snap_mgr = aether::backup::SnapshotManager::new();
        if let Err(e) = snap_mgr.create_snapshot(&workload_state) {
            tracing::warn!("Failed to create pre-migrate snapshot: {}", e);
        }
    }

    output::section_with_icon("🔄", &format!("Migrating workload '{}'", name));

    println!(
        "{}",
        output::property_table(&[
            ("Workload", name.to_string()),
            ("Source", output::runtime_display(&source_runtime)),
            ("Target", output::runtime_display(&target_runtime)),
            ("Strategy", format!("{:?}", strategy)),
        ])
    );

    // Create migration plan
    let mut plan = MigrationPlan::new(
        name.to_string(),
        source_runtime,
        target_runtime,
        strategy,
        !no_rollback,
    );
    if no_validation {
        plan.validation_delay = Duration::from_secs(5);
    } else {
        plan.validation_delay = Duration::from_secs(30);
    }
    plan.verbose_trace = verbose_trace
        || std::env::var("AETHER_MIGRATION_TRACE")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    // Execute migration with progress indicators
    let total_steps = match strategy_str {
        "rolling" => 4,
        "blue-green" => 3,
        _ => 2,
    };
    output::step(1, total_steps, "Preparing migration");
    let migration_start = std::time::Instant::now();
    let engine = MigrationEngine::new(StateStore::default_path());
    let sp = output::spinner("Executing migration...");
    let result = engine.migrate(plan).await?;
    let migration_duration = migration_start.elapsed().as_secs_f64();

    // Record metrics
    aether::metrics::record_migration(
        &source_runtime.to_string(),
        &target_runtime.to_string(),
        strategy_str,
        migration_duration,
        result.success,
        result.rollback_performed,
    );

    if result.success {
        output::spinner_success(&sp, "Migration completed");

        let duration_str = if migration_duration < 1.0 {
            format!("{:.0}ms", migration_duration * 1000.0)
        } else {
            format!("{:.1}s", migration_duration)
        };

        let mut summary_items: Vec<(&str, String)> = vec![
            ("Workload", name.to_string()),
            ("Source", format!("{}", source_runtime)),
            ("Target", format!("{}", target_runtime)),
            ("Strategy", strategy_str.to_string()),
            ("Duration", duration_str),
        ];
        if let Some(ref instance) = result.target_instance {
            summary_items.push(("Instance", format!("{} ({})", instance.name, instance.id)));
        }
        output::summary_success("Migration Successful", &summary_items);

        emit_event(
            aether::events::EventSeverity::Info,
            aether::events::EventCategory::Migration,
            "cli",
            Some(name),
            "Migration completed",
            &format!(
                "Migrated from {} to {} using {}",
                source_runtime, target_runtime, strategy_str
            ),
        );
    } else {
        output::spinner_fail(&sp, "Migration failed");

        let mut error_items: Vec<(&str, String)> = vec![
            ("Workload", name.to_string()),
            ("Source", format!("{}", source_runtime)),
            ("Target", format!("{}", target_runtime)),
        ];
        if let Some(ref error) = result.error {
            error_items.push(("Error", error.clone()));
        }
        if result.rollback_performed {
            error_items.push(("Rollback", "Performed successfully".to_string()));
            if let Some(ref instance) = result.source_instance {
                error_items.push(("Restored", format!("{} ({})", instance.name, instance.id)));
            }
        }
        output::summary_error("Migration Failed", &error_items);

        emit_event(
            aether::events::EventSeverity::Error,
            aether::events::EventCategory::Migration,
            "cli",
            Some(name),
            "Migration failed",
            &format!(
                "Failed migrating from {} to {}",
                source_runtime, target_runtime
            ),
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
    use aether::ui::App;
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
    app: &mut aether::ui::App,
) -> Result<()> {
    use aether::ui::{render_dashboard, render_logs, Screen};

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
        aether::ui::handle_events(app).await?;

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
            aether::completions::list_shells();
            anyhow::bail!("Unknown shell: '{}'. See list above.", shell_str);
        }
    };

    let mut cmd = Cli::command();
    aether::completions::generate_completions(shell, &mut cmd);
    Ok(())
}

pub(crate) async fn metrics_command() {
    output::section_with_icon("📊", "Aether Metrics");
    output::muted(&format!("Updated: {}", aether::resources::now_rfc3339()));
    println!();

    // Update workload state metrics from state store
    if let Ok(state) = StateStore::load(&StateStore::default_path()) {
        let workloads = state.list();
        let states: Vec<(String, String)> = workloads
            .iter()
            .map(|w| (w.runtime.to_string(), "running".to_string()))
            .collect();
        aether::metrics::update_workload_states(&states);

        // Update running workload count
        let mut runtime_counts = std::collections::HashMap::new();
        for w in workloads {
            *runtime_counts.entry(w.runtime.to_string()).or_insert(0) += 1;
        }
    }

    // Gather and print metrics
    print!("{}", aether::metrics::gather());
}

pub(crate) async fn backup_command(
    name: Option<String>,
    description: Option<String>,
) -> Result<()> {
    use aether::backup::BackupManager;

    let sp = output::spinner("Creating backup...");

    // Load current state
    let state = StateStore::load(&StateStore::default_path())?;
    let workload_count = state.list().len();

    if workload_count == 0 {
        output::spinner_fail(&sp, "No workloads to backup");
        output::warning("No workloads to backup");
        return Ok(());
    }

    // Create backup
    let manager = BackupManager::new(BackupManager::default_dir());
    let backup_path = manager.create_backup(&state, name.clone(), description.clone())?;

    output::spinner_success(&sp, "Backup created");
    output::success(&format!("Backup created: {}", backup_path.display()));
    output::kv_tree("Workloads", &workload_count.to_string(), description.is_none());
    if let Some(desc) = description {
        output::kv_tree("Description", &desc, true);
    }

    Ok(())
}

pub(crate) async fn restore_command(backup_path: &Path, merge: bool) -> Result<()> {
    use aether::backup::Backup;

    let sp = output::spinner("Loading backup...");

    // Load backup
    let backup = Backup::load(backup_path)?;

    output::spinner_success(&sp, "Backup loaded");

    output::section_with_icon("📦", "Backup Details");
    println!(
        "{}",
        output::property_table(&[
            ("Created", backup.metadata.created_at.clone()),
            ("Workloads", backup.metadata.workload_count.to_string()),
            ("Version", backup.metadata.aether_version.clone()),
            (
                "Description",
                backup
                    .metadata
                    .description
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
        ])
    );

    // Restore or merge
    let state_path = StateStore::default_path();

    if merge {
        let sp = output::spinner("Merging backup with existing state...");
        backup.merge(&state_path)?;
        output::spinner_success(&sp, "Merge completed");
        output::success("Backup merged successfully");
    } else {
        if !output::confirm("This will replace your current state. Continue?") {
            output::muted("Cancelled.");
            return Ok(());
        }

        let sp = output::spinner("Restoring backup...");
        backup.restore(&state_path)?;
        output::spinner_success(&sp, "Restore completed");
        output::success("Backup restored successfully");
    }

    Ok(())
}

pub(crate) async fn list_backups_command() -> Result<()> {
    use aether::backup::BackupManager;

    let manager = BackupManager::new(BackupManager::default_dir());
    let backups = manager.list_backups()?;

    if backups.is_empty() {
        output::muted("No backups found");
        output::detail(&format!(
            "Backup directory: {}",
            BackupManager::default_dir().display()
        ));
        return Ok(());
    }

    output::section_with_icon("📋", "Available Backups");

    for backup_path in backups {
        match manager.get_backup_info(&backup_path) {
            Ok(info) => {
                let fname = backup_path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| backup_path.display().to_string());

                println!();
                output::kv_tree("File", &fname, false);
                output::kv_tree("Created", &info.created_at, false);
                output::kv_tree("Workloads", &info.workload_count.to_string(), false);
                output::kv_tree("Version", &info.aether_version, info.description.is_none());
                if let Some(desc) = info.description {
                    output::kv_tree("Description", &desc, true);
                }
            }
            Err(e) => {
                output::warning(&format!("{} (error: {})", backup_path.display(), e));
            }
        }
    }

    println!();
    output::muted(&format!(
        "Backup directory: {}",
        BackupManager::default_dir().display()
    ));

    Ok(())
}

pub(crate) async fn cost_command(spec_path: &PathBuf, provider: &str) -> Result<()> {
    use aether::cost::{estimate_cost, CloudProvider, CostComparison};

    let workload = Workload::from_file(spec_path)?;

    if output::is_json() {
        if provider == "all" {
            let comparison = CostComparison::for_workload(&workload)?;
            println!("{}", serde_json::to_string_pretty(&comparison)?);
        } else {
            let cloud_provider: CloudProvider = provider.parse()?;
            let estimate = estimate_cost(&workload, cloud_provider)?;
            let val = serde_json::json!({
                "workload": workload.metadata.name,
                "resources": {
                    "cpu": workload.requirements.cpu,
                    "memory": workload.requirements.memory,
                    "storage": workload.requirements.storage,
                },
                "provider": provider,
                "estimate": estimate,
            });
            println!("{}", serde_json::to_string_pretty(&val)?);
        }
        return Ok(());
    }

    output::section_with_icon("💰", "Cost Estimation");

    if provider == "all" {
        // Show comparison across all providers
        let comparison = CostComparison::for_workload(&workload)?;
        print!("{}", comparison.display());
    } else {
        // Show estimate for specific provider
        let cloud_provider: CloudProvider = provider.parse()?;

        let estimate = estimate_cost(&workload, cloud_provider)?;

        println!(
            "{}",
            output::property_table(&[
                ("Workload", workload.metadata.name.clone()),
                (
                    "Resources",
                    format!(
                        "{} CPU | {} RAM | {} Storage",
                        workload.requirements.cpu,
                        workload.requirements.memory,
                        workload.requirements.storage
                    ),
                ),
            ])
        );
        println!("{}", estimate.display());
    }

    println!();
    output::boxed(
        "Note",
        "Estimates are based on baseline pricing and may vary based on:\n\
         - Region selection\n\
         - Reserved vs. on-demand instances\n\
         - Volume discounts\n\
         - Additional services (load balancers, networking, etc.)",
    );

    Ok(())
}

pub(crate) async fn serve_command(
    host: String,
    port: u16,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
) -> Result<()> {
    use aether::api::{start_server, ApiConfig};

    output::logo();
    output::banner("AETHER API SERVER", "Universal Runtime Control Plane");

    let tls_enabled = tls_cert.is_some() && tls_key.is_some();
    if tls_cert.is_some() != tls_key.is_some() {
        anyhow::bail!("Both --tls-cert and --tls-key must be provided together");
    }

    let config = ApiConfig {
        host,
        port,
        state_path: StateStore::default_path(),
        tls_cert,
        tls_key,
    };

    let scheme = if tls_enabled { "https" } else { "http" };
    output::kv("Dashboard URL", &format!("{}://{}:{}", scheme, config.host, config.port));
    if std::env::var("AETHER_STATE_DATABASE_URL")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        output::kv("Workload state", "PostgreSQL (AETHER_STATE_DATABASE_URL)");
    }

    output::endpoint_category("Workloads");
    output::endpoint("GET", "/health");
    output::endpoint("GET", "/api/workloads");
    output::endpoint("POST", "/api/workloads");
    output::endpoint("GET", "/api/workloads/:name");
    output::endpoint("DELETE", "/api/workloads/:name");
    output::endpoint("GET", "/api/workloads/:name/logs");
    output::endpoint("POST", "/api/workloads/:name/start");
    output::endpoint("POST", "/api/workloads/:name/stop");

    output::endpoint_category("Cost & Backups");
    output::endpoint("POST", "/api/cost");
    output::endpoint("GET", "/api/backups");
    output::endpoint("POST", "/api/backups");

    output::endpoint_category("AI Intelligence");
    output::endpoint("POST", "/api/ai/recommend");
    output::endpoint("GET", "/api/ai/profile/:name");
    output::endpoint("GET", "/api/ai/analyze/:name");
    output::endpoint("GET", "/api/affinity/:class");

    output::endpoint_category("Operations");
    output::endpoint("GET", "/api/drift/:name");
    output::endpoint("POST", "/api/policy/check");
    output::endpoint("GET", "/api/dependencies");
    output::endpoint("POST", "/api/dependencies");
    output::endpoint("GET", "/api/audit");
    output::endpoint("GET", "/api/sla/:workload");

    output::endpoint_category("Templates");
    output::endpoint("GET", "/api/templates");
    output::endpoint("POST", "/api/templates/:name");

    output::endpoint_category("Advanced");
    output::endpoint("GET", "/api/secrets");
    output::endpoint("GET", "/api/events");
    output::endpoint("GET", "/api/events/summary");
    output::endpoint("GET", "/api/environments");
    output::endpoint("GET", "/api/scheduler/utilization");
    output::endpoint("GET", "/api/scheduler/optimize");
    output::endpoint("GET", "/api/orchestrator/status");
    output::endpoint("GET", "/api/orchestrator/summary");

    println!();
    output::muted("Press Ctrl+C to stop the server");
    println!();

    start_server(config).await
}

pub(crate) async fn recommend_command(spec_path: &PathBuf) -> Result<()> {
    use aether::ai::scoring::{format_scoring_report, ScoringEngine};
    use aether::config::Config;

    output::section_with_icon("🤖", "AI-Powered Runtime Recommendation");

    let config = Config::load();
    let workload = Workload::from_file(spec_path)?;
    let engine = ScoringEngine::new(config.engine);
    let result = engine.score(&workload);

    print!("{}", format_scoring_report(&result));

    Ok(())
}

pub(crate) async fn profile_command(spec_path: &PathBuf, name: Option<String>) -> Result<()> {
    use aether::ai::profiler::{format_profile_report, Profiler};
    use aether::config::Config;

    output::section_with_icon("🔍", "Workload Profiler");

    let config = Config::load();
    let profiler = Profiler::new(config.profiler.waste_threshold);

    // If name is provided, look up runtime from state
    let (workload, runtime) = if let Some(ref workload_name) = name {
        let (state, _) = load_workload_state(workload_name)?;
        let ws = get_workload_state(&state, workload_name)?;
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
    use aether::ai::analyzer::{format_analysis_report, LogAnalyzer};
    use aether::config::Config;

    output::section_with_icon("📊", &format!("Log Analysis for '{}'", name));

    let config = Config::load();
    let analyzer = LogAnalyzer::new(config.analyzer);

    let sp = output::spinner("Analyzing logs...");
    let (_state, ws, rt) = load_state_and_runtime(name).await?;
    let logs = rt.logs(&ws.instance, false).await?;

    let analysis = analyzer.analyze(&logs);
    output::spinner_success(&sp, "Analysis complete");

    print!("{}", format_analysis_report(&analysis));

    Ok(())
}

pub(crate) async fn migration_advice_command(name: &str, target: &str) -> Result<()> {
    use aether::ai::migration::{format_migration_advice, MigrationAdvisor};
    use aether::config::Config;

    output::section_with_icon("🔄", &format!("Migration Advisor for '{}'", name));

    let config = Config::load();
    let advisor = MigrationAdvisor::new(config.migration);

    let (state, _) = load_workload_state(name)?;
    let ws = get_workload_state(&state, name)?;

    let target_runtime: RuntimeKind = target.parse()?;

    let workload = Workload::from_file(&ws.spec_path)?;
    let advice = advisor.advise(&workload, ws.runtime, target_runtime);
    print!("{}", format_migration_advice(&advice));

    Ok(())
}

pub(crate) async fn scaling_advice_command() -> Result<()> {
    use aether::ai::scaling::{format_scaling_report, ScalingEngine, TimeSeries};
    use aether::config::Config;

    output::section_with_icon("📈", "Predictive Scaling Advisor");

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

    println!();
    output::info("Using simulated metrics. Connect to Prometheus for real data.");

    Ok(())
}

pub(crate) async fn config_command(show: bool, init: bool) -> Result<()> {
    use aether::config::Config;

    if init {
        let config = Config::default();
        let path = Config::default_path();
        config.save_to(&path)?;
        output::success(&format!("Configuration initialized: {}", path.display()));
        output::detail("Edit this file to customize Aether behavior.");
        return Ok(());
    }

    if show {
        let config = Config::load();
        let yaml = serde_yaml::to_string(&config)?;
        output::section_with_icon(
            "📋",
            &format!("Configuration ({})", Config::default_path().display()),
        );
        println!("{}", yaml);
        return Ok(());
    }

    // Default: show summary
    let config = Config::load();
    let path = Config::default_path();
    let exists = path.exists();

    output::section_with_icon("⚙️", "Aether Configuration");

    println!(
        "{}",
        output::property_table(&[
            ("Config file", path.display().to_string()),
            (
                "Status",
                if exists {
                    "loaded".to_string()
                } else {
                    "using defaults".to_string()
                },
            ),
        ])
    );

    output::section("Engine");
    output::kv("Scoring enabled", &config.engine.enable_scoring.to_string());
    output::kv(
        "Metal3 CPU threshold",
        &format!("{:.0} cores", config.engine.metal3_cpu_threshold),
    );
    output::kv(
        "Metal3 memory threshold",
        &format!("{:.0}Gi", config.engine.metal3_memory_threshold_gi),
    );
    output::kv(
        "Weights",
        &format!(
            "cost={:.0}% perf={:.0}% rel={:.0}% avail={:.0}%",
            config.engine.scoring_weights.cost * 100.0,
            config.engine.scoring_weights.performance * 100.0,
            config.engine.scoring_weights.reliability * 100.0,
            config.engine.scoring_weights.availability * 100.0,
        ),
    );

    output::section("Migration");
    output::kv(
        "Adaptive timing",
        &config.migration.enable_adaptive_timing.to_string(),
    );
    output::kv(
        "Canary error threshold",
        &format!("{:.1}%", config.migration.canary_error_threshold * 100.0),
    );
    output::kv(
        "Max retries",
        &config.migration.max_validation_retries.to_string(),
    );

    output::section("Scaling");
    output::kv("Predictive", &config.scaling.enable_predictive.to_string());
    output::kv(
        "Scale-up threshold",
        &format!("{:.0}%", config.scaling.scale_up_threshold * 100.0),
    );
    output::kv(
        "Scale-down threshold",
        &format!("{:.0}%", config.scaling.scale_down_threshold * 100.0),
    );
    output::kv("Cost-aware", &config.scaling.cost_aware.to_string());

    output::section("Cost");
    output::kv("Include GPU", &config.cost.include_gpu.to_string());
    output::kv("Include network", &config.cost.include_network.to_string());
    output::kv(
        "Include load balancer",
        &config.cost.include_load_balancer.to_string(),
    );

    if !exists {
        println!();
        output::info("Run 'aether config --init' to create a config file");
    }

    Ok(())
}

pub(crate) async fn drift_command(name: &str, reconcile: bool) -> Result<()> {
    use aether::drift::{format_drift_report, DriftDetector};

    let sp = output::spinner(&format!("Checking drift for '{}'...", name));

    let (state, _) = load_workload_state(name)?;
    let ws = get_workload_state(&state, name)?;

    let spec = Workload::from_file(&ws.spec_path)?;
    let detector = DriftDetector::new();
    let report = detector.detect(&spec, &ws);

    output::spinner_success(&sp, "Drift check complete");

    print!("{}", format_drift_report(&report));

    if reconcile && report.has_drift {
        if !output::confirm("Apply reconciliation actions?") {
            output::muted("Reconciliation cancelled.");
            return Ok(());
        }

        output::section_with_icon("🔧", "Executing Reconciliation");
        let mut state = StateStore::load(&StateStore::default_path())?;
        let results = aether::drift::execute_reconciliation(&report, &mut state).await?;
        state.save(&StateStore::default_path())?;

        for r in &results {
            if r.success {
                output::success(&format!("[{}] {}", r.action_type, r.message));
            } else {
                output::error(&format!("[{}] {}", r.action_type, r.message));
            }
        }

        emit_event(
            aether::events::EventSeverity::Info,
            aether::events::EventCategory::DriftDetected,
            "cli",
            Some(name),
            "Drift reconciled",
            &format!("{} action(s) executed", results.len()),
        );
    }

    Ok(())
}

pub(crate) async fn policy_check_command(spec_path: &PathBuf, policy_name: &str) -> Result<()> {
    use aether::policy::{format_policy_report, PolicyEngine};

    output::section_with_icon("📋", "Policy Check");

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
                anyhow::bail!(
                    "Unknown policy set: {}. Use 'production', 'development', or a file path.",
                    policy_name
                );
            }
        }
    };

    let result = engine.evaluate(&workload);
    print!("{}", format_policy_report(&result));

    if !result.passed {
        anyhow::bail!(
            "Policy check failed with {} violation(s)",
            result.violations.len()
        );
    }

    Ok(())
}

pub(crate) async fn deps_command(action: DepsAction) -> Result<()> {
    use aether::dependencies::{format_dependency_report, DependencyGraph};

    let graph_path = DependencyGraph::default_path();
    let mut graph = DependencyGraph::load(&graph_path)?;

    match action {
        DepsAction::Add {
            workload,
            dependency,
        } => {
            graph.add_dependency(&workload, &dependency);
            graph.save(&graph_path)?;
            output::success(&format!("Added dependency: {} -> {}", workload, dependency));

            let issues = graph.validate();
            if !issues.is_empty() {
                println!();
                output::warning("Warnings:");
                for issue in &issues {
                    output::detail(&format!("  - {}", issue));
                }
            }
        }
        DepsAction::Remove {
            workload,
            dependency,
        } => {
            graph.remove_dependency(&workload, &dependency);
            graph.save(&graph_path)?;
            output::success(&format!(
                "Removed dependency: {} -> {}",
                workload, dependency
            ));
        }
        DepsAction::Show => {
            print!("{}", format_dependency_report(&graph));
        }
        DepsAction::Impact { workload } => {
            let impact = graph.impact_analysis(&workload);

            output::section_with_icon("💥", &format!("Impact Analysis for '{}'", workload));

            println!(
                "{}",
                output::property_table(&[
                    ("Severity", impact.severity.to_string()),
                    ("Affected workloads", impact.cascade_count.to_string()),
                ])
            );

            if !impact.affected_workloads.is_empty() {
                output::section("Cascade");
                for affected in &impact.affected_workloads {
                    output::detail(&format!("  - {}", affected));
                }
            }
        }
        DepsAction::Order => match graph.startup_order() {
            Ok(order) => {
                output::section_with_icon("🚀", "Startup Order");
                for (i, name) in order.iter().enumerate() {
                    output::kv(&format!("  {}", i + 1), name);
                }
            }
            Err(e) => {
                output::error(&format!("{}", e));
            }
        },
    }

    Ok(())
}

pub(crate) async fn audit_command(
    last: usize,
    workload: Option<String>,
    summary: bool,
) -> Result<()> {
    use aether::audit::{format_audit_report, AuditLog};

    let audit_path = AuditLog::default_path();
    let log = AuditLog::load(&audit_path)?;

    if summary {
        let s = log.summary();
        output::section_with_icon("📊", "Audit Summary");

        println!(
            "{}",
            output::property_table(&[
                ("Total events", s.total_events.to_string()),
                ("Successes", s.successes.to_string()),
                ("Failures", s.failures.to_string()),
                ("Unique workloads", s.unique_workloads.to_string()),
            ])
        );

        if !s.events_by_action.is_empty() {
            output::section("By Action");
            let mut actions: Vec<_> = s.events_by_action.iter().collect();
            actions.sort_by(|a, b| b.1.cmp(a.1));
            for (action, count) in actions {
                output::kv(action, &count.to_string());
            }
        }
        return Ok(());
    }

    if let Some(ref name) = workload {
        let events = log.events_for(name);
        if events.is_empty() {
            output::muted(&format!("No audit events for '{}'", name));
        } else {
            output::section_with_icon("📋", &format!("Audit Events for '{}'", name));
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
    output_path: Option<PathBuf>,
    list: bool,
) -> Result<()> {
    use aether::templates::{self, TemplateKind, TemplateParams};

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

    if let Some(path) = output_path {
        // Validate output path is safe to write to
        let check_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        let path_str = check_path.to_string_lossy();
        if path_str.contains("..") {
            anyhow::bail!(
                "Refusing to write to path with traversal: {}",
                path.display()
            );
        }
        // Reject known system directories
        for prefix in &["/etc", "/proc", "/sys", "/dev", "/boot", "/sbin", "/usr/sbin"] {
            if check_path.starts_with(prefix) {
                anyhow::bail!("Refusing to write to system path: {}", check_path.display());
            }
        }
        std::fs::write(&path, &yaml)?;
        output::success(&format!("Generated {} template: {}", name, path.display()));
        output::kv_tree("Workload name", &wl_name, true);
    } else {
        output::muted(&format!("# Generated from '{}' template", name));
        println!("{}", yaml);
    }

    Ok(())
}

pub(crate) async fn sla_command(action: SlaAction) -> Result<()> {
    use aether::sla::{format_sla_report, SlaEngine, SlaObservation, SlaTarget};

    // Persist SLA targets via a simple JSON file
    let sla_path = aether::resources::aether_path("sla.json");

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
                _ => anyhow::bail!(
                    "Unknown SLA tier: {}. Use standard, high-availability, or best-effort.",
                    tier
                ),
            };
            output::success(&format!("Added SLA target for '{}' ({})", workload, tier));
            output::kv_tree("Uptime target", &format!("{:.2}%", target.uptime_target_pct), false);
            if let Some(lat) = target.max_latency_ms {
                output::kv_tree("Max latency", &format!("{:.0}ms", lat), false);
            }
            if let Some(err) = target.max_error_rate_pct {
                output::kv_tree("Max error rate", &format!("{:.2}%", err), true);
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
                    output::error(&format!(
                        "No SLA target found for '{}'. Add one with: aether sla add {}",
                        workload, workload
                    ));
                }
            }
        }
        SlaAction::List => {
            let engine = load_engine()?;
            let targets = engine.list_targets();
            if targets.is_empty() {
                output::muted("No SLA targets defined.");
                output::detail("Add one with: aether sla add <workload> --tier standard");
            } else {
                output::section_with_icon("📋", "SLA Targets");
                for target in targets {
                    println!();
                    output::kv_tree(
                        &target.workload,
                        &format!("{:.2}% uptime", target.uptime_target_pct),
                        false,
                    );
                    if let Some(lat) = target.max_latency_ms {
                        output::kv_tree("Max latency", &format!("{:.0}ms", lat), false);
                    }
                    if let Some(err) = target.max_error_rate_pct {
                        output::kv_tree("Max error rate", &format!("{:.2}%", err), false);
                    }
                    if let Some(restarts) = target.max_restarts_per_day {
                        output::kv_tree("Max restarts/day", &restarts.to_string(), true);
                    }
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn secrets_command(action: SecretsAction) -> Result<()> {
    use aether::secrets::{format_secrets_list, SecretStore};

    let path = SecretStore::default_path();
    let mut store = SecretStore::load(&path)?;

    match action {
        SecretsAction::Create { name, namespace } => {
            store.create_secret(&name, &namespace);
            store.save(&path)?;
            aether::metrics::record_secret_operation("create");
            emit_event(
                aether::events::EventSeverity::Info,
                aether::events::EventCategory::SecretRotation,
                "cli",
                None,
                "Secret created",
                &format!("Created secret '{}' in namespace '{}'", name, namespace),
            );
            output::success(&format!(
                "Created secret '{}' in namespace '{}'",
                name, namespace
            ));
        }
        SecretsAction::Set { secret, key, value } => {
            store.set(&secret, &key, &value)?;
            store.save(&path)?;
            aether::metrics::record_secret_operation("set");
            emit_event(
                aether::events::EventSeverity::Info,
                aether::events::EventCategory::SecretRotation,
                "cli",
                None,
                "Secret updated",
                &format!("Set key '{}' in secret '{}'", key, secret),
            );
            output::success(&format!("Set key '{}' in secret '{}'", key, secret));
        }
        SecretsAction::Get { secret, key } => {
            let value = store.get(&secret, &key)?;
            aether::metrics::record_secret_operation("get");
            println!("{}", value);
        }
        SecretsAction::List => {
            let summaries = store.list();
            print!("{}", format_secrets_list(&summaries));
        }
        SecretsAction::Audit => {
            let alerts = store.audit_rotation();
            if alerts.is_empty() {
                output::success("All secrets are within rotation policy limits.");
            } else {
                output::section_with_icon("⚠️", "Rotation Alerts");
                for alert in &alerts {
                    output::kv_tree(&alert.severity.to_string(), &alert.message, false);
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn events_command(
    last: usize,
    severity: Option<String>,
    summary: bool,
) -> Result<()> {
    use aether::events::{format_event_list, format_event_summary, EventBus, EventSeverity};

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
    use aether::environments::{
        format_env_list, EnvTier, EnvironmentManager, PromotionRequest, PromotionStrategy,
    };

    let path = EnvironmentManager::default_path();
    let mut manager = EnvironmentManager::load(&path)?;

    match action {
        EnvAction::Create { name, tier } => {
            let env_tier: EnvTier = tier
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid tier '{}': {}", tier, e))?;
            manager.create_env(&name, env_tier);
            manager.save(&path)?;
            output::success(&format!("Created environment '{}' ({})", name, tier));
        }
        EnvAction::List => {
            let envs = manager.list_envs();
            print!("{}", format_env_list(&envs));
        }
        EnvAction::Promote {
            workload,
            from,
            to,
        } => {
            let request = PromotionRequest {
                workload: workload.clone(),
                from_env: from.clone(),
                to_env: to.clone(),
                strategy: PromotionStrategy::TierAdjusted,
                require_approval: false,
            };
            let result = manager.promote(&request)?;
            manager.save(&path)?;
            aether::metrics::record_env_promotion(&from, &to, true);
            emit_event(
                aether::events::EventSeverity::Info,
                aether::events::EventCategory::Deployment,
                "cli",
                Some(&workload),
                "Environment promotion",
                &format!("Promoted from {} to {}", from, to),
            );
            output::success(&format!(
                "Promoted '{}' from '{}' to '{}'",
                workload, from, to
            ));
            if !result.changes.is_empty() {
                output::section("Changes");
                for change in &result.changes {
                    output::kv_tree(
                        &change.field,
                        &format!("{} -> {} ({})", change.from_value, change.to_value, change.reason),
                        false,
                    );
                }
            }
            if !result.warnings.is_empty() {
                println!();
                output::warning("Warnings:");
                for warn in &result.warnings {
                    output::detail(&format!("  - {}", warn));
                }
            }
        }
        EnvAction::Parity { env1, env2 } => {
            // Parity needs a workload name - check all workloads in env1
            let env = manager
                .get_env(&env1)
                .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", env1))?;
            let workload_names: Vec<String> = env.workloads.keys().cloned().collect();

            if workload_names.is_empty() {
                output::muted(&format!(
                    "No workloads in environment '{}' to compare.",
                    env1
                ));
                return Ok(());
            }

            output::section_with_icon(
                "🔍",
                &format!("Environment Parity: {} vs {}", env1, env2),
            );
            for wl_name in &workload_names {
                match manager.check_parity(&env1, &env2, wl_name) {
                    Ok(report) => {
                        if report.in_sync {
                            output::success(&format!("'{}': In parity", wl_name));
                        } else {
                            output::warning(&format!("'{}': Differences found", wl_name));
                            for diff in &report.diffs {
                                output::detail(&format!(
                                    "    [{}] {}: {} vs {}",
                                    diff.severity, diff.field, diff.env_a_value, diff.env_b_value
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        output::error(&format!("'{}': {}", wl_name, e));
                    }
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn schedule_command(action: ScheduleAction) -> Result<()> {
    use aether::scheduler::{
        format_schedule_decision, format_utilization, Priority, ScheduleRequest, ScheduleStrategy,
        Scheduler,
    };

    let path = Scheduler::default_path();
    let mut scheduler = Scheduler::load(&path)?;

    // Probe live capacity from all available runtimes before scheduling
    scheduler.probe_capacities().await;

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
            if let Ok(affinity_engine) = aether::ai::affinity::AffinityEngine::load(
                &aether::ai::affinity::AffinityEngine::default_path(),
            ) {
                use aether::ai::affinity::WorkloadClass;
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
                    aether::metrics::record_scheduler_placement(
                        &decision.selected_runtime.to_string(),
                        &strategy,
                        true,
                    );
                }
                Err(e) => {
                    aether::metrics::record_scheduler_placement("none", &strategy, false);
                    output::error(&format!("{}", e));
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
                output::success("No optimization suggestions. All runtimes look good.");
            } else {
                output::section_with_icon("💡", "Optimization Suggestions");
                for s in &suggestions {
                    let saving = if let Some(saving) = s.potential_saving {
                        format!(" (potential saving: ${:.2}/day)", saving)
                    } else {
                        String::new()
                    };
                    output::kv_tree(&s.category.to_string(), &format!("{}{}", s.message, saving), false);
                }
            }
        }
        ScheduleAction::Placements => {
            let placements = scheduler.placements();
            if placements.is_empty() {
                output::muted("No workloads placed.");
            } else {
                output::section_with_icon("📍", "Current Placements");
                let rows: Vec<Vec<String>> = placements
                    .iter()
                    .map(|p| {
                        vec![
                            p.workload_name.clone(),
                            format!("{}", p.runtime),
                            format!("{:.0}", p.cpu_reserved),
                            format!("{} MB", p.memory_reserved_mb),
                        ]
                    })
                    .collect();
                println!("{}", output::table(&["Workload", "Runtime", "CPU", "Memory"], rows));
            }
        }
    }

    Ok(())
}

pub(crate) async fn orchestrate_command(action: OrchestrateAction) -> Result<()> {
    use aether::orchestrator::{
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
            output::success(&format!(
                "Registered '{}' for health monitoring ({})",
                name, runtime
            ));
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
            let sp = output::spinner(&format!("Rolling update for '{}'...", name));
            let statuses = orch.rolling_update(&name, replicas, None);
            output::spinner_success(&sp, "Rolling update complete");
            print!("{}", format_rolling_update(&statuses));
        }
        OrchestrateAction::ResetCircuit { name } => {
            if orch.reset_circuit(&name) {
                orch.save(&path)?;
                output::success(&format!("Circuit breaker reset for '{}'", name));
            } else {
                output::error(&format!("Workload '{}' not found", name));
            }
        }
        OrchestrateAction::HealthCheck => {
            let sp = output::spinner("Running health checks...");
            let state_store = StateStore::load(&StateStore::default_path())?;
            let statuses = collect_health_statuses(&orch, &state_store).await;
            let actions = orch.run_health_checks_from_statuses(&statuses);
            orch.save(&path)?;

            // Record health checks to history
            {
                use aether::orchestrator::HealthStatus as OrcHealthStatus;
                let health_path = aether::health::HealthHistory::default_path();
                let mut history = aether::health::HealthHistory::load(&health_path)
                    .unwrap_or_default();
                let now = aether::resources::now_rfc3339();
                for (wl_name, hs) in &statuses {
                    if let Some(ws) = state_store.get(wl_name) {
                        let (inst_state, ready) = match hs {
                            OrcHealthStatus::Healthy => (aether::runtime::InstanceState::Running, true),
                            OrcHealthStatus::Degraded => (aether::runtime::InstanceState::Running, false),
                            OrcHealthStatus::Unhealthy => (aether::runtime::InstanceState::Failed, false),
                            OrcHealthStatus::Unknown => (aether::runtime::InstanceState::Unknown, false),
                        };
                        history.record(aether::health::HealthRecord {
                            timestamp: now.clone(),
                            workload: wl_name.clone(),
                            runtime: ws.runtime,
                            state: inst_state,
                            ready,
                            restart_count: 0,
                            latency_ms: None,
                        });
                    }
                }
                let _ = history.save(&health_path);
            }

            output::spinner_success(&sp, "Health check complete");

            if actions.is_empty() {
                output::success("No actions required.");
            } else {
                output::section_with_icon("🏥", "Health Check Actions");
                for action in &actions {
                    output::detail(&format!("  - {:?}", action));
                }
            }
        }
        OrchestrateAction::Watch { interval } => {
            let recon_config = aether::config::Config::load().reconciliation;
            output::info(&format!(
                "Reconciliation daemon: health={}s, drift={}s, sla={}s (Ctrl+C to stop)",
                recon_config.health_interval_secs,
                recon_config.drift_interval_secs,
                recon_config.sla_interval_secs,
            ));
            println!();

            let mut health_elapsed = 0u64;
            let mut drift_elapsed = 0u64;
            let mut sla_elapsed = 0u64;

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                health_elapsed += interval;
                drift_elapsed += interval;
                sla_elapsed += interval;

                let now_str = chrono::Utc::now().format("%H:%M:%S");
                let state_store = StateStore::load(&StateStore::default_path())?;

                // Health checks
                if health_elapsed >= recon_config.health_interval_secs {
                    health_elapsed = 0;
                    let statuses = collect_health_statuses(&orch, &state_store).await;
                    let actions = orch.run_health_checks_from_statuses(&statuses);
                    orch.save(&path)?;

                    let recon = aether::config::Config::load().reconciliation;
                    let policy = aether::intelligence::policy::AutonomyPolicy::from_config_and_workload(
                        recon.auto_reconcile,
                        None,
                    );
                    let state_arc = std::sync::Arc::new(tokio::sync::RwLock::new(state_store.clone()));
                    let healer = aether::intelligence::healer::execute_orchestrator_actions(
                        &actions,
                        &policy,
                        &state_arc,
                        &StateStore::default_path(),
                        "reconciliation-loop",
                    )
                    .await;

                    if actions.is_empty() {
                        output::success(&format!(
                            "[{}] Health OK ({} workloads)",
                            now_str, statuses.len()
                        ));
                    } else {
                        output::warning(&format!("[{}] {} health action(s):", now_str, actions.len()));
                        for action in &actions {
                            output::detail(&format!("  - {:?}", action));
                        }
                        for executed in &healer.executed {
                            output::success(&format!("  ✓ {executed}"));
                        }
                    }
                }

                // Drift checks
                if drift_elapsed >= recon_config.drift_interval_secs {
                    drift_elapsed = 0;
                    let detector = aether::drift::DriftDetector::new();
                    let mut drift_count = 0usize;
                    let policy = aether::intelligence::policy::AutonomyPolicy::from_config_and_workload(
                        recon_config.auto_reconcile,
                        None,
                    );
                    for ws in state_store.list() {
                        if let Ok(spec) = Workload::from_file(&ws.spec_path) {
                            let report = detector.detect(&spec, ws);
                            if report.has_drift {
                                drift_count += 1;
                                output::warning(&format!(
                                    "[{}] Drift detected on '{}': {} item(s)",
                                    now_str, ws.name, report.drifts.len()
                                ));
                                emit_event(
                                    aether::events::EventSeverity::Warning,
                                    aether::events::EventCategory::DriftDetected,
                                    "reconciliation-loop",
                                    Some(&ws.name),
                                    "Drift detected",
                                    &format!("{} drift(s)", report.drifts.len()),
                                );
                                if policy.allows_drift_reconcile() {
                                    let mut store = StateStore::load(&StateStore::default_path())?;
                                    if let Ok(results) =
                                        aether::drift::execute_reconciliation(&report, &mut store).await
                                    {
                                        store.save(&StateStore::default_path())?;
                                        output::success(&format!(
                                            "[{}] Auto-reconciled '{}': {} action(s)",
                                            now_str,
                                            ws.name,
                                            results.len()
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    if drift_count == 0 {
                        output::success(&format!("[{}] Drift: no divergence", now_str));
                    }
                }

                // SLA checks — verify health-based uptime against baseline
                if sla_elapsed >= recon_config.sla_interval_secs {
                    sla_elapsed = 0;
                    let health_path = aether::health::HealthHistory::default_path();
                    let history = aether::health::HealthHistory::load(&health_path)
                        .unwrap_or_default();
                    let mut at_risk = 0usize;
                    for ws in state_store.list() {
                        let uptime = history.uptime_percent(&ws.name);
                        // Flag workloads below 99% uptime (Standard SLA baseline)
                        if uptime > 0.0 && uptime < 99.0 {
                            at_risk += 1;
                            output::warning(&format!(
                                "[{}] SLA at risk for '{}': {:.1}% uptime",
                                now_str, ws.name, uptime
                            ));
                            emit_event(
                                aether::events::EventSeverity::Warning,
                                aether::events::EventCategory::SlaViolation,
                                "reconciliation-loop",
                                Some(&ws.name),
                                "SLA at risk",
                                &format!("{:.1}% uptime", uptime),
                            );
                        }
                    }
                    if at_risk == 0 {
                        output::success(&format!("[{}] SLA: all workloads healthy", now_str));
                    }
                }

                // Intent constraint checks — flag workloads whose intent
                // SLA or budget conditions are violated by current state
                {
                    let health_path = aether::health::HealthHistory::default_path();
                    let history = aether::health::HealthHistory::load(&health_path)
                        .unwrap_or_default();
                    for ws in state_store.list() {
                        if let Ok(spec) = Workload::from_file(&ws.spec_path) {
                            if let Some(ref intent) = spec.intent {
                                // Check SLA availability constraint
                                if let Some(ref sla) = intent.sla {
                                    if let Some(min_avail) = sla.min_availability_pct {
                                        let uptime = history.uptime_percent(&ws.name);
                                        if uptime > 0.0 && uptime < min_avail {
                                            output::warning(&format!(
                                                "[{}] Intent violation for '{}': uptime {:.1}% < intent min {:.1}%",
                                                now_str, ws.name, uptime, min_avail
                                            ));
                                            emit_event(
                                                aether::events::EventSeverity::Warning,
                                                aether::events::EventCategory::IntentViolation,
                                                "reconciliation-loop",
                                                Some(&ws.name),
                                                "Intent SLA violation",
                                                &format!("Uptime {:.1}% below intent min {:.1}%", uptime, min_avail),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Evaluate alert rules against current metrics
                {
                    let events_path = aether::events::EventBus::default_path();
                    if let Ok(mut bus) = aether::events::EventBus::load(&events_path) {
                        let alert_policy_config = aether::config::Config::load().policy;
                        let workloads: Vec<_> = state_store.list().into_iter().cloned().collect();
                        let metrics =
                            aether::events::SystemMetrics::collect(&workloads, &alert_policy_config);

                        let fired = bus.evaluate_rules(&metrics);
                        if !fired.is_empty() {
                            output::warning(&format!(
                                "[{}] Alerts: {} rule(s) triggered",
                                now_str, fired.len()
                            ));
                            let _ = bus.save(&events_path);
                        }
                    }
                }

                // Process webhook retry queue
                aether::events::WebhookQueue::process_queue_once();
            }
        }
    }

    Ok(())
}

pub(crate) async fn affinity_command(action: AffinityAction) -> Result<()> {
    use aether::ai::affinity::{format_affinity_report, AffinityEngine, WorkloadClass};

    let path = AffinityEngine::default_path();
    let engine = AffinityEngine::load(&path)?;

    match action {
        AffinityAction::Recommend { class } => {
            let wl_class: WorkloadClass = class.parse()?;
            let scores = engine.recommend(&wl_class);
            if let Some(top) = scores.first() {
                aether::metrics::record_affinity_recommendation(
                    &class,
                    &top.runtime.to_string(),
                );
            }
            print!("{}", format_affinity_report(&wl_class, &scores));
        }
        AffinityAction::Matrix => {
            let matrix = engine.compatibility_matrix();
            output::section_with_icon("🔢", "Compatibility Matrix");
            let mut entries: Vec<_> = matrix.iter().collect();
            entries.sort_by_key(|((class, rt), _)| (format!("{}", class), format!("{}", rt)));

            let rows: Vec<Vec<String>> = entries
                .iter()
                .map(|((class, rt), entry)| {
                    vec![
                        if entry.compatible {
                            "✓".to_string()
                        } else {
                            "✗".to_string()
                        },
                        format!("{}", class),
                        format!("{}", rt),
                        format!("{:.0}%", entry.score * 100.0),
                        entry.deployments.to_string(),
                    ]
                })
                .collect();
            println!(
                "{}",
                output::table(&["Compat", "Class", "Runtime", "Score", "Deployments"], rows)
            );
        }
        AffinityAction::Stats => {
            let stats = engine.stats();
            output::section_with_icon("📊", "Affinity Learning Stats");

            println!(
                "{}",
                output::property_table(&[
                    ("Total outcomes", stats.total_outcomes.to_string()),
                    ("Successes", stats.successes.to_string()),
                    ("Failures", stats.failures.to_string()),
                    (
                        "Known incompatibilities",
                        stats.incompatibilities.to_string(),
                    ),
                ])
            );

            if !stats.by_runtime.is_empty() {
                output::section("By Runtime");
                for (rt, count) in &stats.by_runtime {
                    output::kv(rt, &count.to_string());
                }
            }
            if !stats.by_class.is_empty() {
                output::section("By Workload Class");
                for (class, count) in &stats.by_class {
                    output::kv(class, &count.to_string());
                }
            }
        }
    }

    Ok(())
}

pub(crate) async fn rollback_command(name: &str, version: Option<usize>, list: bool) -> Result<()> {
    use aether::backup::{Backup, SnapshotManager};

    let snap_mgr = SnapshotManager::new();

    // List mode: show available snapshots and return
    if list {
        output::section_with_icon("📋", &format!("Snapshots for workload '{}'", name));
        let snapshots = snap_mgr.list_snapshots(name)?;
        if snapshots.is_empty() {
            output::warning(&format!("No snapshots found for workload '{}'", name));
            return Ok(());
        }
        for (i, snap) in snapshots.iter().enumerate() {
            output::kv_tree(
                &format!("#{}", i),
                &snap.display().to_string(),
                i == snapshots.len() - 1,
            );
        }
        return Ok(());
    }

    output::section_with_icon("⏪", &format!("Rolling back workload '{}'", name));

    // Find snapshot based on version or latest
    let snapshot_path = if let Some(v) = version {
        let snapshots = snap_mgr.list_snapshots(name)?;
        snapshots.into_iter().nth(v).ok_or_else(|| {
            anyhow::anyhow!(
                "Snapshot version {} not found for workload '{}'. Use --list to see available versions.",
                v,
                name
            )
        })?
    } else {
        snap_mgr
            .latest_snapshot(name)?
            .ok_or_else(|| anyhow::anyhow!("No snapshot found for workload '{}'", name))?
    };

    output::kv_tree("Found snapshot", &snapshot_path.display().to_string(), true);

    // Load snapshot
    let backup = Backup::load(&snapshot_path)?;
    let snapshot_ws = backup
        .workloads
        .first()
        .ok_or_else(|| anyhow::anyhow!("Snapshot is empty"))?;

    // Stop current instance (best-effort)
    let mut state = StateStore::load(&StateStore::default_path())?;
    if let Some(current) = state.get(name) {
        match aether::runtime::create_runtime_ns(&current.runtime, get_namespace()).await {
            Ok(runtime) => {
                if let Err(e) = runtime.stop(&current.instance).await {
                    tracing::warn!("Failed to stop current instance: {}", e);
                }
            }
            Err(e) => tracing::warn!("Failed to create runtime for stop: {}", e),
        }
    }

    // Re-deploy from snapshot
    let sp = output::spinner("Re-deploying from snapshot...");
    let runtime = aether::runtime::create_runtime_ns(&snapshot_ws.runtime, get_namespace()).await?;
    let spec = Workload::from_file(&snapshot_ws.spec_path)?;
    let image = runtime.build(&spec).await?;
    let instance = runtime.run(&image, &spec).await?;

    output::spinner_success(&sp, "Re-deployment complete");
    output::kv_tree(
        "Re-deployed instance",
        &format!("{} ({})", instance.name, instance.id),
        true,
    );

    // Update state (reuse the already-loaded store)
    state.upsert(
        name.to_string(),
        aether::state::WorkloadState {
            name: name.to_string(),
            runtime: snapshot_ws.runtime,
            instance,
            spec_path: snapshot_ws.spec_path.clone(),
            created_at: snapshot_ws.created_at.clone(),
            updated_at: aether::resources::now_rfc3339(),
            os_version: snapshot_ws.os_version.clone(),
            node_labels: snapshot_ws.node_labels.clone(),
        },
    );
    state.save(&StateStore::default_path())?;

    emit_event(
        aether::events::EventSeverity::Warning,
        aether::events::EventCategory::Deployment,
        "cli",
        Some(name),
        "Workload rolled back",
        &format!("Rolled back to snapshot {}", snapshot_path.display()),
    );

    output::success(&format!("Rollback complete for '{}'", name));

    Ok(())
}

/// Collect live HealthStatus for all managed workloads, grouping by runtime
/// to avoid creating duplicate runtime clients (N+1).
pub(crate) async fn collect_health_statuses(
    orch: &aether::orchestrator::Orchestrator,
    state: &StateStore,
) -> std::collections::HashMap<String, aether::orchestrator::HealthStatus> {
    use aether::orchestrator::HealthStatus;
    use aether::runtime::InstanceState;

    let managed = orch.list_workloads();
    let mut statuses = std::collections::HashMap::new();

    // Group managed workloads by runtime
    let mut by_runtime: std::collections::HashMap<RuntimeKind, Vec<_>> =
        std::collections::HashMap::new();
    for mw in &managed {
        if let Some(ws) = state.get(&mw.name) {
            by_runtime
                .entry(ws.runtime)
                .or_default()
                .push((mw.name.clone(), ws.instance.clone()));
        }
    }

    for (kind, workloads) in by_runtime {
        match aether::runtime::create_runtime_ns(&kind, get_namespace()).await {
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
    use aether::drift::{format_live_diff, DiffRow, LiveDiffReport};

    output::section_with_icon(
        "🔍",
        &format!("Comparing spec vs stored vs live state for '{}'", name),
    );

    let sp = output::spinner("Fetching live state...");
    let (_state, workload_state, runtime) = load_state_and_runtime(name).await?;

    // Load spec
    let spec = Workload::from_file(&workload_state.spec_path)?;

    // Query live status
    let live_status = runtime.status(&workload_state.instance).await?;

    output::spinner_success(&sp, "State retrieved");

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
            matches: live_status.state == aether::runtime::InstanceState::Running,
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

    // Show git-style colored changes for fields that differ
    if has_differences {
        output::section_with_icon("📝", "Changes");
        for row in &report.rows {
            if !row.matches {
                output::change(
                    &row.field,
                    &row.spec_value,
                    &row.live_value,
                );
            }
        }
    }

    Ok(())
}

pub(crate) async fn webhook_command(action: WebhookAction) -> Result<()> {
    use aether::events::{
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

            output::success(&format!(
                "Added webhook channel '{}' -> {} ({})",
                name, url, method
            ));
        }
        WebhookAction::Remove { name } => {
            if bus.remove_channel(&name) {
                bus.save(&path)?;
                output::success(&format!("Removed webhook channel '{}'", name));
            } else {
                output::error(&format!("Channel '{}' not found", name));
            }
        }
        WebhookAction::List => {
            let channels = bus.channels();
            if channels.is_empty() {
                output::muted("No notification channels configured.");
                return Ok(());
            }

            output::section_with_icon("🔔", "Notification Channels");

            let rows: Vec<Vec<String>> = channels
                .iter()
                .map(|ch| {
                    vec![
                        ch.name.clone(),
                        format!("{}", ch.channel_type),
                        if ch.enabled {
                            "yes".to_string()
                        } else {
                            "no".to_string()
                        },
                        format!("{}", ch.min_severity),
                    ]
                })
                .collect();
            println!(
                "{}",
                output::table(&["Name", "Type", "Enabled", "Min Severity"], rows)
            );
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
            output::success(&format!("Test notification sent to channel '{}'", name));
        }
        WebhookAction::Queue => {
            let queue = aether::events::WebhookQueue::load(
                &aether::events::WebhookQueue::default_path(),
            )?;
            if queue.pending.is_empty() {
                output::muted("No pending webhook deliveries.");
            } else {
                output::section_with_icon("📬", "Pending Webhook Deliveries");
                let rows: Vec<Vec<String>> = queue
                    .pending
                    .iter()
                    .map(|p| {
                        vec![
                            p.url.clone(),
                            p.method.clone(),
                            format!("{}/{}", p.attempts, p.max_attempts),
                            p.next_attempt_at.chars().take(19).collect::<String>(),
                        ]
                    })
                    .collect();
                println!(
                    "{}",
                    output::table(&["URL", "Method", "Attempts", "Next Retry"], rows)
                );
            }
        }
        WebhookAction::Flush => {
            output::header("📤", "Flushing Webhook Queue");
            let before = aether::events::WebhookQueue::load(
                &aether::events::WebhookQueue::default_path(),
            )
            .map(|q| q.pending.len())
            .unwrap_or(0);
            aether::events::WebhookQueue::process_queue_once();
            let after = aether::events::WebhookQueue::load(
                &aether::events::WebhookQueue::default_path(),
            )
            .map(|q| q.pending.len())
            .unwrap_or(0);
            output::success(&format!(
                "Processed queue: {} before, {} remaining",
                before, after
            ));
        }
    }

    Ok(())
}

/// Scan a directory for `*.yaml` / `*.yml` files and parse each as a Workload.
/// Invalid files are skipped with a warning.
fn discover_workloads(dir: &Path) -> Result<Vec<(PathBuf, Workload)>> {
    if !dir.is_dir() {
        anyhow::bail!("'{}' is not a directory", dir.display());
    }

    let mut workloads = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        match path.extension().and_then(|e| e.to_str()) {
            Some("yaml" | "yml") => {}
            _ => continue,
        }
        match Workload::from_file(&path.to_path_buf()) {
            Ok(w) => workloads.push((path, w)),
            Err(e) => {
                output::warning(&format!("Skipping {}: {}", path.display(), e));
            }
        }
    }

    if workloads.is_empty() {
        anyhow::bail!("No valid workload specs found in '{}'", dir.display());
    }

    Ok(workloads)
}

/// Order workloads according to DependencyGraph startup order.
/// Workloads present in the graph come first (in topological order);
/// workloads not in the graph are appended in discovery order.
fn order_workloads_by_deps(
    workloads: Vec<(PathBuf, Workload)>,
    graph: &aether::dependencies::DependencyGraph,
) -> Vec<(PathBuf, Workload)> {
    let order = graph.startup_order().unwrap_or_default();

    // Index workloads by name for fast lookup
    let mut by_name: std::collections::HashMap<String, (PathBuf, Workload)> = workloads
        .into_iter()
        .map(|(p, w)| (w.metadata.name.clone(), (p, w)))
        .collect();

    let mut ordered: Vec<(PathBuf, Workload)> = Vec::with_capacity(by_name.len());

    // First: workloads that appear in the graph's startup order
    for name in &order {
        if let Some(entry) = by_name.remove(name) {
            ordered.push(entry);
        }
    }

    // Then: remaining workloads in alphabetical order (stable for reproducibility)
    let mut remaining: Vec<_> = by_name.into_values().collect();
    remaining.sort_by(|a, b| a.1.metadata.name.cmp(&b.1.metadata.name));
    ordered.extend(remaining);

    ordered
}

/// Deploy all workloads discovered in a directory.
pub(crate) async fn deploy_command(
    dir: &Path,
    runtime_override: Option<String>,
    fail_fast: bool,
    dry_run: bool,
) -> Result<()> {
    let sp = output::spinner(&format!("Discovering workloads in '{}'...", dir.display()));

    let workloads = discover_workloads(dir)?;

    // Load dependency graph for ordering
    let graph_path = aether::dependencies::DependencyGraph::default_path();
    let graph = aether::dependencies::DependencyGraph::load(&graph_path).unwrap_or_default();

    let ordered = order_workloads_by_deps(workloads, &graph);

    output::spinner_success(
        &sp,
        &format!("Found {} workloads", ordered.len()),
    );

    // Show plan as table
    output::section_with_icon("📋", &format!("Deployment Plan ({} workloads)", ordered.len()));
    let rows: Vec<Vec<String>> = ordered
        .iter()
        .enumerate()
        .map(|(i, (path, w))| {
            vec![
                format!("{}", i + 1),
                w.metadata.name.clone(),
                path.display().to_string(),
            ]
        })
        .collect();
    println!("{}", output::table(&["#", "Workload", "Spec"], rows));

    if dry_run {
        output::info("Dry run -- no workloads were deployed.");
        return Ok(());
    }

    println!();

    let total = ordered.len();
    let mut succeeded = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut event_batch = EventBatch::new();

    for (i, (path, workload)) in ordered.iter().enumerate() {
        let sp = output::spinner(&format!(
            "[{}/{}] Deploying '{}'...",
            i + 1,
            total,
            workload.metadata.name
        ));

        match deploy_single_workload(workload, path, runtime_override.as_deref(), Some(&mut event_batch)).await {
            Ok(()) => {
                output::spinner_success(&sp, &format!("Deployed '{}'", workload.metadata.name));
                succeeded += 1;

                // If this workload has dependents, wait for it to become ready
                // before deploying the next workload in the dependency chain.
                if !graph.dependents_of(&workload.metadata.name).is_empty() {
                    let state = StateStore::load(&StateStore::default_path())?;
                    if let Some(ws) = state.get(&workload.metadata.name) {
                        if let Ok(rt) = aether::runtime::create_runtime_ns(&ws.runtime, get_namespace()).await {
                            let mut ready = false;
                            for _ in 0..12 { // 12 * 5s = 60s max wait
                                if let Ok(status) = rt.status(&ws.instance).await {
                                    if status.ready {
                                        ready = true;
                                        break;
                                    }
                                }
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            }
                            if !ready {
                                output::warning(&format!(
                                    "Workload '{}' not ready after 60s — dependents may fail",
                                    workload.metadata.name
                                ));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let msg = format!("{}", e);
                output::spinner_fail(
                    &sp,
                    &format!("Failed to deploy '{}'", workload.metadata.name),
                );
                output::error(&format!("Failed to deploy '{}': {}", workload.metadata.name, msg));
                failed.push((workload.metadata.name.clone(), msg));
                if fail_fast {
                    output::error(&format!(
                        "Aborting (--fail-fast): {}/{} succeeded, 1 failed.",
                        succeeded, total
                    ));
                    return Err(anyhow::anyhow!(
                        "Deploy aborted: '{}' failed",
                        failed.last().map(|f| f.0.as_str()).unwrap_or("unknown")
                    ));
                }
            }
        }
    }

    // Summary
    if failed.is_empty() {
        output::summary_success("Deploy Complete", &[
            ("Succeeded", succeeded.to_string()),
            ("Total", total.to_string()),
        ]);
    } else {
        let mut items: Vec<(&str, String)> = vec![
            ("Succeeded", succeeded.to_string()),
            ("Failed", failed.len().to_string()),
            ("Total", total.to_string()),
        ];
        for (name, err) in &failed {
            items.push((name.as_str(), err.clone()));
        }
        output::summary_error("Deploy Failed", &items);
        anyhow::bail!(
            "{} of {} workloads failed to deploy",
            failed.len(),
            total
        );
    }

    Ok(())
}

/// Emit an event and save to the event bus (best-effort, errors are logged).
/// For single operations. Use `EventBatch` for loops to avoid N disk round-trips.
pub(crate) fn emit_event(
    severity: aether::events::EventSeverity,
    category: aether::events::EventCategory,
    source: &str,
    workload: Option<&str>,
    title: &str,
    message: &str,
) {
    let path = aether::events::EventBus::default_path();
    match aether::events::EventBus::load(&path) {
        Ok(mut bus) => {
            bus.emit_simple(severity, category, source, workload, title, message);
            if let Err(e) = bus.save(&path) {
                tracing::warn!("Failed to save event bus: {}", e);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to emit event: {}", e);
        }
    }
}

/// Batched event emitter — loads once, accumulates events, saves once on drop.
pub(crate) struct EventBatch {
    bus: aether::events::EventBus,
    path: std::path::PathBuf,
}

impl EventBatch {
    pub(crate) fn new() -> Self {
        let path = aether::events::EventBus::default_path();
        let bus = aether::events::EventBus::load(&path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load event bus, starting fresh: {}", e);
            aether::events::EventBus::new()
        });
        Self { bus, path }
    }

    pub(crate) fn emit(
        &mut self,
        severity: aether::events::EventSeverity,
        category: aether::events::EventCategory,
        source: &str,
        workload: Option<&str>,
        title: &str,
        message: &str,
    ) {
        self.bus.emit_simple(severity, category, source, workload, title, message);
    }
}

impl Drop for EventBatch {
    fn drop(&mut self) {
        if let Err(e) = self.bus.save(&self.path) {
            tracing::warn!("Failed to save event batch: {}", e);
        }
    }
}

// ─── dry-run: show what would happen without executing ───────────────
pub(crate) async fn dry_run_command(spec_path: &PathBuf, runtime_override: Option<String>) -> Result<()> {
    let workload = Workload::from_file(spec_path)?;
    let engine = Engine::new();

    let runtime_kind = if let Some(override_str) = runtime_override {
        override_str.parse::<RuntimeKind>()?
    } else {
        engine.decide(&workload)?
    };

    output::header("🔍", "Dry Run — Deploy Preview");

    println!(
        "{}",
        output::property_table(&[
            ("Workload", workload.metadata.name.clone()),
            ("Runtime", output::runtime_display(&runtime_kind)),
            ("Image", workload.image_name()),
            ("CPU", workload.requirements.cpu.clone()),
            ("Memory", workload.requirements.memory.clone()),
            ("Storage", workload.requirements.storage.clone()),
            ("Ports", workload.network.ports.iter()
                .map(|p| format!("{}:{}", p.service_port, p.container_port))
                .collect::<Vec<_>>()
                .join(", ")),
        ])
    );

    output::muted("\n[dry-run] No resources were created.");
    Ok(())
}

// ─── exec: run a shell inside a workload ─────────────────────────────
/// Run a subprocess with optional timeout (0 = no timeout).
/// Uses tokio::process::Command for proper async child management.
async fn run_with_timeout(cmd: std::process::Command, label: &str, timeout_secs: u64) -> Result<()> {
    // Convert std::process::Command to tokio::process::Command for async wait
    let mut tokio_cmd = tokio::process::Command::from(cmd);
    let mut child = tokio_cmd.spawn()?;

    if timeout_secs == 0 {
        // No timeout — just await the child
        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("{} exited with status {}", label, status);
        }
        return Ok(());
    }

    // Await with timeout
    match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        child.wait(),
    ).await {
        Ok(Ok(status)) => {
            if !status.success() {
                anyhow::bail!("{} exited with status {}", label, status);
            }
            Ok(())
        }
        Ok(Err(e)) => Err(e.into()),
        Err(_) => {
            let _ = child.kill().await;
            anyhow::bail!("{} timed out after {}s", label, timeout_secs);
        }
    }
}

pub(crate) async fn exec_command(name: &str, command: &str, interactive: bool, timeout: u64) -> Result<()> {
    let (_state, ws, _rt) = load_state_and_runtime(name).await?;

    output::header("🐚", &format!("Exec into {}", name));

    match ws.runtime {
        RuntimeKind::Podman | RuntimeKind::Docker => {
            let bin = if ws.runtime == RuntimeKind::Docker { "docker" } else { "podman" };
            let mut cmd = std::process::Command::new(bin);
            cmd.args(["exec"]);
            if interactive {
                cmd.args(["-it"]);
            }
            cmd.arg(&ws.instance.id).arg(command);
            run_with_timeout(cmd, &format!("{} exec", bin), timeout).await?;
        }
        RuntimeKind::Kubernetes => {
            let mut cmd = std::process::Command::new("kubectl");
            cmd.args(["exec"]);
            if interactive {
                cmd.args(["-it"]);
            }
            cmd.arg(&ws.instance.name)
                .arg("--")
                .arg(command);
            run_with_timeout(cmd, "kubectl exec", timeout).await?;
        }
        RuntimeKind::KubeVirt => {
            let mut cmd = std::process::Command::new("virtctl");
            cmd.args(["console", &ws.instance.name]);
            run_with_timeout(cmd, "virtctl console", timeout).await?;
        }
        RuntimeKind::Metal3 => {
            anyhow::bail!(
                "Exec is not supported for Metal3 bare-metal hosts.\n\
                 Hint: Use SSH or BMC console to access the host directly."
            );
        }
    }
    Ok(())
}

// ─── port-forward: forward local ports to workload ───────────────────
pub(crate) async fn port_forward_command(name: &str, ports: &str, timeout: u64) -> Result<()> {
    let (_state, ws, _rt) = load_state_and_runtime(name).await?;

    output::header("🔀", &format!("Port-forward {}", name));

    // Parse port mapping (local:remote)
    let parts: Vec<&str> = ports.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid port format '{}'. Expected local:remote (e.g., 8080:80)",
            ports
        );
    }
    let local_port: u16 = parts[0].parse().map_err(|_| {
        anyhow::anyhow!("Invalid local port '{}'. Must be a number 1-65535", parts[0])
    })?;
    if local_port == 0 {
        anyhow::bail!("Local port cannot be 0 (reserved for OS-assigned ports)");
    }
    let remote_port: u16 = parts[1].parse().map_err(|_| {
        anyhow::anyhow!("Invalid remote port '{}'. Must be a number 1-65535", parts[1])
    })?;
    if remote_port == 0 {
        anyhow::bail!("Remote port cannot be 0");
    }

    match ws.runtime {
        RuntimeKind::Podman | RuntimeKind::Docker => {
            let rt_name = if ws.runtime == RuntimeKind::Docker { "Docker" } else { "Podman" };
            output::info(&format!(
                "{} containers use direct port mapping. Port {}:{} was configured at deploy time.",
                rt_name, local_port, remote_port
            ));
            output::muted(&format!(
                "Access your workload at http://localhost:{}",
                local_port
            ));
        }
        RuntimeKind::Kubernetes => {
            output::info(&format!(
                "Forwarding localhost:{} → {}:{}",
                local_port, ws.instance.name, remote_port
            ));
            let mut cmd = std::process::Command::new("kubectl");
            cmd.args([
                "port-forward",
                &format!("pod/{}", ws.instance.name),
                &format!("{}:{}", local_port, remote_port),
            ]);
            run_with_timeout(cmd, "kubectl port-forward", timeout).await?;
        }
        RuntimeKind::KubeVirt => {
            output::info(&format!(
                "Forwarding localhost:{} → {}:{}",
                local_port, ws.instance.name, remote_port
            ));
            let mut cmd = std::process::Command::new("virtctl");
            cmd.args([
                "port-forward",
                &ws.instance.name,
                &format!("{}:{}", local_port, remote_port),
            ]);
            run_with_timeout(cmd, "virtctl port-forward", timeout).await?;
        }
        RuntimeKind::Metal3 => {
            anyhow::bail!(
                "Port-forward is not supported for Metal3 bare-metal hosts.\n\
                 Hint: Configure networking directly on the host."
            );
        }
    }
    Ok(())
}

// ─── cp: copy files to/from a workload pod ───────────────────────────
pub(crate) async fn cp_command(name: &str, src: &str, dest: &str, timeout: u64) -> Result<()> {
    let (_state, ws, _rt) = load_state_and_runtime(name).await?;

    output::header("📋", &format!("Copy files for {}", name));

    match ws.runtime {
        RuntimeKind::Podman | RuntimeKind::Docker => {
            let bin = if ws.runtime == RuntimeKind::Docker {
                "docker"
            } else {
                "podman"
            };
            let mut cmd = std::process::Command::new(bin);
            cmd.args(["cp", src, dest]);
            run_with_timeout(cmd, &format!("{} cp", bin), timeout).await?;
        }
        RuntimeKind::Kubernetes | RuntimeKind::KubeVirt => {
            let pod = ws.instance.name.clone();
            let src_arg = if let Some(path) = src.strip_prefix("pod:") {
                format!("{}/{}", pod, path)
            } else {
                src.to_string()
            };
            let dest_arg = if let Some(path) = dest.strip_prefix("pod:") {
                format!("{}/{}", pod, path)
            } else {
                dest.to_string()
            };
            let mut cmd = std::process::Command::new("kubectl");
            cmd.args(["cp", &src_arg, &dest_arg]);
            run_with_timeout(cmd, "kubectl cp", timeout).await?;
        }
        RuntimeKind::Metal3 => {
            anyhow::bail!(
                "Copy is not supported for Metal3 bare-metal hosts.\n\
                 Hint: Use scp or rsync to transfer files to the host."
            );
        }
    }
    output::success("Copy completed");
    Ok(())
}

// ─── watch: file-watch and auto-redeploy ─────────────────────────────
pub(crate) async fn watch_command(spec_path: &PathBuf, runtime: Option<String>) -> Result<()> {
    use std::time::{Instant, SystemTime};

    output::header("👁", "Watch Mode");
    output::info(&format!("Watching {} for changes...", spec_path.display()));
    output::muted("Press Ctrl+C to stop");

    let mut last_modified = std::fs::metadata(spec_path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|_| SystemTime::now());

    // Debounce: wait at least 1s after the last change before deploying,
    // to avoid triggering on rapid successive saves (editors do tmp-write + rename).
    let debounce_duration = std::time::Duration::from_secs(1);
    let mut pending_change: Option<Instant> = None;
    let mut deploy_count: u32 = 0;

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let current_modified = match std::fs::metadata(spec_path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if current_modified > last_modified {
            last_modified = current_modified;
            pending_change = Some(Instant::now());
        }

        // Deploy once the debounce window has elapsed
        if let Some(changed_at) = pending_change {
            if changed_at.elapsed() >= debounce_duration {
                pending_change = None;
                deploy_count += 1;
                output::section_with_icon(
                    "🔄",
                    &format!("Change detected — redeploying (#{})...", deploy_count),
                );

                // Read the file content atomically, then parse — minimizes
                // the TOCTOU window between stat() and read().
                let content = match std::fs::read_to_string(spec_path) {
                    Ok(c) => c,
                    Err(e) => {
                        output::error(&format!("Failed to read spec: {}", e));
                        output::muted("Watching for more changes...");
                        continue;
                    }
                };
                let workload: Workload = match serde_yaml::from_str(&content) {
                    Ok(w) => w,
                    Err(e) => {
                        output::error(&format!("Failed to parse spec: {}", e));
                        output::muted("Watching for more changes...");
                        continue;
                    }
                };
                if let Err(e) = workload.validate() {
                    output::error(&format!("Invalid spec: {}", e));
                    output::muted("Watching for more changes...");
                    continue;
                }
                match deploy_single_workload(&workload, spec_path, runtime.as_deref(), None).await {
                    Ok(()) => output::success("Redeployment complete"),
                    Err(e) => output::error(&format!("Redeployment failed: {}", e)),
                }

                output::muted("Watching for more changes...");
            }
        }
    }
}

// ─── compare: cross-runtime comparison ───────────────────────────────
pub(crate) async fn compare_command(spec_path: &PathBuf) -> Result<()> {
    let workload = Workload::from_file(spec_path)?;
    let engine = Engine::new();

    output::header("⚖️", "Cross-Runtime Comparison");

    let cpu = aether::resources::parse_cpu(&workload.requirements.cpu);
    let mem_gi = aether::resources::parse_memory_gi(&workload.requirements.memory);

    // Evaluate each runtime
    let recommended = engine.decide(&workload).ok();
    let mut rows = Vec::new();
    for runtime in aether::runtime::RuntimeKind::ALL {
        let is_recommended = recommended.as_ref().map(|r| *r == runtime).unwrap_or(false);

        let suitability = match runtime {
            RuntimeKind::Podman | RuntimeKind::Docker => {
                if workload.requirements.gpu.is_some() { "Limited (no GPU)" }
                else if cpu > 8.0 { "Adequate" }
                else { "Excellent" }
            }
            RuntimeKind::Kubernetes => {
                if workload.network.service { "Excellent" }
                else { "Good" }
            }
            RuntimeKind::KubeVirt => {
                if workload.requirements.gpu.is_some() { "Excellent (GPU passthrough)" }
                else if mem_gi > 16.0 { "Good (VM isolation)" }
                else { "Adequate" }
            }
            RuntimeKind::Metal3 => {
                if cpu > 16.0 || mem_gi > 64.0 { "Excellent (bare-metal perf)" }
                else { "Over-provisioned" }
            }
        };

        let limitations = match runtime {
            RuntimeKind::Podman => "Single host, no HA, no service mesh",
            RuntimeKind::Docker => "Single host, no HA, no service mesh",
            RuntimeKind::Kubernetes => "Requires cluster, higher complexity",
            RuntimeKind::KubeVirt => "Requires KubeVirt operator, VM overhead",
            RuntimeKind::Metal3 => "Requires BMC, slow provisioning, no autoscale",
        };

        let marker = if is_recommended { "★ " } else { "  " };
        rows.push(vec![
            format!("{}{}", marker, output::runtime_display(&runtime)),
            suitability.to_string(),
            limitations.to_string(),
        ]);
    }

    println!(
        "{}",
        output::table(
            &["Runtime", "Suitability", "Limitations"],
            rows,
        )
    );

    // Cost comparison
    let mut cost_rows = Vec::new();
    if let Ok(estimates) = aether::cost::estimate_all_providers(&workload) {
        for estimate in &estimates {
            cost_rows.push(vec![
                format!("{:?}", estimate.provider),
                format!("${:.2}/mo", estimate.total_monthly),
                format!("${:.4}/hr", estimate.total_monthly / 730.0),
            ]);
        }
    }

    println!(
        "\n{}",
        output::table(
            &["Provider", "Monthly", "Hourly"],
            cost_rows,
        )
    );

    output::muted("\n★ = AI-recommended runtime for this workload");
    Ok(())
}

// ─── decide: explain runtime placement ───────────────────────────────
pub(crate) async fn decide_command(spec_path: &PathBuf, explain: bool) -> Result<()> {
    use aether::ai::scoring::{format_scoring_report_with_options, ScoringEngine};
    use aether::config::Config;

    output::header("🎯", "Runtime Placement Decision");

    let config = Config::load();
    let workload = Workload::from_file(spec_path)?;
    let engine = ScoringEngine::new(config.engine);
    let result = engine.score(&workload);

    print!("{}", format_scoring_report_with_options(&result, explain));

    Ok(())
}

// ─── intent: evaluate intent-based runtime recommendation ────────────
pub(crate) async fn intent_command(spec_path: &PathBuf) -> Result<()> {
    use aether::ai::scoring::{format_scoring_report, ScoringEngine};
    use aether::config::Config;

    let workload = Workload::from_file(spec_path)?;

    let intent = match &workload.intent {
        Some(i) => i,
        None => {
            output::warning("No intent section found in workload spec.");
            output::info("Add an intent block to enable intent-based decisions:");
            output::muted(
                "  intent:\n    goal: low-latency\n    sla:\n      maxLatencyMs: 50\n    budget:\n      maxMonthlyUsd: 500",
            );
            output::muted("\nFalling back to standard recommendation.");
            let config = Config::load();
            let engine = ScoringEngine::new(config.engine);
            let result = engine.score(&workload);
            print!("{}", format_scoring_report(&result));
            return Ok(());
        }
    };

    output::header("🧠", "Intent-Based Runtime Decision");

    // Show intent summary
    let goal_str = format!("{:?}", intent.goal);
    let mut intent_pairs: Vec<(&str, String)> = vec![
        ("Goal", goal_str),
    ];
    if let Some(ref sla) = intent.sla {
        if let Some(latency) = sla.max_latency_ms {
            intent_pairs.push(("Max Latency", format!("{}ms", latency)));
        }
        if let Some(avail) = sla.min_availability_pct {
            intent_pairs.push(("Min Availability", format!("{:.2}%", avail)));
        }
    }
    if let Some(ref budget) = intent.budget {
        intent_pairs.push(("Budget Cap", format!("${:.0}/mo", budget.max_monthly_usd)));
    }
    if let Some(ref resilience) = intent.resilience {
        intent_pairs.push(("Resilience", format!("{:?}", resilience)));
    }
    if let Some(ref compliance) = intent.compliance {
        let mut flags = Vec::new();
        if compliance.isolation_required {
            flags.push("isolation");
        }
        if compliance.encryption_required {
            flags.push("encryption");
        }
        if !flags.is_empty() {
            intent_pairs.push(("Compliance", flags.join(", ")));
        }
    }
    println!("{}", output::property_table(&intent_pairs));

    // Run intent-aware scoring
    let config = Config::load();
    let engine = ScoringEngine::new(config.engine);
    let result = engine.score(&workload);

    output::section_with_icon("📊", "Intent-Aware Scoring Results");
    print!("{}", format_scoring_report(&result));

    // Show what intent changed
    output::section("Intent Influence");
    let base_engine = ScoringEngine::new(Config::load().engine);
    let mut base_workload = workload.clone();
    base_workload.intent = None;
    let base_result = base_engine.score(&base_workload);

    if result.recommended != base_result.recommended {
        output::change(
            "Recommended runtime",
            &format!("{}", base_result.recommended),
            &format!("{}", result.recommended),
        );
    } else {
        output::success(&format!(
            "Intent confirms base recommendation: {}",
            result.recommended
        ));
    }

    Ok(())
}

// ─── init: first-run onboarding wizard ───────────────────────────────
pub(crate) async fn init_command() -> Result<()> {
    output::banner("AETHER", "First-Time Setup Wizard");

    // Step 1: Detect available runtimes
    output::step(1, 4, "Detecting available runtimes");
    let mut capabilities = Vec::new();

    let podman_ok = which::which("podman").is_ok();
    capabilities.push(("Podman", podman_ok, if podman_ok { "Ready" } else { "Not found — install with: sudo dnf install podman" }));

    let kubectl_ok = which::which("kubectl").is_ok();
    capabilities.push(("Kubernetes (kubectl)", kubectl_ok, if kubectl_ok { "Ready" } else { "Not found — install kubectl" }));

    let virtctl_ok = which::which("virtctl").is_ok();
    capabilities.push(("KubeVirt (virtctl)", virtctl_ok, if virtctl_ok { "Ready" } else { "Not found — install virtctl" }));

    output::capabilities(&capabilities);

    // Step 2: Create config directory
    output::step(2, 4, "Initializing configuration");
    aether::state::StateStore::ensure_state_dir()?;
    output::success("Created ~/.aether/ directory");

    // Step 3: Generate sample workload in current directory
    output::step(3, 4, "Generating sample workload");
    let cwd = std::env::current_dir().unwrap_or_default();
    output::info(&format!("Target directory: {}", cwd.display()));
    let sample_path = cwd.join("workload.yaml");
    if sample_path.exists() {
        output::info("workload.yaml already exists, skipping");
    } else {
        let sample = r#"apiVersion: aether/v1
kind: Workload

metadata:
  name: hello-world
  owner: team
  project: demo

build:
  context: "."
  dockerfile: Dockerfile
  registry: localhost

requirements:
  cpu: "1"
  memory: 512Mi
  storage: 1Gi

runtime:
  preferred: auto
  allow: [container, kube]

network:
  service: true
  serviceType: ClusterIP
  ports:
    - containerPort: 8080
      servicePort: 8080
      protocol: TCP
"#;
        std::fs::write(sample_path, sample)?;
        output::success("Generated workload.yaml");
    }

    // Step 4: Summary
    output::step(4, 4, "Setup complete");

    let available_count = [podman_ok, kubectl_ok, virtctl_ok]
        .iter()
        .filter(|&&v| v)
        .count();

    output::summary_success(
        "Setup Complete",
        &[
            ("Runtimes available", format!("{}/3", available_count)),
            ("Config directory", "~/.aether/".to_string()),
            ("Sample workload", "workload.yaml".to_string()),
        ],
    );

    output::section("Next Steps");
    output::bullet_list(&[
        "aether validate          — Validate the sample workload",
        "aether recommend         — Get AI runtime recommendation",
        "aether compare           — Compare runtimes for your workload",
        "aether run               — Deploy the workload",
        "aether tui               — Launch interactive dashboard",
        "aether help-all          — View complete command reference",
    ]);

    Ok(())
}

// ─── Contextual error helpers ────────────────────────────────────────

// ─── compose: multi-workload compose file ────────────────────────────
pub(crate) async fn compose_command(action: ComposeAction) -> Result<()> {
    use aether::compose;

    match action {
        ComposeAction::Validate { file } => {
            output::header("📋", "Compose Validate");
            let spec = compose::load(&file)?;
            compose::validate(&spec)?;

            let order = compose::resolve_order(&spec)?;
            output::success(&format!(
                "Compose file valid: {} workloads",
                spec.workloads.len()
            ));
            output::section("Deploy Order");
            for (i, name) in order.iter().enumerate() {
                let wl = &spec.workloads[name];
                output::kv_tree(
                    &format!("{}. {}", i + 1, name),
                    &wl.spec.display().to_string(),
                    i == order.len() - 1,
                );
            }
            Ok(())
        }
        ComposeAction::Up { file, runtime, dry_run } => {
            output::header("🚀", "Compose Up");
            let spec = compose::load(&file)?;
            compose::validate(&spec)?;

            let order = compose::resolve_order(&spec)?;
            output::info(&format!(
                "Deploying {} workloads in dependency order",
                order.len()
            ));

            for (i, name) in order.iter().enumerate() {
                let wl = &spec.workloads[name];
                output::step(i + 1, order.len(), &format!("Deploying {}", name));

                if dry_run {
                    output::muted(&format!(
                        "  [dry-run] Would deploy {} from {}",
                        name,
                        wl.spec.display()
                    ));
                    continue;
                }

                let rt_override = wl.runtime.as_deref().or(runtime.as_deref());
                let workload = Workload::from_file(&wl.spec)?.with_env(&wl.env);
                deploy_single_workload(&workload, &wl.spec, rt_override, None).await?;
            }

            if dry_run {
                output::muted("\n[dry-run] No resources were created.");
            } else {
                output::success(&format!("All {} workloads deployed", order.len()));
            }
            Ok(())
        }
        ComposeAction::Down { file } => {
            output::header("🛑", "Compose Down");
            let spec = compose::load(&file)?;

            // Stop in reverse dependency order
            let order = compose::resolve_order(&spec)?;
            let reversed: Vec<_> = order.into_iter().rev().collect();

            for name in &reversed {
                output::info(&format!("Stopping {}", name));
                match stop_command(name, false).await {
                    Ok(()) => output::success(&format!("Stopped {}", name)),
                    Err(e) => output::warning(&format!("Could not stop {}: {}", name, e)),
                }
            }

            output::success("All workloads stopped");
            Ok(())
        }
    }
}

// ─── plugin: runtime plugin management ───────────────────────────────
pub(crate) async fn plugin_command(action: PluginAction) -> Result<()> {
    use aether::plugin::PluginRegistry;

    let path = PluginRegistry::default_path();

    match action {
        PluginAction::List => {
            output::header("🔌", "Registered Plugins");
            let reg = PluginRegistry::load(&path)?;
            if reg.plugins.is_empty() {
                output::muted("No plugins registered. Run `aether plugin discover` to scan for plugins.");
                return Ok(());
            }
            let rows: Vec<Vec<String>> = reg.plugins.values().map(|p| {
                vec![
                    p.name.clone(),
                    p.version.clone(),
                    p.runtime_kind.clone(),
                    p.capabilities.join(", "),
                ]
            }).collect();
            println!("{}", output::table(&["Name", "Version", "Runtime", "Capabilities"], rows));
            Ok(())
        }
        PluginAction::Discover => {
            output::header("🔍", "Plugin Discovery");
            let sp = output::spinner("Scanning ~/.aether/plugins/...");
            let mut reg = PluginRegistry::load(&path)?;
            let count = reg.discover()?;
            reg.save(&path)?;
            output::spinner_success(&sp, &format!("Found {} plugin(s)", count));

            if count > 0 {
                let rows: Vec<Vec<String>> = reg.plugins.values().map(|p| {
                    vec![p.name.clone(), p.runtime_kind.clone(), p.command.clone()]
                }).collect();
                println!("{}", output::table(&["Name", "Runtime", "Command"], rows));
            }
            Ok(())
        }
        PluginAction::Register { manifest } => {
            output::header("📦", "Register Plugin");
            let content = std::fs::read_to_string(&manifest)?;
            let plugin: aether::plugin::PluginManifest = serde_json::from_str(&content)?;
            let mut reg = PluginRegistry::load(&path)?;
            let name = plugin.name.clone();
            reg.register(plugin);
            reg.save(&path)?;
            output::success(&format!("Registered plugin '{}'", name));
            Ok(())
        }
        PluginAction::Remove { name } => {
            let mut reg = PluginRegistry::load(&path)?;
            match reg.unregister(&name) {
                Some(_) => {
                    reg.save(&path)?;
                    output::success(&format!("Removed plugin '{}'", name));
                }
                None => {
                    output::warning(&format!("Plugin '{}' not found", name));
                }
            }
            Ok(())
        }
    }
}

// ─── health: view health history and uptime ──────────────────────────
pub(crate) async fn health_command(name: &str, last: usize, summary_only: bool) -> Result<()> {
    use aether::health::HealthHistory;

    let health_path = HealthHistory::default_path();
    let history = HealthHistory::load(&health_path)?;

    if summary_only {
        let summary = history.summary(name);

        if output::is_json() {
            println!("{}", serde_json::to_string_pretty(&summary)?);
            return Ok(());
        }

        output::header("💓", &format!("Health Summary: {}", name));

        if summary.total_checks == 0 {
            output::muted(&format!(
                "No health data for '{}'. Run `aether status {}` to record a check.",
                name, name
            ));
            return Ok(());
        }

        let uptime_color = if summary.uptime_percent >= 99.0 { "🟢" }
            else if summary.uptime_percent >= 95.0 { "🟡" }
            else { "🔴" };

        println!(
            "{}",
            output::property_table(&[
                ("Total Checks", format!("{}", summary.total_checks)),
                ("Ready Checks", format!("{}", summary.ready_checks)),
                ("Uptime", format!("{} {:.2}%", uptime_color, summary.uptime_percent)),
                ("Last State", summary.last_state.clone()),
                ("Last Restart Count", format!("{}", summary.last_restart_count)),
            ])
        );
        return Ok(());
    }

    // Timeline view
    output::header("💓", &format!("Health Timeline: {} (last {})", name, last));

    let timeline = history.timeline(name, last);

    if timeline.is_empty() {
        output::muted(&format!(
            "No health data for '{}'. Run `aether status {}` to record a check.",
            name, name
        ));
        return Ok(());
    }

    if output::is_json() {
        println!("{}", serde_json::to_string_pretty(&timeline)?);
        return Ok(());
    }

    let rows: Vec<Vec<String>> = timeline
        .iter()
        .map(|r| {
            let ready_icon = if r.ready { "●" } else { "○" };
            let latency = r.latency_ms
                .map(|ms| format!("{:.0}ms", ms))
                .unwrap_or_else(|| "-".to_string());
            vec![
                r.timestamp.chars().take(19).collect::<String>(),
                format!("{}", r.state),
                ready_icon.to_string(),
                format!("{}", r.restart_count),
                latency,
            ]
        })
        .collect();

    println!(
        "{}",
        output::table(&["Timestamp", "State", "Ready", "Restarts", "Latency"], rows)
    );

    // Show summary at bottom
    let summary = history.summary(name);
    output::muted(&format!(
        "\n  Uptime: {:.2}%  |  {} / {} checks ready  |  {} restarts",
        summary.uptime_percent, summary.ready_checks, summary.total_checks, summary.last_restart_count
    ));

    Ok(())
}

pub(crate) async fn health_collect_command() -> Result<()> {
    output::section_with_icon("\u{1F493}", "Collecting Health Data");

    let state = StateStore::load(&StateStore::default_path())?;
    let workloads = state.list();

    if workloads.is_empty() {
        output::info("No workloads to check");
        return Ok(());
    }

    let health_path = aether::health::HealthHistory::default_path();
    let mut history = aether::health::HealthHistory::load(&health_path).unwrap_or_default();
    let mut collected = 0u32;

    for ws in &workloads {
        match aether::runtime::create_runtime_ns(&ws.runtime, get_namespace()).await {
            Ok(rt) => {
                match rt.status(&ws.instance).await {
                    Ok(status) => {
                        let record = aether::health::record_from_status(
                            &ws.name,
                            ws.runtime,
                            &status,
                        );
                        history.record(record);
                        collected += 1;
                    }
                    Err(e) => {
                        tracing::debug!("Failed to get status for '{}': {}", ws.name, e);
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Failed to create runtime for '{}': {}", ws.name, e);
            }
        }
    }

    history.save(&health_path)?;
    output::success(&format!("Collected health data for {} workloads", collected));
    Ok(())
}

pub(crate) async fn gitops_command(action: GitOpsAction) -> Result<()> {
    use aether::gitops::{format_status, GitOpsConfig, GitOpsController};

    match action {
        GitOpsAction::Init { repo, branch } => {
            let sp = output::spinner("Initializing GitOps repository...");

            let config = GitOpsConfig {
                repo_url: repo.clone(),
                branch: branch.clone(),
                ..Default::default()
            };

            let mut ctrl = GitOpsController::new(config);
            ctrl.init_repo()?;

            output::spinner_success(&sp, "GitOps repository initialized");
            output::success(&format!("Repository: {}", repo));
            output::success(&format!("Branch: {}", branch));
            output::success(&format!("Local path: {}", ctrl.repo_dir.display()));

            emit_event(
                aether::events::EventSeverity::Info,
                aether::events::EventCategory::Deployment,
                "cli",
                None,
                "GitOps initialized",
                &format!("Repository {} (branch {}) initialized", repo, branch),
            );

            Ok(())
        }
        GitOpsAction::Status => {
            // Load saved config from state dir, or show unconfigured status
            let state_path = aether::resources::aether_path("gitops.json");
            if state_path.exists() {
                let data = std::fs::read_to_string(&state_path)?;
                let status: aether::gitops::GitOpsStatus = serde_json::from_str(&data)?;
                print!("{}", format_status(&status));
            } else {
                output::info("GitOps is not configured. Run `aether git-ops init --repo <URL>` to get started.");
            }
            Ok(())
        }
        GitOpsAction::Sync => {
            let state_path = aether::resources::aether_path("gitops.json");
            if !state_path.exists() {
                anyhow::bail!(
                    "GitOps is not configured. Run `aether git-ops init --repo <URL>` first."
                );
            }

            let data = std::fs::read_to_string(&state_path)?;
            let config: GitOpsConfig = serde_json::from_str(&data)?;

            let sp = output::spinner("Syncing from Git repository...");
            let mut ctrl = GitOpsController::new(config);
            let changes = ctrl.sync()?;
            output::spinner_success(&sp, "Sync complete");

            if changes.is_empty() {
                output::info("No workload changes detected.");
            } else {
                output::success(&format!("{} change(s) detected:", changes.len()));
                for change in &changes {
                    output::info(&format!("  [{}] {}", change.change_type, change.file_path));
                }
            }

            // Persist updated status
            let status_json = serde_json::to_string_pretty(ctrl.status())?;
            std::fs::write(&state_path, status_json)?;

            Ok(())
        }
    }
}

pub(crate) async fn helm_export_command(
    spec_path: &PathBuf,
    output_dir: &Path,
    chart_version: Option<&str>,
) -> Result<()> {
    let workload = Workload::from_file(spec_path)?;
    aether::helm::export_helm_chart(&workload, output_dir, chart_version)?;
    output::success(&format!("Helm chart exported to {}", output_dir.display()));
    Ok(())
}

pub(crate) async fn confidential_command(action: ConfidentialAction, spec_path: &PathBuf) -> Result<()> {
    use aether::ragnarok::client::RagnarokClient;
    use aether::ragnarok::guestkit::{GuestKitRequest, GuestKitService, InspectionMode};
    use aether::ragnarok::image::ImageCatalog;
    use aether::ragnarok::isolation::{self, IsolationPolicy};
    use crate::cli::GuestKitAction;

    let data_dir = RagnarokClient::attestation_data_dir();

    match action {
        ConfidentialAction::Image { action } => {
            let catalog = ImageCatalog::load(&data_dir);
            match action {
                ConfidentialImageAction::List => {
                    let items = catalog.list();
                    if items.is_empty() {
                        output::info("No measured images in catalog.");
                    } else {
                        for m in items {
                            println!(
                                "{}  digest={}  signed={}  key={}",
                                m.name,
                                m.launch_digest.as_deref().unwrap_or(&m.image_hash),
                                m.signed_at,
                                m.signing_key_id
                            );
                        }
                    }
                }
                ConfidentialImageAction::Sign { name, path, key } => {
                    let manifest = catalog.sign(&name, &path, &key)?;
                    output::success(&format!(
                        "Signed '{}' launch_digest={}",
                        manifest.name,
                        manifest.launch_digest.as_deref().unwrap_or(&manifest.image_hash)
                    ));
                }
                ConfidentialImageAction::Verify { name, path } => {
                    let ok = catalog.verify(&name, &path)?;
                    if ok {
                        output::success(&format!("Image '{name}' matches catalog"));
                    } else {
                        anyhow::bail!("Image '{name}' hash mismatch");
                    }
                }
                ConfidentialImageAction::VerifyDigest { digest } => {
                    if catalog.verify_digest(&digest) {
                        output::success(&format!("Digest '{digest}' found in catalog"));
                    } else {
                        anyhow::bail!("Digest '{digest}' not in verified catalog");
                    }
                }
            }
        }
        ConfidentialAction::Guestkit { action } => {
            let catalog = ImageCatalog::load(&data_dir);
            let svc = GuestKitService::new(data_dir);
            match action {
                GuestKitAction::Inspect {
                    vm_id,
                    image,
                    mode,
                    policy,
                } => {
                    let policy_manifest = match policy {
                        Some(p) if std::path::Path::new(&p).exists() => {
                            Some(std::fs::read_to_string(&p)?)
                        }
                        Some(p) => Some(p),
                        None => None,
                    };
                    let expected_digest = Workload::from_file(spec_path)
                        .ok()
                        .and_then(|w| {
                            w.confidential
                                .as_ref()
                                .and_then(|c| c.image_digest.clone())
                        });
                    let result = svc.inspect(
                        &GuestKitRequest {
                            vm_id: vm_id.clone(),
                            image_path: image.map(|p| p.display().to_string()),
                            mode: InspectionMode::parse(&mode),
                            policy_manifest,
                            expected_digest,
                        },
                        &catalog,
                    )?;
                    if result.passed {
                        output::success(&format!(
                            "GuestKit {} inspection passed for '{}'",
                            result.mode.as_str(),
                            vm_id
                        ));
                    } else {
                        for f in &result.findings {
                            output::error(f);
                        }
                        anyhow::bail!("GuestKit inspection failed for '{vm_id}'");
                    }
                    for f in &result.findings {
                        output::info(f);
                    }
                    if !result.repair_steps.is_empty() {
                        output::info("Repair playbook:");
                        for step in &result.repair_steps {
                            println!("  - {step}");
                        }
                    }
                }
                GuestKitAction::History { vm_id } => {
                    let history = svc.history(&vm_id);
                    if history.is_empty() {
                        output::info(&format!("No GuestKit inspections for '{vm_id}'"));
                    } else {
                        for entry in history {
                            println!(
                                "{}  mode={}  passed={}  findings={}",
                                entry.inspected_at,
                                entry.mode.as_str(),
                                entry.passed,
                                entry.findings.join("; ")
                            );
                        }
                    }
                }
            }
        }
        ConfidentialAction::IsolationCheck => {
            let workload = Workload::from_file(spec_path)?;
            let policy = IsolationPolicy::from_env();
            let verdict = isolation::evaluate(&workload, &policy);
            if verdict.compliant {
                output::success("Isolation policy: compliant");
            } else {
                for v in &verdict.violations {
                    output::error(v);
                }
                anyhow::bail!("isolation policy violated");
            }
            if !verdict.scheduler_hints.is_empty() {
                output::info("Scheduler hints:");
                for (k, v) in &verdict.scheduler_hints {
                    println!("  {k}={v}");
                }
            }
        }
        ConfidentialAction::SovereignCheck => {
            let workload = Workload::from_file(spec_path)?;
            let config = aether::ragnarok::sovereign::SovereignConfig::from_env();
            let verdict = aether::ragnarok::sovereign::evaluate(&workload, &config);
            if verdict.compliant {
                output::success("Sovereign policy: compliant");
            } else {
                for v in &verdict.violations {
                    output::error(v);
                }
                anyhow::bail!("sovereign policy violated");
            }
            for h in &verdict.hints {
                output::info(h);
            }
        }
        ConfidentialAction::Placement => {
            use aether::ragnarok::scheduling;
            let workload = Workload::from_file(spec_path)?;
            let advice = scheduling::placement_advice(&workload);
            println!(
                "{}  runtime={}  tee={}  host_tee_ready={}",
                advice.workload,
                advice.recommended_runtime,
                advice.tee.as_deref().unwrap_or("—"),
                advice.host_tee_ready
            );
            if let Some(ref rc) = advice.kata_runtime_class {
                println!("  kata_runtime_class={rc}");
            }
            if !advice.schedule_constraints.is_empty() {
                output::info("Schedule constraints:");
                for c in &advice.schedule_constraints {
                    println!("  - {c}");
                }
            }
            if !advice.gitops_issues.is_empty() {
                output::info("GitOps / policy issues:");
                for issue in &advice.gitops_issues {
                    output::error(issue);
                }
                anyhow::bail!("confidential placement blocked by policy");
            }
            if advice.blockers.is_empty() {
                output::success(&format!(
                    "Placement ready (score bonus {:.0}%)",
                    advice.placement_score_bonus * 100.0
                ));
            } else {
                for b in &advice.blockers {
                    output::error(b);
                }
                anyhow::bail!("confidential placement blocked");
            }
        }
        ConfidentialAction::Migration { action } => {
            use aether::ragnarok::migration::{
                plan_confidential_migration_tee, ConfidentialMigrationStore,
            };
            use crate::cli::ConfidentialMigrationAction;
            let workload = Workload::from_file(spec_path)?;
            let host = aether::ragnarok::probe_host_tee();
            match action {
                ConfidentialMigrationAction::Plan { target } => {
                    let plan = plan_confidential_migration_tee(
                        &workload,
                        host.sev_snp,
                        host.sev_snp,
                        host.tdx,
                        host.tdx,
                    );
                    output::info(&format!(
                        "Strategy: {} | encrypted: {}",
                        plan.recommended_strategy, plan.encrypted_channel_required
                    ));
                    output::info(&format!("Migration URI: {}", plan.encrypted_migration_uri));
                    if !plan.blockers.is_empty() {
                        for b in &plan.blockers {
                            output::error(b);
                        }
                        anyhow::bail!("migration plan has blockers");
                    }
                    for phase in &plan.phases {
                        println!("  phase: {phase}");
                    }
                    for hint in &plan.hyper2kvm_hints {
                        println!("  hyper2kvm: {hint}");
                    }
                    if !target.is_empty() {
                        println!("  target: {target}");
                    }
                }
                ConfidentialMigrationAction::Status { name } => {
                    let vm = name
                        .clone()
                        .unwrap_or_else(|| workload.metadata.name.clone());
                    let store =
                        ConfidentialMigrationStore::new(&data_dir);
                    match store.get(&vm) {
                        Some(rec) => {
                            println!(
                                "{} phase={:?} cutover_ready={} uri={}",
                                rec.workload, rec.phase, rec.cutover_ready, rec.migration_uri
                            );
                            if let Some(e) = &rec.error {
                                output::error(e);
                            }
                        }
                        None => output::info(&format!("No migration record for '{vm}'")),
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) async fn sbom_command(action: crate::cli::SbomAction) -> Result<()> {
    use crate::cli::SbomAction;
    match action {
        SbomAction::Export { output } => {
            let bom = aether::sbom::generate_cyclonedx(
                std::env::current_exe().ok().as_deref(),
            )?;
            let text = serde_json::to_string_pretty(&bom)?;
            if let Some(path) = output {
                aether::sbom::export_to_path(&path, std::env::current_exe().ok().as_deref())?;
                output::success(&format!("SBOM written to {}", path.display()));
            } else {
                println!("{text}");
            }
        }
        SbomAction::Verify { file } => {
            let ok = aether::sbom::verify_file(&file)?;
            if ok {
                output::success(&format!("Valid CycloneDX SBOM: {}", file.display()));
            } else {
                anyhow::bail!("invalid SBOM format: {}", file.display());
            }
        }
    }
    Ok(())
}

pub(crate) async fn edge_agent_command(
    control_plane: &str,
    site: &str,
    token: Option<&str>,
    kube_context: Option<&str>,
    dry_run: bool,
    interval_secs: u64,
) -> Result<()> {
    use aether::fleet::edge::{EdgeHeartbeatRequest, EdgeRegisterRequest};
    use std::time::Duration;

    let base = control_plane.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("reqwest client")?;

    let token_owned = token
        .map(str::to_string)
        .or_else(|| std::env::var("AETHER_EDGE_TOKEN").ok())
        .filter(|s| !s.is_empty());

    let register_body = EdgeRegisterRequest {
        site: site.to_string(),
        kube_context: kube_context.map(str::to_string),
        labels: std::collections::HashMap::new(),
    };

    if dry_run {
        output::info(&format!(
            "edge-agent dry-run: would register {site} at {base}/api/fleet/edge/register"
        ));
        return Ok(());
    }

    let mut req = client
        .post(format!("{base}/api/fleet/edge/register"))
        .json(&register_body);
    if let Some(ref t) = token_owned {
        req = req.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
    }
    req.send()
        .await
        .context("edge register")?
        .error_for_status()
        .context("edge register status")?;

    output::success(&format!("Edge agent '{site}' registered with {base}"));

    loop {
        let mut poll = client.get(format!("{base}/api/fleet/edge/queue?site={site}"));
        if let Some(ref t) = token_owned {
            poll = poll.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
        }
        if let Ok(resp) = poll.send().await {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(jobs) = body.pointer("/data").and_then(|d| d.as_array()) {
                    for job in jobs {
                        let action = job.get("action").and_then(|v| v.as_str()).unwrap_or("?");
                        let id = job.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        output::info(&format!("Edge job {id}: {action}"));
                        if action == "gitops_sync" {
                            let _ = gitops_command(crate::cli::GitOpsAction::Sync).await;
                        } else if action == "stop" {
                            if let Some(name) = job.pointer("/payload/workload").and_then(|v| v.as_str()) {
                                let _ = stop_command(name, false).await;
                            }
                        }
                    }
                }
            }
        }

        let mut hb = client
            .post(format!("{base}/api/fleet/edge/heartbeat"))
            .json(&EdgeHeartbeatRequest {
                site: site.to_string(),
                queue_depth: 0,
                last_error: None,
            });
        if let Some(ref t) = token_owned {
            hb = hb.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
        }
        if let Err(e) = hb.send().await {
            output::error(&format!("heartbeat failed: {e}"));
        }

        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}

/// Wrap an error with a contextual suggestion for the user.
pub(crate) fn suggest_on_error(err: anyhow::Error) -> anyhow::Error {
    let msg = err.to_string();

    if msg.contains("not found") && (msg.contains("Workload") || msg.contains("workload")) {
        return err.context(
            "Hint: Run `aether list` to see deployed workloads, \
             or `aether run` to deploy one first."
        );
    }
    if msg.contains("Podman not found") {
        return err.context(
            "Hint: Install podman with `sudo dnf install podman` (Fedora) \
             or `sudo apt install podman` (Debian/Ubuntu)."
        );
    }
    if msg.contains("error trying to connect") || msg.contains("connection refused") {
        return err.context(
            "Hint: Check your Kubernetes cluster with `kubectl cluster-info`, \
             or set KUBECONFIG to point to a valid kubeconfig file."
        );
    }
    if msg.contains("Unknown runtime") {
        return err.context(
            "Hint: Valid runtimes are: podman, kubernetes (kube/k8s), kubevirt (vm), metal3 (metal/bare-metal)."
        );
    }

    err
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether::runtime::RuntimeKind;
    use aether::spec::*;
    use std::collections::HashMap;

    // ---------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------

    fn make_valid_workload() -> Workload {
        Workload {
            api_version: "aether/v1".to_string(),
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
            ..Default::default()
            },
            requirements: ResourceRequirements {
                cpu: "2".to_string(),
                memory: "4Gi".to_string(),
                storage: "20Gi".to_string(),
                gpu: None,
                cpu_request: None,
                memory_request: None,
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
            mesh: None,
            intent: None,
            autonomy: None,
            confidential: None,
            schedule: None,
        kubernetes: None,
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
        let path = PathBuf::from("/tmp/nonexistent_aether_test.yaml");
        let result = validate_command(&path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_command_invalid_spec() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.yaml");
        let mut spec = make_valid_workload();
        spec.api_version = "aether/v99".to_string();
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

    #[tokio::test]
    async fn test_cost_command_json_output() {
        output::set_json(true);
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = cost_command(&path, "aws").await;
        output::set_json(false);
        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------
    // recommend_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_recommend_command() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_valid_spec(dir.path());
        let result = recommend_command(&path).await;
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
        let path = PathBuf::from("/tmp/nonexistent_aether_policy.yaml");
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
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_command_web_app_stdout() {
        let result = template_command(
            "web-app",
            Some("my-web".to_string()),
            "team",
            "demo",
            "ghcr.io/org",
            None,
            false,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_command_rest_api_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("api.yaml");
        let result = template_command(
            "rest-api",
            Some("my-api".to_string()),
            "team",
            "demo",
            "ghcr.io/org",
            Some(out.clone()),
            false,
        )
        .await;
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
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_command_unknown() {
        let result = template_command(
            "unknown-kind", None, "team", "demo", "ghcr.io/org", None, false,
        )
        .await;
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
        let store = aether::state::StateStore::new();
        store.save(&state_path).unwrap();

        // list_command uses StateStore::default_path() so we can't easily
        // redirect it. Instead, test the underlying StateStore directly.
        let loaded = aether::state::StateStore::load(&state_path).unwrap();
        assert!(loaded.list().is_empty());
    }

    // ---------------------------------------------------------------
    // metrics_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_metrics_gather_not_empty() {
        aether::metrics::init();
        let output = aether::metrics::gather();
        // Metrics output should contain at least the HELP/TYPE headers
        assert!(!output.is_empty());
    }

    // ---------------------------------------------------------------
    // config_command
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_config_show() {
        // The show branch serializes Config to YAML and prints it
        let config = aether::config::Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("engine"));
        assert!(yaml.contains("migration"));
        assert!(yaml.contains("scaling"));
    }

    #[tokio::test]
    async fn test_config_init_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        let config = aether::config::Config::default();
        config.save_to(&path).unwrap();
        assert!(path.exists());

        let loaded = aether::config::Config::load_from(&path);
        assert_eq!(loaded.engine.enable_scoring, config.engine.enable_scoring);
    }

    #[tokio::test]
    async fn test_config_load_nonexistent_returns_default() {
        let path = PathBuf::from("/tmp/nonexistent_aether_config.yaml");
        let config = aether::config::Config::load_from(&path);
        let default = aether::config::Config::default();
        assert_eq!(config.engine.enable_scoring, default.engine.enable_scoring);
    }

    // ---------------------------------------------------------------
    // audit (underlying AuditLog)
    // ---------------------------------------------------------------

    #[tokio::test]
    async fn test_audit_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.json");
        let log = aether::audit::AuditLog::new();
        log.save(&path).unwrap();

        let loaded = aether::audit::AuditLog::load(&path).unwrap();
        let summary = loaded.summary();
        assert_eq!(summary.total_events, 0);
    }

    #[tokio::test]
    async fn test_audit_record_and_summary() {
        use aether::audit::{ActionResult, AuditAction, AuditLog};

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
        use aether::events::{EventBus, EventCategory, EventSeverity};

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
        use aether::events::{EventBus, EventCategory, EventSeverity};

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
        use aether::runtime::Instance;

        let mut store = aether::state::StateStore::new();

        let ws = aether::state::WorkloadState {
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
            os_version: None,
            node_labels: vec![],
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
        use aether::runtime::Instance;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        let mut store = aether::state::StateStore::new();
        store.upsert(
            "svc".to_string(),
            aether::state::WorkloadState {
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
            os_version: None,
            node_labels: vec![],
            },
        );
        store.save(&path).unwrap();

        let loaded = aether::state::StateStore::load(&path).unwrap();
        assert_eq!(loaded.list().len(), 1);
        assert_eq!(loaded.get("svc").unwrap().name, "svc");
    }

    // ---------------------------------------------------------------
    // Template generation for each kind
    // ---------------------------------------------------------------

    #[test]
    fn test_template_generate_all_kinds() {
        use aether::templates::{generate, TemplateKind, TemplateParams};

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
            assert_eq!(spec.api_version, "aether/v1");
            assert_eq!(spec.kind, "Workload");
            assert!(
                spec.validate().is_ok(),
                "template {:?} produced invalid spec",
                kind
            );
        }
    }

    // ---------------------------------------------------------------
    // Cost estimation (library-level)
    // ---------------------------------------------------------------

    #[test]
    fn test_cost_estimate_all_providers() {
        let spec = make_valid_workload();
        let estimates = aether::cost::estimate_all_providers(&spec).unwrap();
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
        let engine = aether::policy::PolicyEngine::production();
        let spec = make_valid_workload();
        let result = engine.evaluate(&spec);
        // Result should have at least some rules evaluated
        assert!(result.policies_evaluated > 0);
    }

    #[test]
    fn test_policy_development_evaluates() {
        let engine = aether::policy::PolicyEngine::development();
        let spec = make_valid_workload();
        let result = engine.evaluate(&spec);
        assert!(result.policies_evaluated > 0);
    }

    // ---------------------------------------------------------------
    // Scoring engine (library-level)
    // ---------------------------------------------------------------

    #[test]
    fn test_scoring_engine_recommends_runtime() {
        let engine = aether::ai::scoring::ScoringEngine::with_defaults();
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
        let profiler = aether::ai::profiler::Profiler::with_defaults();
        let spec = make_valid_workload();
        let profile = profiler.profile(&spec, None);
        assert_eq!(profile.name, "test-app");
        assert!(profile.optimization_score >= 0.0 && profile.optimization_score <= 100.0);
    }

    #[test]
    fn test_profiler_with_runtime() {
        let profiler = aether::ai::profiler::Profiler::with_defaults();
        let spec = make_valid_workload();
        let profile = profiler.profile(&spec, Some(RuntimeKind::Podman));
        assert_eq!(profile.name, "test-app");
    }

    // ---------------------------------------------------------------
    // Scaling engine (library-level)
    // ---------------------------------------------------------------

    #[test]
    fn test_scaling_engine_recommend() {
        use aether::ai::scaling::{ScalingEngine, TimeSeries};

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
        use aether::drift::{format_live_diff, DiffRow, LiveDiffReport};

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
        use aether::events::{ChannelType, EventBus, EventSeverity, NotificationChannel};

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
        let ch = bus
            .channels()
            .iter()
            .find(|c| c.name == "slack-ops")
            .unwrap();
        assert!(ch.enabled);
        assert_eq!(ch.min_severity, EventSeverity::Warning);

        // Remove channel
        assert!(bus.remove_channel("slack-ops"));
        assert_eq!(bus.channels().len(), initial);

        // Remove non-existent channel
        assert!(!bus.remove_channel("nonexistent"));
    }

    // ---------------------------------------------------------------
    // discover_workloads
    // ---------------------------------------------------------------

    #[test]
    fn test_discover_workloads_valid_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Write two valid specs with distinct names
        let mut spec_a = make_valid_workload();
        spec_a.metadata.name = "alpha-svc".to_string();
        let mut spec_b = make_valid_workload();
        spec_b.metadata.name = "beta-svc".to_string();

        std::fs::write(
            dir.path().join("alpha.yaml"),
            serde_yaml::to_string(&spec_a).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("beta.yml"),
            serde_yaml::to_string(&spec_b).unwrap(),
        )
        .unwrap();

        let result = discover_workloads(dir.path()).unwrap();
        assert_eq!(result.len(), 2);
        let names: Vec<&str> = result
            .iter()
            .map(|(_, w)| w.metadata.name.as_str())
            .collect();
        assert!(names.contains(&"alpha-svc"));
        assert!(names.contains(&"beta-svc"));
    }

    #[test]
    fn test_discover_workloads_skips_invalid() {
        let dir = tempfile::tempdir().unwrap();
        // One valid, one invalid YAML, one non-YAML file
        let spec = make_valid_workload();
        std::fs::write(
            dir.path().join("good.yaml"),
            serde_yaml::to_string(&spec).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.path().join("bad.yaml"), "not: valid: [[[").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();

        let result = discover_workloads(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.metadata.name, "test-app");
    }

    #[test]
    fn test_discover_workloads_empty_dir_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = discover_workloads(dir.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("No valid workload specs found"));
    }

    #[test]
    fn test_discover_workloads_not_a_dir_error() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let result = discover_workloads(&file_path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("is not a directory"));
    }

    // ---------------------------------------------------------------
    // order_workloads_by_deps
    // ---------------------------------------------------------------

    #[test]
    fn test_order_workloads_with_graph() {
        use aether::dependencies::DependencyGraph;

        let mut graph = DependencyGraph::new();
        // api depends on db
        graph.add_dependency("api-svc", "db-svc");

        let mut db = make_valid_workload();
        db.metadata.name = "db-svc".to_string();
        let mut api = make_valid_workload();
        api.metadata.name = "api-svc".to_string();

        // Provide in reverse order to prove reordering works
        let workloads = vec![
            (PathBuf::from("api.yaml"), api),
            (PathBuf::from("db.yaml"), db),
        ];

        let ordered = order_workloads_by_deps(workloads, &graph);
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].1.metadata.name, "db-svc");
        assert_eq!(ordered[1].1.metadata.name, "api-svc");
    }

    #[test]
    fn test_order_workloads_without_graph() {
        use aether::dependencies::DependencyGraph;

        let graph = DependencyGraph::new();

        let mut a = make_valid_workload();
        a.metadata.name = "beta-svc".to_string();
        let mut b = make_valid_workload();
        b.metadata.name = "alpha-svc".to_string();

        let workloads = vec![
            (PathBuf::from("beta.yaml"), a),
            (PathBuf::from("alpha.yaml"), b),
        ];

        let ordered = order_workloads_by_deps(workloads, &graph);
        // No graph entries, so remaining are sorted alphabetically by name
        assert_eq!(ordered[0].1.metadata.name, "alpha-svc");
        assert_eq!(ordered[1].1.metadata.name, "beta-svc");
    }

    #[test]
    fn test_order_workloads_partial_graph() {
        use aether::dependencies::DependencyGraph;

        let mut graph = DependencyGraph::new();
        // Only db-svc is in the graph
        graph.add_workload("db-svc");

        let mut db = make_valid_workload();
        db.metadata.name = "db-svc".to_string();
        let mut web = make_valid_workload();
        web.metadata.name = "web-svc".to_string();
        let mut cache = make_valid_workload();
        cache.metadata.name = "cache-svc".to_string();

        let workloads = vec![
            (PathBuf::from("web.yaml"), web),
            (PathBuf::from("cache.yaml"), cache),
            (PathBuf::from("db.yaml"), db),
        ];

        let ordered = order_workloads_by_deps(workloads, &graph);
        assert_eq!(ordered.len(), 3);
        // db-svc is in the graph, so it comes first
        assert_eq!(ordered[0].1.metadata.name, "db-svc");
        // Remaining sorted alphabetically
        assert_eq!(ordered[1].1.metadata.name, "cache-svc");
        assert_eq!(ordered[2].1.metadata.name, "web-svc");
    }
}
