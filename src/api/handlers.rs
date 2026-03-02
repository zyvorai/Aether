//! API handler functions

use super::types::*;
use crate::config::Config;
use crate::engine::Engine;
use crate::runtime::{self, RuntimeKind};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use crate::{backup, cost, Runtime};

use axum::{
    extract::{Path, State as AxumState},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use axum::http::header;
use std::path::PathBuf;

/// Look up a workload by name from state, returning a cloned WorkloadState
/// or an HTTP 404 error response.
async fn lookup_workload<T: serde::Serialize>(
    app_state: &AppState,
    name: &str,
) -> Result<WorkloadState, (StatusCode, Json<ApiResponse<T>>)> {
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

/// Embedded dashboard HTML
pub(crate) const DASHBOARD_HTML: &str = include_str!("../../web/index.html");

/// GET / - Serve the web dashboard
pub(crate) async fn serve_dashboard() -> impl IntoResponse {
    Html(DASHBOARD_HTML)
}

/// GET /health - Health check endpoint
pub(crate) async fn health_check() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Json(ApiResponse::success(response))
}

/// GET /api/workloads - List all workloads
pub(crate) async fn list_workloads(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let state = app_state.state.read().await;
    let workloads: Vec<WorkloadResponse> = state
        .list()
        .iter()
        .map(|w| WorkloadResponse {
            name: w.name.clone(),
            runtime: format!("{:?}", w.runtime),
            image: w.instance.image.clone(),
            status: format!("deployed ({})", w.runtime),
            created_at: w.created_at.clone(),
        })
        .collect();

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

    // Load workload spec from the stored path
    let spec = match Workload::from_file(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => {
            return err_internal::<String>(format!("Failed to load workload spec: {}", e))
        }
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

    // Update state with the new instance
    let mut state = app_state.state.write().await;
    state.upsert(
        name.clone(),
        WorkloadState {
            name: name.clone(),
            runtime: workload_state.runtime,
            instance,
            spec_path: workload_state.spec_path,
            created_at: workload_state.created_at,
            updated_at: crate::resources::now_rfc3339(),
        },
    );

    if let Err(e) = state.save(&StateStore::default_path()) {
        return err_internal::<String>(e);
    }

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
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(format!("Workload {} stopped", name))),
        ),
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

    let spec = match Workload::from_file(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => {
            return err_internal::<serde_json::Value>(format!("Failed to load spec: {}", e))
        }
    };

    let config = Config::load();
    let profiler = Profiler::new(config.profiler.waste_threshold);
    let profile = profiler.profile(&spec, Some(workload_state.runtime));

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::to_value(profile).unwrap_or_default())),
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

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::to_value(analysis).unwrap_or_default())),
    )
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

    let spec = match Workload::from_file(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => {
            return err_internal::<MigrationAdviceResponse>(format!("Failed to load workload spec: {}", e))
        }
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

    let spec = match Workload::from_file(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => {
            return err_internal::<serde_json::Value>(format!("Failed to load spec: {}", e))
        }
    };

    let detector = DriftDetector::new();
    let report = detector.detect(&spec, &workload_state);

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            serde_json::to_value(report).unwrap_or_default(),
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
            serde_json::to_value(result).unwrap_or_default(),
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
            serde_json::to_value(templates).unwrap_or_default(),
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
            serde_json::to_value(spec).unwrap_or_default(),
        )),
    )
}

/// GET /api/sla/:workload - Check SLA compliance
pub(crate) async fn api_sla_check(Path(workload): Path<String>) -> impl IntoResponse {
    use crate::sla::{SlaEngine, SlaTarget};

    let sla_path = crate::resources::orchestr8_path("sla.json");

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
                serde_json::to_value(target).unwrap_or_default(),
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
                    serde_json::to_value(summaries).unwrap_or_default(),
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
                    serde_json::to_value(events).unwrap_or_default(),
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
                    serde_json::to_value(summary).unwrap_or_default(),
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
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
                    serde_json::to_value(envs).unwrap_or_default(),
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
                    serde_json::to_value(utils).unwrap_or_default(),
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
                    serde_json::to_value(suggestions).unwrap_or_default(),
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
                    serde_json::to_value(list).unwrap_or_default(),
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
                    serde_json::to_value(summary).unwrap_or_default(),
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
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    serde_json::to_value(scores).unwrap_or_default(),
                )),
            )
        }
        Err(e) => err_internal::<serde_json::Value>(e),
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

    let plan = MigrationPlan {
        workload_name: name.clone(),
        source_runtime,
        target_runtime,
        strategy,
        validation_delay: std::time::Duration::from_secs(30),
        rollback_on_failure: true,
    };

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
            Err(_) => {
                return (
                    StatusCode::OK,
                    Json(ApiResponse::success(format!(
                        "Workload {} migrated to {}",
                        name, target_runtime
                    ))),
                )
            }
        };
        let mut state = app_state.state.write().await;
        // Sync our in-memory state with what migration engine wrote
        if let Some(updated) = new_state.get(&name) {
            state.upsert(name.clone(), updated.clone());
        }

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

    // Load workload spec from the stored path
    let spec = match Workload::from_file(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => {
            return err_internal::<BuildResponse>(format!("Failed to load workload spec: {}", e))
        }
    };

    // Build based on runtime
    let runtime = match make_runtime::<BuildResponse>(&workload_state.runtime).await {
        Ok(r) => r,
        Err(e) => return e,
    };
    let image = runtime.build(&spec).await;

    match image {
        Ok(img) => {
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
                        needs_rotation: store.list().iter().any(|s| s.name == name && s.needs_rotation),
                        rotation_policy: rotation_info,
                    };
                    (
                        StatusCode::OK,
                        Json(ApiResponse::success(
                            serde_json::to_value(response).unwrap_or_default(),
                        )),
                    )
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // Dashboard HTML content
    // ---------------------------------------------------------------

    #[test]
    fn test_dashboard_html_is_not_empty() {
        assert!(!DASHBOARD_HTML.is_empty());
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
        assert!(DASHBOARD_HTML.contains("Orchestr8"));
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
}
