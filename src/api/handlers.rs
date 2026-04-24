//! API handler functions

use super::types::*;
use crate::config::Config;
use crate::engine::Engine;
use crate::kubecluster::ClusterLogsRequest;
use crate::runtime::{self, RuntimeKind};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use crate::{backup, cost, Runtime};

use axum::{
    extract::{Path, Query, State as AxumState},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use axum::http::header;
use futures::stream::StreamExt;
use std::path::PathBuf;

/// Validate a workload name from API input.
/// Accepts only DNS-1123 compatible names: lowercase alphanumeric, hyphens, dots,
/// up to 253 characters. Rejects empty names, path traversal, and special characters.
fn validate_api_name<T: serde::Serialize>(name: &str) -> Result<(), (StatusCode, Json<ApiResponse<T>>)> {
    if name.is_empty() || name.len() > 253 {
        return Err(err_bad_request(format!("Invalid workload name: '{}' (must be 1-253 characters)", name)));
    }
    // Only allow lowercase alphanumeric, hyphens, and dots (DNS-1123 compatible)
    let valid = name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.');
    if !valid || name.starts_with('-') || name.starts_with('.') || name.ends_with('-') || name.ends_with('.') {
        return Err(err_bad_request(format!(
            "Invalid workload name: '{}'. Must match [a-z0-9][a-z0-9.-]*[a-z0-9]", name
        )));
    }
    Ok(())
}

/// Validate that a spec_path does not contain path traversal sequences or absolute paths.
fn validate_spec_path<T: serde::Serialize>(path: &std::path::Path) -> Result<(), (StatusCode, Json<ApiResponse<T>>)> {
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        return Err(err_bad_request("spec_path contains path traversal sequence"));
    }
    if path.is_absolute() {
        return Err(err_bad_request("spec_path must be a relative path"));
    }
    Ok(())
}

/// Load a workload spec from a validated path, or return an HTTP error.
fn load_spec_safe<T: serde::Serialize>(path: &PathBuf) -> Result<Workload, (StatusCode, Json<ApiResponse<T>>)> {
    validate_spec_path(path)?;
    Workload::from_file(path).map_err(|e| err_internal(format!("Failed to load workload spec: {}", e)))
}

/// Look up a workload by name from state, returning a cloned WorkloadState
/// or an HTTP 404 error response.
async fn lookup_workload<T: serde::Serialize>(
    app_state: &AppState,
    name: &str,
) -> Result<WorkloadState, (StatusCode, Json<ApiResponse<T>>)> {
    validate_api_name(name)?;
    let state = app_state.state.read().await;
    state
        .get(name)
        .cloned()
        .ok_or_else(|| err_not_found(format!("Workload {} not found", name)))
}

/// Create a runtime client or return an HTTP 500 error response.
async fn make_runtime<T: serde::Serialize>(
    kind: &RuntimeKind,
) -> Result<Box<dyn Runtime>, (StatusCode, Json<ApiResponse<T>>)> {
    runtime::create_runtime(kind).await.map_err(|e| err_internal(e))
}

/// Emit a server-sent event for real-time dashboard updates.
fn emit_sse(app_state: &AppState, event: &ServerEvent) {
    if let Ok(json) = serde_json::to_string(event) {
        let _ = app_state.event_tx.send(json);
    }
}

/// Shorthand for a successful JSON response.
fn ok_json<T: serde::Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
    (StatusCode::OK, Json(ApiResponse::success(data)))
}

/// Shorthand for a 201 Created JSON response.
fn created_json<T: serde::Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
    (StatusCode::CREATED, Json(ApiResponse::success(data)))
}

/// Shorthand for an internal-server-error JSON response.
fn err_internal<T: serde::Serialize>(e: impl std::fmt::Display) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::error(e.to_string())),
    )
}

/// Shorthand for a bad-request JSON response.
fn err_bad_request<T: serde::Serialize>(e: impl std::fmt::Display) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(e.to_string())),
    )
}

/// Shorthand for a not-found JSON response.
fn err_not_found<T: serde::Serialize>(msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error(msg.into())),
    )
}

/// Embedded dashboard HTML (built from web/dashboard/ React app)
pub(crate) const DASHBOARD_HTML: &str = include_str!("../../web/dashboard/dist/index.html");
const DASHBOARD_CSS: &str = include_str!("../../web/dashboard/dist/assets/index-DiVd-Lmd.css");
const DASHBOARD_JS: &str = include_str!("../../web/dashboard/dist/assets/index-qv9Tu_QH.js");

/// GET / - Serve the web dashboard
pub(crate) async fn serve_dashboard() -> impl IntoResponse {
    Html(DASHBOARD_HTML)
}

/// GET /assets/*.css - Serve embedded dashboard CSS
pub(crate) async fn serve_dashboard_css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], DASHBOARD_CSS)
}

/// GET /assets/*.js - Serve embedded dashboard JS
pub(crate) async fn serve_dashboard_js() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/javascript")], DASHBOARD_JS)
}

/// GET /health - Health check endpoint
pub(crate) async fn health_check() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Json(ApiResponse::success(response))
}

/// Discover workloads live from Kubernetes Deployments and KubeVirt VMs
/// across all namespaces. Returns instances regardless of managed-by labels.
async fn discover_live_workloads() -> Vec<WorkloadResponse> {
    let mut results = Vec::new();

    // Discover K8s Deployments across all namespaces
    if let Ok(client) = kube::Client::try_default().await {
        let deployments: kube::Api<k8s_openapi::api::apps::v1::Deployment> = kube::Api::all(client.clone());
        if let Ok(deploy_list) = deployments.list(&kube::api::ListParams::default()).await {
            for deploy in &deploy_list.items {
                let name = deploy.metadata.name.clone().unwrap_or_default();
                let ns = deploy.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
                let labels = deploy.metadata.labels.as_ref();
                let managed = labels.map_or(false, |l| l.get("managed-by").map_or(false, |v| v == "aether"));

                let image = deploy.spec.as_ref()
                    .and_then(|s| s.template.spec.as_ref())
                    .and_then(|ps| ps.containers.first())
                    .and_then(|c| c.image.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let ready_replicas = deploy.status.as_ref()
                    .and_then(|s| s.ready_replicas)
                    .unwrap_or(0);
                let replicas = deploy.spec.as_ref()
                    .and_then(|s| s.replicas)
                    .unwrap_or(1);

                let status = if ready_replicas >= replicas {
                    "running".to_string()
                } else if ready_replicas > 0 {
                    "degraded".to_string()
                } else {
                    "pending".to_string()
                };

                let created_at = deploy.metadata.creation_timestamp.as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default();

                let runtime_label = if managed { "Kubernetes (aether)" } else { "Kubernetes" };

                results.push(WorkloadResponse {
                    name: format!("{}/{}", ns, name),
                    runtime: runtime_label.to_string(),
                    image,
                    status,
                    created_at,
                    source: Some("cluster".to_string()),
                    cluster: None,
                    namespace: Some(ns),
                    kind: Some("Deployment".to_string()),
                });
            }
        }

        // Discover KubeVirt VMs across all namespaces
        let vms: kube::Api<kube::core::DynamicObject> = kube::Api::all_with(
            client.clone(),
            &kube::discovery::ApiResource {
                group: "kubevirt.io".to_string(),
                version: "v1".to_string(),
                api_version: "kubevirt.io/v1".to_string(),
                kind: "VirtualMachineInstance".to_string(),
                plural: "virtualmachineinstances".to_string(),
            },
        );
        if let Ok(vm_list) = vms.list(&kube::api::ListParams::default()).await {
            for vm in &vm_list.items {
                let name = vm.metadata.name.clone().unwrap_or_default();
                let ns = vm.metadata.namespace.clone().unwrap_or_else(|| "default".to_string());
                let labels = vm.metadata.labels.as_ref();
                let managed = labels.map_or(false, |l| l.get("managed-by").map_or(false, |v| v == "aether"));

                let phase = vm.data.get("status")
                    .and_then(|s| s.get("phase"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("Unknown");

                let status = match phase {
                    "Running" => "running",
                    "Succeeded" => "stopped",
                    "Failed" => "failed",
                    "Scheduling" | "Scheduled" | "Pending" => "pending",
                    _ => "unknown",
                };

                let created_at = vm.metadata.creation_timestamp.as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default();

                let runtime_label = if managed { "KubeVirt (aether)" } else { "KubeVirt" };

                results.push(WorkloadResponse {
                    name: format!("{}/{}", ns, name),
                    runtime: runtime_label.to_string(),
                    image: format!("vm:{}", name),
                    status: status.to_string(),
                    created_at,
                    source: Some("cluster".to_string()),
                    cluster: None,
                    namespace: Some(ns),
                    kind: Some("VirtualMachineInstance".to_string()),
                });
            }
        }
    }

    results
}

/// GET /api/workloads - List all workloads (state store + live discovery)
pub(crate) async fn list_workloads(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    // Start with state store entries
    let mut seen = std::collections::HashSet::new();
    let state = app_state.state.read().await;
    let mut workloads: Vec<WorkloadResponse> = state
        .list()
        .iter()
        .map(|w| {
            seen.insert(w.name.clone());
            WorkloadResponse {
                name: w.name.clone(),
                runtime: format!("{:?}", w.runtime),
                image: w.instance.image.clone(),
                status: format!("deployed ({})", w.runtime),
                created_at: w.created_at.clone(),
                source: Some("aether".to_string()),
                cluster: None,
                namespace: None,
                kind: None,
            }
        })
        .collect();
    drop(state);

    // Merge live-discovered workloads (skip duplicates already in state store)
    let live = discover_live_workloads().await;
    for w in live {
        // Match by bare name (state store uses bare name, discovery uses ns/name)
        let bare_name = w.name.rsplit('/').next().unwrap_or(&w.name);
        if !seen.contains(bare_name) && !seen.contains(&w.name) {
            seen.insert(w.name.clone());
            workloads.push(w);
        }
    }

    if let Ok(cluster_workloads) = crate::kubecluster::list_workloads().await {
        for workload in cluster_workloads {
            let full_name = format!("{}/{}/{}", workload.cluster, workload.namespace, workload.name);
            if seen.insert(full_name.clone()) {
                workloads.push(WorkloadResponse {
                    name: full_name,
                    runtime: "Kubernetes".to_string(),
                    image: workload.image,
                    status: workload.status,
                    created_at: workload.created_at,
                    source: Some("cluster".to_string()),
                    cluster: Some(workload.cluster),
                    namespace: Some(workload.namespace),
                    kind: Some(workload.kind),
                });
            }
        }
    }

    Json(ApiResponse::success(workloads))
}

/// POST /api/workloads - Create and deploy a workload
pub(crate) async fn create_workload(
    AxumState(app_state): AxumState<AppState>,
    Json(request): Json<CreateWorkloadRequest>,
) -> impl IntoResponse {
    // Select runtime
    let runtime_kind = if let Some(runtime_name) = request.runtime {
        match runtime_name.parse::<RuntimeKind>() {
            Ok(rt) => rt,
            Err(e) => {
                return err_bad_request::<String>(e)
            }
        }
    } else {
        // Use decision engine
        let engine = Engine::new();
        match engine.decide(&request.spec) {
            Ok(runtime) => runtime,
            Err(e) => {
                return err_internal::<String>(e)
            }
        }
    };

    // Build and run based on runtime
    let runtime = match make_runtime::<String>(&runtime_kind).await {
        Ok(r) => r,
        Err(e) => return e,
    };
    let image = match runtime.build(&request.spec).await {
        Ok(img) => img,
        Err(e) => {
            return err_internal::<String>(e)
        }
    };
    let instance = match runtime.run(&image, &request.spec).await {
        Ok(inst) => inst,
        Err(e) => {
            return err_internal::<String>(e)
        }
    };

    // Save state
    let mut state = app_state.state.write().await;
    state.upsert(
        request.spec.metadata.name.clone(),
        WorkloadState::new(
            request.spec.metadata.name.clone(),
            runtime_kind,
            instance,
            PathBuf::from("api_created"),
        ),
    );

    // Persist to disk
    if let Err(e) = state.save(&StateStore::default_path()) {
        return err_internal::<String>(e);
    }

    emit_sse(&app_state, &ServerEvent::WorkloadChanged {
        name: request.spec.metadata.name.clone(),
        action: "created".to_string(),
    });

    created_json(format!("Workload {} created", request.spec.metadata.name))
}

/// GET /api/workloads/:name - Get workload details
pub(crate) async fn get_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let workload = match lookup_workload::<WorkloadResponse>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let response = WorkloadResponse {
        name: workload.name.clone(),
        runtime: format!("{:?}", workload.runtime),
        image: workload.instance.image.clone(),
        status: format!("deployed ({})", workload.runtime),
        created_at: workload.created_at.clone(),
        source: Some("aether".to_string()),
        cluster: None,
        namespace: None,
        kind: None,
    };
    ok_json(response)
}

/// DELETE /api/workloads/:name - Delete a workload
pub(crate) async fn delete_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut state = app_state.state.write().await;

    match state.remove(&name) {
        Some(_) => {
            // Persist to disk
            if let Err(e) = state.save(&StateStore::default_path()) {
                return err_internal::<String>(e);
            }

            emit_sse(&app_state, &ServerEvent::WorkloadChanged {
                name: name.clone(),
                action: "deleted".to_string(),
            });

            (
                StatusCode::OK,
                Json(ApiResponse::success(format!("Workload {} deleted", name))),
            )
        }
        None => err_not_found::<String>(format!("Workload {} not found", name)),
    }
}

/// GET /api/workloads/:name/logs - Get workload logs
pub(crate) async fn get_logs(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let workload = match lookup_workload::<String>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let rt = match make_runtime::<String>(&workload.runtime).await {
        Ok(r) => r,
        Err(e) => return e,
    };

    match rt.logs(&workload.instance, false).await {
        Ok(logs) => ok_json(logs),
        Err(e) => err_internal::<String>(e),
    }
}

/// POST /api/workloads/:name/start - Start a workload
pub(crate) async fn start_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let workload_state = match lookup_workload::<String>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    // Load workload spec (validates path against traversal)
    let spec = match load_spec_safe::<String>(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => return e,
    };

    // Build and run on the same runtime
    let runtime = match make_runtime::<String>(&workload_state.runtime).await {
        Ok(r) => r,
        Err(e) => return e,
    };
    let image = match runtime.build(&spec).await {
        Ok(img) => img,
        Err(e) => {
            return err_internal::<String>(e)
        }
    };
    let instance = match runtime.run(&image, &spec).await {
        Ok(inst) => inst,
        Err(e) => {
            return err_internal::<String>(e)
        }
    };

    // Re-check workload still exists before updating state (guard against concurrent delete)
    let mut state = app_state.state.write().await;
    if state.get(&name).is_none() {
        // Workload was deleted while we were building/running — stop the orphaned instance
        tracing::warn!(
            "Workload '{}' was deleted during start; stopping orphaned instance",
            name
        );
        // Reuse the runtime client we already created to stop the orphaned instance
        let _ = runtime.stop(&instance).await;
        return err_internal::<String>(format!(
            "Workload '{}' was deleted by a concurrent request during start",
            name
        ));
    }

    state.upsert(
        name.clone(),
        WorkloadState {
            name: name.clone(),
            runtime: workload_state.runtime,
            instance,
            spec_path: workload_state.spec_path,
            created_at: workload_state.created_at,
            updated_at: crate::resources::now_rfc3339(),
            os_version: workload_state.os_version,
            node_labels: workload_state.node_labels,
        },
    );

    if let Err(e) = state.save(&StateStore::default_path()) {
        return err_internal::<String>(e);
    }

    emit_sse(&app_state, &ServerEvent::WorkloadChanged {
        name: name.clone(),
        action: "started".to_string(),
    });

    (
        StatusCode::OK,
        Json(ApiResponse::success(format!("Workload {} started", name))),
    )
}

/// POST /api/workloads/:name/stop - Stop a workload
pub(crate) async fn stop_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let workload = match lookup_workload::<String>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let rt = match make_runtime::<String>(&workload.runtime).await {
        Ok(r) => r,
        Err(e) => return e,
    };

    match rt.stop(&workload.instance).await {
        Ok(_) => {
            emit_sse(&app_state, &ServerEvent::WorkloadChanged {
                name: name.clone(),
                action: "stopped".to_string(),
            });

            (
                StatusCode::OK,
                Json(ApiResponse::success(format!("Workload {} stopped", name))),
            )
        }
        Err(e) => err_internal::<String>(e),
    }
}

/// POST /api/cost - Estimate costs for a workload
pub(crate) async fn estimate_cost(Json(spec): Json<Workload>) -> impl IntoResponse {
    match cost::estimate_all_providers(&spec) {
        Ok(estimates) => ok_json(estimates),
        Err(e) => err_internal::<Vec<cost::CostEstimate>>(e),
    }
}

/// GET /api/backups - List all backups
pub(crate) async fn list_backups() -> impl IntoResponse {
    let manager = backup::BackupManager::new(backup::BackupManager::default_dir());

    match manager.list_backups() {
        Ok(backups) => ok_json(backups),
        Err(e) => err_internal::<Vec<PathBuf>>(e),
    }
}

/// POST /api/backups - Create a new backup
pub(crate) async fn create_backup(
    AxumState(app_state): AxumState<AppState>,
    Json(request): Json<CreateBackupRequest>,
) -> impl IntoResponse {
    let state = app_state.state.read().await;
    let manager = backup::BackupManager::new(backup::BackupManager::default_dir());

    match manager.create_backup(&state, request.name, request.description) {
        Ok(path) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(format!("Backup created: {:?}", path))),
        ),
        Err(e) => err_internal::<String>(e),
    }
}

/// POST /api/ai/recommend - AI-powered runtime recommendation
pub(crate) async fn ai_recommend(Json(spec): Json<Workload>) -> impl IntoResponse {
    use crate::ai::scoring::ScoringEngine;
    use crate::config::Config;

    let config = Config::load();
    let engine = ScoringEngine::new(config.engine);
    let result = engine.score(&spec);

    ok_json(result)
}

/// GET /api/ai/profile/:name - Profile a deployed workload
pub(crate) async fn ai_profile(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::ai::profiler::Profiler;

    let workload_state = match lookup_workload::<serde_json::Value>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let spec = match load_spec_safe::<serde_json::Value>(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let config = Config::load();
    let profiler = Profiler::new(config.profiler.waste_threshold);
    let profile = profiler.profile(&spec, Some(workload_state.runtime));

    let value = match serde_json::to_value(profile) {
        Ok(v) => v,
        Err(e) => return err_internal::<serde_json::Value>(format!("Failed to serialize profile: {}", e)),
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(value)),
    )
}

/// GET /api/ai/analyze/:name - Analyze workload logs
pub(crate) async fn ai_analyze_logs(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::ai::analyzer::LogAnalyzer;

    let workload = match lookup_workload::<serde_json::Value>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let rt = match make_runtime::<serde_json::Value>(&workload.runtime).await {
        Ok(r) => r,
        Err(e) => return e,
    };

    let logs = match rt.logs(&workload.instance, false).await {
        Ok(l) => l,
        Err(e) => {
            return err_internal::<serde_json::Value>(e)
        }
    };

    let config = Config::load();
    let analyzer = LogAnalyzer::new(config.analyzer);
    let analysis = analyzer.analyze(&logs);

    match serde_json::to_value(analysis) {
        Ok(value) => (StatusCode::OK, Json(ApiResponse::success(value))),
        Err(e) => err_internal::<serde_json::Value>(format!("Failed to serialize analysis: {}", e)),
    }
}

/// GET /api/ai/migration-advice/:name/:target - Migration path recommendations
pub(crate) async fn ai_migration_advice(
    AxumState(app_state): AxumState<AppState>,
    Path((name, target)): Path<(String, String)>,
) -> impl IntoResponse {
    use crate::ai::migration::MigrationAdvisor;

    let workload_state = match lookup_workload::<MigrationAdviceResponse>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let target_runtime = match target.parse::<RuntimeKind>() {
        Ok(rt) => rt,
        Err(e) => {
            return err_bad_request::<MigrationAdviceResponse>(e)
        }
    };

    let spec = match load_spec_safe::<MigrationAdviceResponse>(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let config = Config::load();
    let advisor = MigrationAdvisor::new(config.migration);
    let advice = advisor.advise(&spec, workload_state.runtime, target_runtime);

    let response = MigrationAdviceResponse {
        workload_name: name,
        source_runtime: format!("{}", workload_state.runtime),
        target_runtime: format!("{}", target_runtime),
        recommended_strategy: format!("{:?}", advice.recommended_strategy),
        estimated_downtime_secs: advice.estimated_downtime_secs,
        risk_level: format!("{}", advice.risk_level),
        reasons: advice.reasons,
        warnings: advice.warnings,
        timing: TimingAdviceResponse {
            recommendation: advice.suggested_timing.recommendation,
            preferred_window: advice.suggested_timing.preferred_window,
            avoid_times: advice.suggested_timing.avoid_times,
        },
        canary_config: CanaryConfigResponse {
            steps: advice.canary_config.steps,
            step_interval_secs: advice.canary_config.step_interval_secs,
            error_threshold: advice.canary_config.error_threshold,
            latency_threshold_pct: advice.canary_config.latency_threshold_pct,
            min_observation_secs: advice.canary_config.min_observation_secs,
        },
    };

    ok_json(response)
}

/// GET /api/ai/scaling-advice - Predictive scaling recommendations
pub(crate) async fn ai_scaling_advice() -> impl IntoResponse {
    use crate::ai::scaling::{ScalingEngine, TimeSeries};

    let config = Config::load();
    let engine = ScalingEngine::new(config.scaling);

    // Generate simulated metrics (same as CLI scaling_advice_command)
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

    let response = ScalingAdviceResponse {
        action: format!("{}", rec.action),
        current_replicas: rec.current_replicas,
        recommended_replicas: rec.recommended_replicas,
        reason: rec.reason,
        confidence: rec.confidence,
        forecast: ForecastResponse {
            trend: format!("{}", rec.forecast.trend),
            predicted_value: rec.forecast.predicted_value,
            lower_bound: rec.forecast.lower_bound,
            upper_bound: rec.forecast.upper_bound,
            horizon_minutes: rec.forecast.horizon_minutes,
        },
        cost_impact: CostImpactResponse {
            current_hourly: rec.cost_impact.current_hourly,
            projected_hourly: rec.cost_impact.projected_hourly,
            delta_hourly: rec.cost_impact.delta_hourly,
            delta_monthly: rec.cost_impact.delta_monthly,
        },
    };

    ok_json(response)
}

/// GET /api/drift/:name - Check drift for a workload
pub(crate) async fn api_drift_check(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::drift::DriftDetector;

    let workload_state = match lookup_workload::<serde_json::Value>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let spec = match load_spec_safe::<serde_json::Value>(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let detector = DriftDetector::new();
    let report = detector.detect(&spec, &workload_state);

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            match serde_json::to_value(report) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
        )),
    )
}

/// POST /api/policy/check - Check workload against policies
pub(crate) async fn api_policy_check(Json(request): Json<PolicyCheckRequest>) -> impl IntoResponse {
    use crate::policy::PolicyEngine;

    let engine = match request.policy_set.as_deref() {
        Some("development") => PolicyEngine::development(),
        _ => PolicyEngine::production(),
    };

    let result = engine.evaluate(&request.spec);

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            match serde_json::to_value(result) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
        )),
    )
}

/// GET /api/dependencies - Show dependency graph
pub(crate) async fn api_deps_show() -> impl IntoResponse {
    use crate::dependencies::DependencyGraph;

    let graph_path = DependencyGraph::default_path();
    match DependencyGraph::load(&graph_path) {
        Ok(graph) => {
            let stats = graph.stats();
            let order = graph.startup_order().ok();
            let response = serde_json::json!({
                "stats": stats,
                "startup_order": order,
                "issues": graph.validate(),
            });
            ok_json(response)
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// POST /api/dependencies - Add a dependency
pub(crate) async fn api_deps_add(Json(request): Json<AddDependencyRequest>) -> impl IntoResponse {
    use crate::dependencies::DependencyGraph;

    let graph_path = DependencyGraph::default_path();
    match DependencyGraph::load(&graph_path) {
        Ok(mut graph) => {
            graph.add_dependency(&request.workload, &request.dependency);
            if let Err(e) = graph.save(&graph_path) {
                return err_internal::<String>(e);
            }
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(format!(
                    "Dependency added: {} -> {}",
                    request.workload, request.dependency
                ))),
            )
        }
        Err(e) => err_internal::<String>(e),
    }
}

/// GET /api/audit - List audit events
pub(crate) async fn api_audit_list() -> impl IntoResponse {
    use crate::audit::AuditLog;

    let audit_path = AuditLog::default_path();
    match AuditLog::load(&audit_path) {
        Ok(log) => {
            let summary = log.summary();
            let recent = log.last_n(20);
            let response = serde_json::json!({
                "summary": summary,
                "recent_events": recent,
            });
            (
                StatusCode::OK,
                Json(ApiResponse::success(response)),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/templates - List available templates
pub(crate) async fn api_template_list() -> impl IntoResponse {
    use crate::templates;

    let templates = templates::list_templates();
    (
        StatusCode::OK,
        Json(ApiResponse::success(
            match serde_json::to_value(templates) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
        )),
    )
}

/// POST /api/templates/:name - Generate a workload from template
pub(crate) async fn api_template_generate(
    Path(name): Path<String>,
    Json(request): Json<TemplateRequest>,
) -> impl IntoResponse {
    use crate::templates::{self, TemplateKind, TemplateParams};

    let kind = match name.parse::<TemplateKind>() {
        Ok(k) => k,
        Err(e) => {
            return err_bad_request::<serde_json::Value>(e)
        }
    };

    let params = TemplateParams {
        name: request
            .workload_name
            .unwrap_or_else(|| format!("my-{}", name)),
        owner: request.owner.unwrap_or_else(|| "team".to_string()),
        project: request.project.unwrap_or_else(|| "default".to_string()),
        registry: request
            .registry
            .unwrap_or_else(|| "ghcr.io/org".to_string()),
        cpu: request.cpu,
        memory: request.memory,
        storage: None,
        port: request.port,
        replicas: request.replicas,
        host: None,
    };

    let spec = templates::generate(&kind, &params);

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            match serde_json::to_value(spec) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
        )),
    )
}

/// GET /api/sla/:workload - Check SLA compliance
pub(crate) async fn api_sla_check(Path(workload): Path<String>) -> impl IntoResponse {
    use crate::sla::{SlaEngine, SlaTarget};

    let sla_path = crate::resources::aether_path("sla.json");

    if !sla_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(
                "No SLA targets configured".to_string(),
            )),
        );
    }

    let content = match std::fs::read_to_string(&sla_path) {
        Ok(c) => c,
        Err(e) => {
            return err_internal::<serde_json::Value>(e)
        }
    };

    let targets: Vec<SlaTarget> = match serde_json::from_str(&content) {
        Ok(t) => t,
        Err(e) => {
            return err_internal::<serde_json::Value>(e)
        }
    };

    let mut engine = SlaEngine::new();
    for target in targets {
        engine.add_target(target);
    }

    match engine.get_target(&workload) {
        Some(target) => (
            StatusCode::OK,
            Json(ApiResponse::success(
                match serde_json::to_value(target) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
            )),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(format!(
                "No SLA target for workload: {}",
                workload
            ))),
        ),
    }
}

/// GET /api/secrets - List secrets
pub(crate) async fn api_secrets_list() -> impl IntoResponse {
    use crate::secrets::SecretStore;

    let path = SecretStore::default_path();
    match SecretStore::load(&path) {
        Ok(store) => {
            let summaries = store.list();
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(summaries) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/events - List recent events
pub(crate) async fn api_events_list() -> impl IntoResponse {
    use crate::events::EventBus;

    let path = EventBus::default_path();
    match EventBus::load(&path) {
        Ok(bus) => {
            let events = bus.last_n(50);
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(events) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/events/summary - Event summary
pub(crate) async fn api_events_summary() -> impl IntoResponse {
    use crate::events::EventBus;

    let path = EventBus::default_path();
    match EventBus::load(&path) {
        Ok(bus) => {
            let summary = bus.summary();
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(summary) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/cluster/summary - Native kubeconfig-backed cluster summary
pub(crate) async fn api_cluster_summary() -> impl IntoResponse {
    ok_json(crate::kubecluster::cluster_summary().await)
}

/// GET /api/cluster/logs - Get logs for a kubeconfig-backed Kubernetes workload
pub(crate) async fn api_cluster_logs(
    Query(query): Query<ClusterLogsQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::workload_logs(&ClusterLogsRequest {
        cluster: query.cluster,
        namespace: query.namespace,
        kind: query.kind,
        name: query.name,
    })
    .await
    {
        Ok(logs) => ok_json(logs),
        Err(error) => err_internal::<String>(error),
    }
}

/// GET /api/cluster/resource - Inspect a native Kubernetes workload manifest and pods.
pub(crate) async fn api_cluster_resource(
    Query(query): Query<ClusterResourceQuery>,
) -> impl IntoResponse {
    let request = ClusterLogsRequest {
        cluster: query.cluster,
        namespace: query.namespace,
        kind: query.kind,
        name: query.name,
    };

    match crate::kubecluster::workload_detail(&request).await {
        Ok(detail) => ok_json(detail),
        Err(error) => err_internal::<crate::kubecluster::ClusterResourceDetail>(error),
    }
}

/// POST /api/cluster/action - Execute a native Kubernetes workload action.
pub(crate) async fn api_cluster_action(
    Json(request): Json<ClusterActionRequestBody>,
) -> impl IntoResponse {
    match crate::kubecluster::workload_action(&crate::kubecluster::ClusterActionRequest {
        cluster: request.cluster,
        namespace: request.namespace,
        kind: request.kind,
        name: request.name,
        action: request.action,
        replicas: request.replicas,
    }).await {
        Ok(message) => ok_json(message),
        Err(error) => err_internal::<String>(error),
    }
}

/// GET /api/cluster/namespaces - List namespaces for a kubeconfig-backed cluster.
pub(crate) async fn api_cluster_namespaces(
    Query(query): Query<ClusterNamespacesQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::list_namespaces(&query.cluster).await {
        Ok(namespaces) => ok_json(namespaces),
        Err(error) => err_internal::<Vec<crate::kubecluster::ClusterNamespaceSummary>>(error),
    }
}

/// GET /api/cluster/browse - Browse native Kubernetes resources by cluster, namespace, and kind.
pub(crate) async fn api_cluster_browse(
    Query(query): Query<ClusterBrowseQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::browse_resources(&crate::kubecluster::ClusterBrowseRequest {
        cluster: query.cluster,
        namespace: query.namespace,
        kind: query.kind,
    }).await {
        Ok(resources) => ok_json(resources),
        Err(error) => err_internal::<Vec<crate::kubecluster::ClusterResourceSummary>>(error),
    }
}

/// POST /api/cluster/apply - Apply an edited Kubernetes manifest back to the cluster.
pub(crate) async fn api_cluster_apply(
    Json(request): Json<ClusterApplyRequestBody>,
) -> impl IntoResponse {
    match crate::kubecluster::apply_manifest(
        &request.cluster,
        &request.namespace,
        &request.kind,
        request.manifest,
    )
    .await
    {
        Ok(message) => ok_json(message),
        Err(error) => err_internal::<String>(error),
    }
}

/// GET /api/environments - List environments
pub(crate) async fn api_env_list() -> impl IntoResponse {
    use crate::environments::EnvironmentManager;

    let path = EnvironmentManager::default_path();
    match EnvironmentManager::load(&path) {
        Ok(manager) => {
            let envs = manager.list_envs();
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(envs) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/scheduler/utilization - Runtime utilization
pub(crate) async fn api_scheduler_utilization() -> impl IntoResponse {
    use crate::scheduler::Scheduler;

    let path = Scheduler::default_path();
    match Scheduler::load(&path) {
        Ok(scheduler) => {
            let utils = scheduler.utilization_summary();
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(utils) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/scheduler/optimize - Optimization suggestions
pub(crate) async fn api_scheduler_optimize() -> impl IntoResponse {
    use crate::scheduler::Scheduler;

    let path = Scheduler::default_path();
    match Scheduler::load(&path) {
        Ok(scheduler) => {
            let suggestions = scheduler.optimize();
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(suggestions) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/orchestrator/status - Managed workload statuses
pub(crate) async fn api_orchestrator_status() -> impl IntoResponse {
    use crate::orchestrator::Orchestrator;

    let path = Orchestrator::default_path();
    match Orchestrator::load(&path) {
        Ok(orch) => {
            let list = orch.list_workloads();
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(list) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/orchestrator/summary - Health summary
pub(crate) async fn api_orchestrator_summary() -> impl IntoResponse {
    use crate::orchestrator::Orchestrator;

    let path = Orchestrator::default_path();
    match Orchestrator::load(&path) {
        Ok(orch) => {
            let summary = orch.health_summary();
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(summary) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// GET /api/affinity/:class - Runtime affinity recommendation
pub(crate) async fn api_affinity_recommend(Path(class): Path<String>) -> impl IntoResponse {
    use crate::ai::affinity::{AffinityEngine, WorkloadClass};

    let wl_class = match class.parse::<WorkloadClass>() {
        Ok(c) => c,
        Err(e) => {
            return err_bad_request::<serde_json::Value>(e)
        }
    };

    let path = AffinityEngine::default_path();
    match AffinityEngine::load(&path) {
        Ok(engine) => {
            let scores = engine.recommend(&wl_class);
            match serde_json::to_value(scores) {
                Ok(v) => (StatusCode::OK, Json(ApiResponse::success(v))),
                Err(e) => err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            }
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

// GET /api/plugins - List registered plugins
pub(crate) async fn api_plugins_list() -> impl IntoResponse {
    let path = crate::plugin::PluginRegistry::default_path();
    match crate::plugin::PluginRegistry::load(&path) {
        Ok(reg) => {
            let plugins: Vec<&crate::plugin::PluginManifest> = reg.plugins.values().collect();
            match serde_json::to_value(plugins) {
                Ok(v) => (StatusCode::OK, Json(ApiResponse::success(v))),
                Err(e) => err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            }
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

// POST /api/plugins/discover - Discover plugins
pub(crate) async fn api_plugins_discover() -> impl IntoResponse {
    let path = crate::plugin::PluginRegistry::default_path();
    match crate::plugin::PluginRegistry::load(&path) {
        Ok(mut reg) => match reg.discover() {
            Ok(count) => {
                let _ = reg.save(&path);
                (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "discovered": count,
                        "total": reg.plugins.len(),
                    }))),
                )
            }
            Err(e) => err_internal::<serde_json::Value>(e),
        },
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

// GET /api/health/:workload - Health history summary
pub(crate) async fn api_health_summary(Path(workload): Path<String>) -> impl IntoResponse {
    let health_path = crate::health::HealthHistory::default_path();
    match crate::health::HealthHistory::load(&health_path) {
        Ok(history) => {
            let summary = history.summary(&workload);
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    match serde_json::to_value(summary) {
                Ok(v) => v,
                Err(e) => return err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
            },
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

// POST /api/compose/validate - Validate a compose spec
pub(crate) async fn api_compose_validate(
    body: String,
) -> impl IntoResponse {
    use crate::compose;

    match serde_yaml::from_str::<compose::ComposeSpec>(&body) {
        Ok(spec) => match compose::validate(&spec) {
            Ok(()) => {
                let order = compose::resolve_order(&spec).unwrap_or_default();
                (
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "valid": true,
                        "workload_count": spec.workloads.len(),
                        "deploy_order": order,
                    }))),
                )
            }
            Err(e) => {
                let msg = e.to_string();
                (StatusCode::BAD_REQUEST, Json(ApiResponse::error(msg)))
            }
        },
        Err(e) => {
            let msg = format!("Invalid YAML: {}", e);
            (StatusCode::BAD_REQUEST, Json(ApiResponse::error(msg)))
        }
    }
}

/// POST /api/workloads/:name/migrate - Migrate a workload to a different runtime
pub(crate) async fn migrate_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
    Json(request): Json<MigrateWorkloadRequest>,
) -> impl IntoResponse {
    use crate::migration::{MigrationEngine, MigrationPlan, MigrationStrategy};

    let workload_state = match lookup_workload::<String>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let source_runtime = workload_state.runtime;

    let target_runtime = match request.target_runtime.parse::<RuntimeKind>() {
        Ok(rt) => rt,
        Err(e) => {
            return err_bad_request::<String>(e)
        }
    };

    let strategy = match request.strategy.parse::<MigrationStrategy>() {
        Ok(s) => s,
        Err(e) => {
            return err_bad_request::<String>(e)
        }
    };

    let plan = MigrationPlan::new(
        name.clone(),
        source_runtime,
        target_runtime,
        strategy,
        true,
    );

    let migration_start = std::time::Instant::now();
    let engine = MigrationEngine::new(StateStore::default_path());
    let result = match engine.migrate(plan).await {
        Ok(r) => r,
        Err(e) => {
            return err_internal::<String>(format!("Migration failed: {}", e))
        }
    };
    let duration = migration_start.elapsed().as_secs_f64();

    crate::metrics::record_migration(
        &source_runtime.to_string(),
        &target_runtime.to_string(),
        &request.strategy,
        duration,
        result.success,
        result.rollback_performed,
    );

    if result.success {
        // Reload state after migration engine updated it
        let new_state = match StateStore::load(&StateStore::default_path()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    "Migration succeeded but failed to reload state from disk: {}. \
                     In-memory state may be stale — restart the API server to resync.",
                    e
                );
                return err_internal::<String>(format!(
                    "Migration succeeded but state reload failed: {}. Restart API server to resync.",
                    e
                ));
            }
        };
        let mut state = app_state.state.write().await;
        // Sync our in-memory state with what migration engine wrote
        if let Some(updated) = new_state.get(&name) {
            state.upsert(name.clone(), updated.clone());
        }

        emit_sse(&app_state, &ServerEvent::WorkloadChanged {
            name: name.clone(),
            action: "migrated".to_string(),
        });

        (
            StatusCode::OK,
            Json(ApiResponse::success(format!(
                "Workload {} migrated from {} to {} (strategy: {}, duration: {:.1}s)",
                name, source_runtime, target_runtime, request.strategy, duration
            ))),
        )
    } else {
        let error_msg = result
            .error
            .unwrap_or_else(|| "Unknown error".to_string());
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(format!(
                "Migration failed: {}{}",
                error_msg,
                if result.rollback_performed {
                    " (rollback performed)"
                } else {
                    ""
                }
            ))),
        )
    }
}

/// POST /api/workloads/:name/build - Trigger a build for a workload
pub(crate) async fn build_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let workload_state = match lookup_workload::<BuildResponse>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    // Load workload spec (validates path against traversal)
    let spec = match load_spec_safe::<BuildResponse>(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => return e,
    };

    // Build based on runtime
    let runtime = match make_runtime::<BuildResponse>(&workload_state.runtime).await {
        Ok(r) => r,
        Err(e) => return e,
    };
    let image = runtime.build(&spec).await;

    match image {
        Ok(img) => {
            emit_sse(&app_state, &ServerEvent::WorkloadChanged {
                name: name.clone(),
                action: "built".to_string(),
            });

            let response = BuildResponse {
                image_name: img.name.clone(),
                image_tag: img.tag.clone(),
                full_name: img.full_name(),
                runtime: format!("{}", workload_state.runtime),
            };
            ok_json(response)
        }
        Err(e) => err_internal::<BuildResponse>(format!("Build failed: {}", e)),
    }
}

/// POST /api/validate - Validate a workload YAML specification
pub(crate) async fn validate_workload(
    Json(request): Json<ValidateRequest>,
) -> impl IntoResponse {
    // Try to parse the YAML as a Workload spec
    let workload_result: Result<Workload, _> = serde_yaml::from_str(&request.yaml);

    match workload_result {
        Ok(workload) => {
            // YAML parsed successfully, now run validation
            match workload.validate() {
                Ok(()) => {
                    let response = ValidateResponse {
                        valid: true,
                        workload_name: Some(workload.metadata.name),
                        errors: vec![],
                    };
                    ok_json(response)
                }
                Err(e) => {
                    let response = ValidateResponse {
                        valid: false,
                        workload_name: Some(workload.metadata.name),
                        errors: vec![e.to_string()],
                    };
                    ok_json(response)
                }
            }
        }
        Err(e) => {
            let response = ValidateResponse {
                valid: false,
                workload_name: None,
                errors: vec![format!("YAML parse error: {}", e)],
            };
            ok_json(response)
        }
    }
}

/// GET /api/secrets/:name - Get a specific secret's metadata (not raw values)
pub(crate) async fn get_secret(
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::secrets::SecretStore;

    let path = SecretStore::default_path();
    match SecretStore::load(&path) {
        Ok(store) => {
            match store.get_secret(&name) {
                Some(secret) => {
                    let rotation_info = secret.rotation_policy.as_ref().map(|p| {
                        SecretRotationInfo {
                            interval_days: p.interval_days,
                            max_age_days: p.max_age_days,
                            notify_before_days: p.notify_before_days,
                        }
                    });
                    let keys: Vec<String> = secret.data.keys().cloned().collect();
                    let response = SecretMetadataResponse {
                        name: secret.name.clone(),
                        namespace: secret.namespace.clone(),
                        key_count: secret.data.len(),
                        keys,
                        created_at: secret.created_at.clone(),
                        updated_at: secret.updated_at.clone(),
                        needs_rotation: store.needs_rotation(secret),
                        rotation_policy: rotation_info,
                    };
                    match serde_json::to_value(response) {
                        Ok(v) => (StatusCode::OK, Json(ApiResponse::success(v))),
                        Err(e) => err_internal::<serde_json::Value>(format!("Serialization failed: {}", e)),
                    }
                }
                None => (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<serde_json::Value>::error(format!(
                        "Secret '{}' not found",
                        name
                    ))),
                ),
            }
        }
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// DELETE /api/secrets/:name - Delete a specific secret
pub(crate) async fn delete_secret(
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::secrets::SecretStore;

    let path = SecretStore::default_path();
    match SecretStore::load(&path) {
        Ok(mut store) => {
            match store.delete_secret(&name) {
                Some(_) => {
                    // Save the updated store
                    if let Err(e) = store.save(&path) {
                        return err_internal::<String>(format!("Failed to save secret store: {}", e));
                    }
                    (
                        StatusCode::OK,
                        Json(ApiResponse::success(format!("Secret '{}' deleted", name))),
                    )
                }
                None => (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<String>::error(format!(
                        "Secret '{}' not found",
                        name
                    ))),
                ),
            }
        }
        Err(e) => err_internal::<String>(e),
    }
}

/// GET /api/metrics - Export Prometheus metrics as text/plain
pub(crate) async fn get_metrics() -> impl IntoResponse {
    let metrics_output = crate::metrics::gather();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        metrics_output,
    )
}

/// GET /api/events/stream - Server-Sent Events for real-time dashboard updates
pub(crate) async fn sse_events(
    AxumState(state): AxumState<AppState>,
) -> axum::response::Sse<impl futures::stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    let rx = state.event_tx.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .filter_map(|msg| async move {
            match msg {
                Ok(data) => Some(Ok(axum::response::sse::Event::default().data(data))),
                Err(_) => None,
            }
        });
    axum::response::Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
}

/// GET /api/audit/verify — Verify integrity of all audit events
pub(crate) async fn api_audit_verify() -> impl IntoResponse {
    use crate::audit::AuditLog;

    let path = AuditLog::default_path();
    match AuditLog::load(&path) {
        Ok(log) => {
            let events = log.events();
            let total = events.len();
            let mut verified = 0usize;
            let mut tampered = Vec::new();

            for event in events {
                if AuditLog::verify_event_integrity(event) {
                    verified += 1;
                } else {
                    tampered.push(serde_json::json!({
                        "id": event.id,
                        "timestamp": event.timestamp,
                        "action": format!("{}", event.action),
                        "workload": event.workload,
                    }));
                }
            }

            let result = serde_json::json!({
                "total": total,
                "verified": verified,
                "tampered": tampered.len(),
                "integrity": if tampered.is_empty() { "VERIFIED" } else { "TAMPERED" },
                "tampered_events": tampered,
            });

            Json(ApiResponse::success(result))
        }
        Err(e) => {
            Json(ApiResponse::error(format!("Failed to load audit log: {}", e)))
        }
    }
}

// -----------------------------------------------------------------------
// RBAC management handlers
// -----------------------------------------------------------------------

/// List all registered API keys (without exposing raw keys)
pub(crate) async fn rbac_list_keys(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let store = app_state.rbac.read().await;
    let keys: Vec<ApiKeySummary> = store
        .list_keys()
        .iter()
        .map(|entry| ApiKeySummary {
            name: entry.name.clone(),
            role: entry.role.to_string(),
            created_at: entry.created_at.clone(),
        })
        .collect();
    Json(ApiResponse::success(keys))
}

/// Create a new RBAC API key
pub(crate) async fn rbac_create_key(
    AxumState(app_state): AxumState<AppState>,
    Json(request): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    let role = match request.role.to_lowercase().as_str() {
        "admin" => crate::rbac::Role::Admin,
        "operator" => crate::rbac::Role::Operator,
        "viewer" => crate::rbac::Role::Viewer,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<CreateApiKeyResponse>::error(
                    "Invalid role. Must be 'admin', 'operator', or 'viewer'".to_string(),
                )),
            );
        }
    };

    let mut store = app_state.rbac.write().await;
    let plaintext_key = store.create_key(&request.name, role.clone());

    // Save to disk
    if let Err(e) = store.save(&crate::rbac::RbacStore::default_path()) {
        tracing::error!("Failed to save RBAC store: {}", e);
    }

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(CreateApiKeyResponse {
            name: request.name,
            role: role.to_string(),
            key: plaintext_key,
        })),
    )
}

/// Revoke an RBAC API key by name
pub(crate) async fn rbac_revoke_key(
    AxumState(app_state): AxumState<AppState>,
    Json(request): Json<RevokeApiKeyRequest>,
) -> impl IntoResponse {
    let mut store = app_state.rbac.write().await;
    let removed = store.revoke_key(&request.name);

    if removed {
        if let Err(e) = store.save(&crate::rbac::RbacStore::default_path()) {
            tracing::error!("Failed to save RBAC store: {}", e);
        }
        Json(ApiResponse::success(serde_json::json!({
            "message": format!("API key '{}' revoked", request.name),
        })))
    } else {
        Json(ApiResponse::error(format!(
            "API key '{}' not found",
            request.name
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // Dashboard HTML content
    // ---------------------------------------------------------------

    #[test]
    fn test_dashboard_html_is_not_empty() {
        assert!(DASHBOARD_HTML.len() > 100);
    }

    #[test]
    fn test_dashboard_html_is_valid_html_document() {
        assert!(DASHBOARD_HTML.contains("<!DOCTYPE html>"));
        assert!(DASHBOARD_HTML.contains("<html"));
        assert!(DASHBOARD_HTML.contains("</html>"));
    }

    #[test]
    fn test_dashboard_html_has_head_section() {
        assert!(DASHBOARD_HTML.contains("<head>"));
        assert!(DASHBOARD_HTML.contains("</head>"));
    }

    #[test]
    fn test_dashboard_html_has_title() {
        assert!(DASHBOARD_HTML.contains("<title>"));
        assert!(DASHBOARD_HTML.contains("Aether"));
    }

    #[test]
    fn test_dashboard_html_has_charset() {
        assert!(DASHBOARD_HTML.contains("charset"));
        assert!(DASHBOARD_HTML.contains("UTF-8"));
    }

    #[test]
    fn test_dashboard_html_has_viewport_meta() {
        assert!(DASHBOARD_HTML.contains("viewport"));
    }

    // ---------------------------------------------------------------
    // RBAC API types tests
    // ---------------------------------------------------------------

    #[test]
    fn test_create_api_key_request_deserialization() {
        let json = r#"{"name": "my-key", "role": "operator"}"#;
        let req: CreateApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "my-key");
        assert_eq!(req.role, "operator");
    }

    #[test]
    fn test_create_api_key_response_serialization() {
        let resp = CreateApiKeyResponse {
            name: "test-key".to_string(),
            role: "admin".to_string(),
            key: "aether_abc123".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("test-key"));
        assert!(json.contains("aether_abc123"));
    }

    #[test]
    fn test_revoke_api_key_request_deserialization() {
        let json = r#"{"name": "old-key"}"#;
        let req: RevokeApiKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "old-key");
    }

    #[test]
    fn test_api_key_summary_serialization() {
        let summary = ApiKeySummary {
            name: "ops-key".to_string(),
            role: "operator".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("ops-key"));
        assert!(json.contains("operator"));
    }
}
