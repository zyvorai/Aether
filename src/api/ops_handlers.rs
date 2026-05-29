// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! REST wrappers for CLI-only operations (GitOps init, env/SLA mutations, orchestrator actions, etc.).

use super::handlers::{err_bad_request, err_internal, err_not_found, ok_json, persist_workload_api};
use super::types::*;
use crate::backup::{Backup, SnapshotManager};
use crate::orchestrator::{HealthStatus as OrcHealthStatus, Orchestrator};
use crate::runtime::{self, InstanceState, RuntimeKind};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use axum::{
    extract::{Path, Query, State as AxumState},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub(crate) struct GitOpsInitRequest {
    pub repo: String,
    #[serde(default = "default_gitops_branch")]
    pub branch: String,
}

fn default_gitops_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Deserialize)]
pub(crate) struct EnvCreateRequest {
    pub name: String,
    #[serde(default = "default_env_tier")]
    pub tier: String,
}

fn default_env_tier() -> String {
    "development".to_string()
}

#[derive(Debug, Deserialize)]
pub(crate) struct EnvPromoteRequest {
    pub workload: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EnvParityQuery {
    pub env1: String,
    pub env2: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlaAddRequest {
    pub workload: String,
    #[serde(default = "default_sla_tier")]
    pub tier: String,
}

fn default_sla_tier() -> String {
    "standard".to_string()
}

#[derive(Debug, Deserialize)]
pub(crate) struct OrchestratorRegisterRequest {
    pub name: String,
    pub runtime: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OrchestratorNameRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OrchestratorRollingUpdateRequest {
    pub name: String,
    pub replicas: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RollbackRequest {
    pub version: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HelmExportRequest {
    pub yaml: String,
    #[serde(default)]
    pub chart_version: Option<String>,
}

fn sla_path() -> PathBuf {
    crate::resources::aether_path("sla.json")
}

fn load_sla_engine() -> Result<crate::sla::SlaEngine, String> {
    use crate::sla::{SlaEngine, SlaTarget};
    let path = sla_path();
    let mut engine = SlaEngine::new();
    if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let targets: Vec<SlaTarget> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        for target in targets {
            engine.add_target(target);
        }
    }
    Ok(engine)
}

fn save_sla_engine(engine: &crate::sla::SlaEngine) -> Result<(), String> {
    if let Some(parent) = sla_path().parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let targets = engine.list_targets();
    let content = serde_json::to_string_pretty(&targets).map_err(|e| e.to_string())?;
    std::fs::write(sla_path(), content).map_err(|e| e.to_string())
}

async fn collect_orchestrator_health(
    orch: &Orchestrator,
    state: &StateStore,
) -> HashMap<String, OrcHealthStatus> {
    let managed = orch.list_workloads();
    let mut statuses = HashMap::new();
    let mut by_runtime: HashMap<RuntimeKind, Vec<(String, crate::runtime::Instance)>> =
        HashMap::new();

    for mw in &managed {
        if let Some(ws) = state.get(&mw.name) {
            by_runtime
                .entry(ws.runtime)
                .or_default()
                .push((mw.name.clone(), ws.instance.clone()));
        }
    }

    for (kind, workloads) in by_runtime {
        match runtime::create_runtime(&kind).await {
            Ok(rt) => {
                for (name, instance) in workloads {
                    let hs = match rt.status(&instance).await {
                        Ok(status) => match status.state {
                            InstanceState::Running if status.ready => OrcHealthStatus::Healthy,
                            InstanceState::Running => OrcHealthStatus::Degraded,
                            InstanceState::Failed => OrcHealthStatus::Unhealthy,
                            _ => OrcHealthStatus::Unknown,
                        },
                        Err(e) => {
                            tracing::warn!("health status for {name}: {e}");
                            OrcHealthStatus::Unknown
                        }
                    };
                    statuses.insert(name, hs);
                }
            }
            Err(e) => {
                tracing::warn!("runtime {kind}: {e}");
                for (name, _) in workloads {
                    statuses.insert(name, OrcHealthStatus::Unknown);
                }
            }
        }
    }

    statuses
}

/// POST /api/gitops/init — Clone repo and persist GitOps configuration.
pub(crate) async fn api_gitops_init(Json(req): Json<GitOpsInitRequest>) -> impl IntoResponse {
    if req.repo.trim().is_empty() {
        return err_bad_request::<serde_json::Value>("repo is required");
    }

    let config = crate::gitops::GitOpsConfig {
        repo_url: req.repo.clone(),
        branch: req.branch.clone(),
        ..Default::default()
    };

    let mut ctrl = crate::gitops::GitOpsController::new(config);
    if let Err(e) = ctrl.init_repo() {
        return err_internal::<serde_json::Value>(e);
    }

    let state_path = crate::resources::aether_path("gitops.json");
    let mut status = ctrl.status().clone();
    status.configured = true;
    status.repo_url = req.repo;
    status.branch = req.branch;
    if let Some(parent) = state_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return err_internal::<serde_json::Value>(e);
        }
    }
    match serde_json::to_string_pretty(&status) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&state_path, json) {
                return err_internal::<serde_json::Value>(e);
            }
        }
        Err(e) => return err_internal::<serde_json::Value>(e),
    }

    ok_json(json!({
        "configured": true,
        "repo_url": status.repo_url,
        "branch": status.branch,
        "repo_dir": ctrl.repo_dir.display().to_string(),
    }))
}

/// POST /api/environments — Create a new environment tier.
pub(crate) async fn api_env_create(Json(req): Json<EnvCreateRequest>) -> impl IntoResponse {
    use crate::environments::{EnvTier, EnvironmentManager};

    let tier: EnvTier = match req.tier.parse() {
        Ok(t) => t,
        Err(e) => return err_bad_request::<serde_json::Value>(format!("Invalid tier: {e}")),
    };

    let path = EnvironmentManager::default_path();
    let mut manager = match EnvironmentManager::load(&path) {
        Ok(m) => m,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };
    manager.create_env(&req.name, tier);
    if let Err(e) = manager.save(&path) {
        return err_internal::<serde_json::Value>(e);
    }
    ok_json(json!({ "name": req.name, "tier": req.tier }))
}

/// POST /api/environments/promote — Promote a workload between environments.
pub(crate) async fn api_env_promote(Json(req): Json<EnvPromoteRequest>) -> impl IntoResponse {
    use crate::environments::{EnvironmentManager, PromotionRequest, PromotionStrategy};

    let path = EnvironmentManager::default_path();
    let mut manager = match EnvironmentManager::load(&path) {
        Ok(m) => m,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };

    let request = PromotionRequest {
        workload: req.workload.clone(),
        from_env: req.from.clone(),
        to_env: req.to.clone(),
        strategy: PromotionStrategy::TierAdjusted,
        require_approval: false,
    };

    match manager.promote(&request) {
        Ok(result) => {
            if let Err(e) = manager.save(&path) {
                return err_internal::<serde_json::Value>(e);
            }
            crate::metrics::record_env_promotion(&req.from, &req.to, true);
            ok_json(serde_json::to_value(result).unwrap_or(json!({})))
        }
        Err(e) => err_bad_request::<serde_json::Value>(e),
    }
}

/// GET /api/environments/parity — Compare workloads between two environments.
pub(crate) async fn api_env_parity(Query(q): Query<EnvParityQuery>) -> impl IntoResponse {
    use crate::environments::EnvironmentManager;

    let path = EnvironmentManager::default_path();
    let manager = match EnvironmentManager::load(&path) {
        Ok(m) => m,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };

    let env = match manager.get_env(&q.env1) {
        Some(e) => e,
        None => {
            return err_not_found::<serde_json::Value>(format!("Environment '{}' not found", q.env1))
        }
    };

    let workload_names: Vec<String> = env.workloads.keys().cloned().collect();
    let mut reports: Vec<serde_json::Value> = Vec::new();
    for wl_name in workload_names {
        match manager.check_parity(&q.env1, &q.env2, &wl_name) {
            Ok(report) => reports.push(serde_json::to_value(report).unwrap_or(json!({}))),
            Err(e) => reports.push(json!({ "workload": wl_name, "error": e.to_string() })),
        }
    }

    ok_json(json!({ "env1": q.env1, "env2": q.env2, "reports": reports }))
}

/// GET /api/sla — List all configured SLA targets.
pub(crate) async fn api_sla_list() -> impl IntoResponse {
    match load_sla_engine() {
        Ok(engine) => ok_json(engine.list_targets()).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}

/// POST /api/sla — Add an SLA target for a workload.
pub(crate) async fn api_sla_add(Json(req): Json<SlaAddRequest>) -> impl IntoResponse {
    use crate::sla::SlaTarget;

    let mut engine = match load_sla_engine() {
        Ok(e) => e,
        Err(err) => return err_internal::<serde_json::Value>(err).into_response(),
    };

    let target = match req.tier.as_str() {
        "standard" => SlaTarget::standard(&req.workload),
        "high-availability" | "ha" => SlaTarget::high_availability(&req.workload),
        "best-effort" | "be" => SlaTarget::best_effort(&req.workload),
        _ => {
            return err_bad_request::<serde_json::Value>(
                "Unknown tier — use standard, high-availability, or best-effort",
            )
            .into_response()
        }
    };

    engine.add_target(target.clone());
    if let Err(e) = save_sla_engine(&engine) {
        return err_internal::<serde_json::Value>(e).into_response();
    }
    ok_json(target).into_response()
}

/// POST /api/orchestrator/register — Register a workload for health monitoring.
pub(crate) async fn api_orchestrator_register(
    Json(req): Json<OrchestratorRegisterRequest>,
) -> impl IntoResponse {
    let rt: RuntimeKind = match req.runtime.parse() {
        Ok(r) => r,
        Err(e) => return err_bad_request::<serde_json::Value>(e),
    };

    let path = Orchestrator::default_path();
    let mut orch = match Orchestrator::load(&path) {
        Ok(o) => o,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };
    orch.register(&req.name, rt, None);
    if let Err(e) = orch.save(&path) {
        return err_internal::<serde_json::Value>(e);
    }
    ok_json(json!({ "registered": req.name, "runtime": req.runtime }))
}

/// POST /api/orchestrator/health-check — Run health checks for managed workloads.
pub(crate) async fn api_orchestrator_health_check() -> impl IntoResponse {
    let path = Orchestrator::default_path();
    let mut orch = match Orchestrator::load(&path) {
        Ok(o) => o,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };

    let state_path = StateStore::default_path();
    let state_store = match StateStore::load(&state_path) {
        Ok(s) => s,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };

    let statuses = collect_orchestrator_health(&orch, &state_store).await;
    let actions = orch.run_health_checks_from_statuses(&statuses);
    if let Err(e) = orch.save(&path) {
        return err_internal::<serde_json::Value>(e);
    }

    let health_path = crate::health::HealthHistory::default_path();
    let mut history = crate::health::HealthHistory::load(&health_path).unwrap_or_default();
    let now = crate::resources::now_rfc3339();
    for (wl_name, hs) in &statuses {
        if let Some(ws) = state_store.get(wl_name) {
            let (inst_state, ready) = match hs {
                OrcHealthStatus::Healthy => (InstanceState::Running, true),
                OrcHealthStatus::Degraded => (InstanceState::Running, false),
                OrcHealthStatus::Unhealthy => (InstanceState::Failed, false),
                OrcHealthStatus::Unknown => (InstanceState::Unknown, false),
            };
            history.record(crate::health::HealthRecord {
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

    ok_json(json!({
        "statuses": statuses,
        "actions": actions,
    }))
}

/// POST /api/orchestrator/reset-circuit — Reset circuit breaker for a workload.
pub(crate) async fn api_orchestrator_reset_circuit(
    Json(req): Json<OrchestratorNameRequest>,
) -> impl IntoResponse {
    let path = Orchestrator::default_path();
    let mut orch = match Orchestrator::load(&path) {
        Ok(o) => o,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };

    if orch.reset_circuit(&req.name) {
        if let Err(e) = orch.save(&path) {
            return err_internal::<serde_json::Value>(e);
        }
        ok_json(json!({ "reset": req.name }))
    } else {
        err_not_found::<serde_json::Value>(format!("Workload '{}' not found in orchestrator", req.name))
    }
}

/// POST /api/orchestrator/rolling-update — Rolling update for a managed workload.
pub(crate) async fn api_orchestrator_rolling_update(
    Json(req): Json<OrchestratorRollingUpdateRequest>,
) -> impl IntoResponse {
    let path = Orchestrator::default_path();
    let orch = match Orchestrator::load(&path) {
        Ok(o) => o,
        Err(e) => return err_internal::<serde_json::Value>(e).into_response(),
    };

    let statuses = orch.rolling_update(&req.name, req.replicas, None);
    if let Err(e) = orch.save(&path) {
        return err_internal::<serde_json::Value>(e).into_response();
    }
    ok_json(statuses).into_response()
}

/// GET /api/scheduler/placements — Current scheduler placement records.
pub(crate) async fn api_scheduler_placements() -> impl IntoResponse {
    use crate::scheduler::Scheduler;

    let path = Scheduler::default_path();
    match Scheduler::load(&path) {
        Ok(scheduler) => ok_json(scheduler.placements()).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}

/// GET /api/affinity/matrix — Runtime compatibility matrix.
pub(crate) async fn api_affinity_matrix() -> impl IntoResponse {
    use crate::ai::affinity::AffinityEngine;

    let path = AffinityEngine::default_path();
    match AffinityEngine::load(&path) {
        Ok(engine) => {
            let matrix = engine.compatibility_matrix();
            let rows: Vec<_> = matrix
                .iter()
                .map(|((class, rt), entry)| {
                    json!({
                        "class": format!("{class}"),
                        "runtime": format!("{rt}"),
                        "compatible": entry.compatible,
                        "score": entry.score,
                        "deployments": entry.deployments,
                        "known_issues": entry.known_issues,
                    })
                })
                .collect();
            ok_json(rows).into_response()
        }
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}

/// GET /api/affinity/stats — Affinity learning statistics.
pub(crate) async fn api_affinity_stats() -> impl IntoResponse {
    use crate::ai::affinity::AffinityEngine;

    let path = AffinityEngine::default_path();
    match AffinityEngine::load(&path) {
        Ok(engine) => ok_json(engine.stats()).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}

/// POST /api/plugins/register — Register a plugin from manifest JSON.
pub(crate) async fn api_plugins_register(
    Json(manifest): Json<crate::plugin::PluginManifest>,
) -> impl IntoResponse {
    let path = crate::plugin::PluginRegistry::default_path();
    let mut reg = match crate::plugin::PluginRegistry::load(&path) {
        Ok(r) => r,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };
    let name = manifest.name.clone();
    reg.register(manifest);
    if let Err(e) = reg.save(&path) {
        return err_internal::<serde_json::Value>(e);
    }
    ok_json(json!({ "registered": name }))
}

/// DELETE /api/plugins/:name — Remove a registered plugin.
pub(crate) async fn api_plugins_remove(Path(name): Path<String>) -> impl IntoResponse {
    let path = crate::plugin::PluginRegistry::default_path();
    let mut reg = match crate::plugin::PluginRegistry::load(&path) {
        Ok(r) => r,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };
    match reg.unregister(&name) {
        Some(_) => {
            if let Err(e) = reg.save(&path) {
                return err_internal::<serde_json::Value>(e);
            }
            ok_json(json!({ "removed": name }))
        }
        None => err_not_found::<serde_json::Value>(format!("Plugin '{name}' not found")),
    }
}

/// POST /api/compose/down — Stop workloads defined in a compose YAML (reverse dependency order).
pub(crate) async fn api_compose_down(
    AxumState(app_state): AxumState<AppState>,
    body: String,
) -> impl IntoResponse {
    use crate::compose;

    let spec = match serde_yaml::from_str::<compose::ComposeSpec>(&body) {
        Ok(s) => s,
        Err(e) => return err_bad_request::<serde_json::Value>(format!("Invalid YAML: {e}")),
    };

    let order = match compose::resolve_order(&spec) {
        Ok(o) => o,
        Err(e) => return err_bad_request::<serde_json::Value>(e.to_string()),
    };

    let mut stopped = Vec::new();
    let mut errors = Vec::new();
    for name in order.into_iter().rev() {
        match super::handlers::stop_workload_if_exists(&app_state, &name).await {
            Ok(()) => stopped.push(name),
            Err(msg) => errors.push(json!({ "workload": name, "error": msg })),
        }
    }

    ok_json(json!({ "stopped": stopped, "errors": errors }))
}

/// GET /api/workloads/:name/snapshots — List backup snapshots for rollback.
pub(crate) async fn api_workload_snapshots(Path(name): Path<String>) -> impl IntoResponse {
    let snap_mgr = SnapshotManager::new();
    match snap_mgr.list_snapshots(&name) {
        Ok(paths) => {
            let snapshots: Vec<_> = paths
                .into_iter()
                .enumerate()
                .map(|(i, p)| json!({ "version": i, "path": p.display().to_string() }))
                .collect();
            ok_json(snapshots).into_response()
        }
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}

/// POST /api/workloads/:name/rollback — Roll back to a snapshot (latest or by version index).
pub(crate) async fn api_workload_rollback(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
    Json(req): Json<RollbackRequest>,
) -> impl IntoResponse {
    let snap_mgr = SnapshotManager::new();
    let snapshot_path = if let Some(v) = req.version {
        match snap_mgr.list_snapshots(&name) {
            Ok(snaps) => match snaps.into_iter().nth(v) {
                Some(p) => p,
                None => {
                    return err_not_found::<serde_json::Value>(format!(
                        "Snapshot version {v} not found for '{name}'"
                    ))
                }
            },
            Err(e) => return err_internal::<serde_json::Value>(e),
        }
    } else {
        match snap_mgr.latest_snapshot(&name) {
            Ok(Some(p)) => p,
            Ok(None) => {
                return err_not_found::<serde_json::Value>(format!("No snapshot for '{name}'"))
            }
            Err(e) => return err_internal::<serde_json::Value>(e),
        }
    };

    let backup = match Backup::load(&snapshot_path) {
        Ok(b) => b,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };
    let snapshot_ws = match backup.workloads.first() {
        Some(ws) => ws.clone(),
        None => return err_bad_request::<serde_json::Value>("Snapshot is empty"),
    };

    let mut store = match StateStore::load(&app_state.state_path) {
        Ok(s) => s,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };

    if let Some(current) = store.get(&name) {
        if let Ok(rt) = runtime::create_runtime(&current.runtime).await {
            let _ = rt.stop(&current.instance).await;
        }
    }

    let rt = match runtime::create_runtime(&snapshot_ws.runtime).await {
        Ok(r) => r,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };
    let spec = match Workload::from_file(&snapshot_ws.spec_path) {
        Ok(s) => s,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };
    let image = match rt.build(&spec).await {
        Ok(i) => i,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };
    let instance = match rt.run(&image, &spec).await {
        Ok(i) => i,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };

    store.upsert(
        name.clone(),
        WorkloadState {
            name: name.clone(),
            runtime: snapshot_ws.runtime,
            instance,
            spec_path: snapshot_ws.spec_path.clone(),
            created_at: snapshot_ws.created_at.clone(),
            updated_at: crate::resources::now_rfc3339(),
            os_version: snapshot_ws.os_version.clone(),
            node_labels: snapshot_ws.node_labels.clone(),
        },
    );
    if let Err(e) = persist_workload_api(&app_state, &store).await {
        return err_internal::<serde_json::Value>(e);
    }
    {
        let mut locked = app_state.state.write().await;
        *locked = store;
    }

    ok_json(json!({
        "rolled_back": name,
        "snapshot": snapshot_path.display().to_string(),
    }))
}

/// GET /api/helm/catalog — Curated Helm charts for App Store UI.
pub(crate) async fn api_helm_catalog() -> impl IntoResponse {
    ok_json(crate::helm::helm_catalog())
}

/// POST /api/helm/export — Export a Helm chart from workload YAML.
pub(crate) async fn api_helm_export(Json(req): Json<HelmExportRequest>) -> impl IntoResponse {
    let spec: Workload = match serde_yaml::from_str(&req.yaml) {
        Ok(s) => s,
        Err(e) => return err_bad_request::<serde_json::Value>(format!("Invalid YAML: {e}")),
    };

    let sub = format!("helm-export/{}", spec.metadata.name);
    let output_dir = crate::resources::aether_path(&sub);
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        return err_internal::<serde_json::Value>(e);
    }
    if let Err(e) = crate::helm::export_helm_chart(
        &spec,
        &output_dir,
        req.chart_version.as_deref(),
    ) {
        return err_internal::<serde_json::Value>(e);
    }

    let files: Vec<String> = walkdir_files(&output_dir);
    ok_json(json!({
        "output_dir": output_dir.display().to_string(),
        "workload": spec.metadata.name,
        "files": files,
    }))
}

fn walkdir_files(dir: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir_files(&path));
            } else if let Some(s) = path.to_str() {
                files.push(s.to_string());
            }
        }
    }
    files.sort();
    files
}

/// GET /api/webhooks/queue — Pending webhook delivery queue.
pub(crate) async fn api_webhooks_queue() -> impl IntoResponse {
    match crate::events::WebhookQueue::load(&crate::events::WebhookQueue::default_path()) {
        Ok(queue) => ok_json(queue.pending).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}

/// POST /api/webhooks/flush — Process pending webhook deliveries once.
pub(crate) async fn api_webhooks_flush() -> impl IntoResponse {
    let before = crate::events::WebhookQueue::load(&crate::events::WebhookQueue::default_path())
        .map(|q| q.pending.len())
        .unwrap_or(0);
    crate::events::WebhookQueue::process_queue_once();
    let after = crate::events::WebhookQueue::load(&crate::events::WebhookQueue::default_path())
        .map(|q| q.pending.len())
        .unwrap_or(0);
    ok_json(json!({ "before": before, "after": after, "processed": before.saturating_sub(after) }))
}
