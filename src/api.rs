//! REST API Server for Orchestr8
//!
//! Provides HTTP endpoints for workload management

use crate::adapters::{KubeVirtRuntime, KubernetesRuntime, Metal3Runtime, PodmanRuntime};
use crate::config::Config;
use crate::engine::Engine;
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use crate::{backup, cost, Runtime};
use axum::{
    extract::{Path, State as AxumState},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono;

/// API Server configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub state_path: PathBuf,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            state_path: StateStore::default_path(),
        }
    }
}

/// Shared application state
#[derive(Clone)]
struct AppState {
    state: Arc<RwLock<StateStore>>,
}

/// API Response wrapper
#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Workload creation request
#[derive(Debug, Deserialize)]
struct CreateWorkloadRequest {
    spec: Workload,
    runtime: Option<String>,
}

/// Workload response
#[derive(Debug, Serialize)]
struct WorkloadResponse {
    name: String,
    runtime: String,
    image: String,
    status: String,
    created_at: String,
}

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Embedded dashboard HTML
const DASHBOARD_HTML: &str = include_str!("../web/index.html");

/// Start the API server
pub async fn start_server(config: ApiConfig) -> anyhow::Result<()> {
    // Load state
    let state_store = StateStore::load(&config.state_path)?;
    let app_state = AppState {
        state: Arc::new(RwLock::new(state_store)),
    };

    // Build router
    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/health", get(health_check))
        .route("/api/workloads", get(list_workloads))
        .route("/api/workloads", post(create_workload))
        .route("/api/workloads/:name", get(get_workload))
        .route("/api/workloads/:name", delete(delete_workload))
        .route("/api/workloads/:name/logs", get(get_logs))
        .route("/api/workloads/:name/start", post(start_workload))
        .route("/api/workloads/:name/stop", post(stop_workload))
        .route("/api/cost", post(estimate_cost))
        .route("/api/backups", get(list_backups))
        .route("/api/backups", post(create_backup))
        .route("/api/ai/recommend", post(ai_recommend))
        .route("/api/ai/profile/:name", get(ai_profile))
        .route("/api/ai/analyze/:name", get(ai_analyze_logs))
        .route("/api/drift/:name", get(api_drift_check))
        .route("/api/policy/check", post(api_policy_check))
        .route("/api/dependencies", get(api_deps_show))
        .route("/api/dependencies", post(api_deps_add))
        .route("/api/audit", get(api_audit_list))
        .route("/api/templates", get(api_template_list))
        .route("/api/templates/:name", post(api_template_generate))
        .route("/api/sla/:workload", get(api_sla_check))
        .with_state(app_state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port)
        .parse::<SocketAddr>()?;

    println!("🌐 Starting API server on http://{}", addr);
    println!("📊 Dashboard: http://{}", addr);
    println!("📋 API Health: http://{}/health", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// GET / - Serve the web dashboard
async fn serve_dashboard() -> impl IntoResponse {
    Html(DASHBOARD_HTML)
}

/// GET /health - Health check endpoint
async fn health_check() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Json(ApiResponse::success(response))
}

/// GET /api/workloads - List all workloads
async fn list_workloads(
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
async fn create_workload(
    AxumState(app_state): AxumState<AppState>,
    Json(request): Json<CreateWorkloadRequest>,
) -> impl IntoResponse {
    // Select runtime
    let runtime_kind = if let Some(runtime_name) = request.runtime {
        match runtime_name.to_lowercase().as_str() {
            "podman" => RuntimeKind::Podman,
            "kubernetes" => RuntimeKind::Kubernetes,
            "kubevirt" => RuntimeKind::KubeVirt,
            "metal3" => RuntimeKind::Metal3,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<String>::error(format!(
                        "Invalid runtime: {}",
                        runtime_name
                    ))),
                )
            }
        }
    } else {
        // Use decision engine
        let engine = Engine::new();
        match engine.decide(&request.spec) {
            Ok(runtime) => runtime,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<String>::error(e.to_string())),
                )
            }
        }
    };

    // Build and run based on runtime
    let instance = match runtime_kind {
        RuntimeKind::Podman => {
            let runtime = match PodmanRuntime::new() {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            };
            match runtime.build(&request.spec).await {
                Ok(image) => match runtime.run(&image, &request.spec).await {
                    Ok(inst) => inst,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(e.to_string())),
                        )
                    }
                },
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            }
        }
        RuntimeKind::Kubernetes => {
            let runtime = match KubernetesRuntime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            };
            match runtime.build(&request.spec).await {
                Ok(image) => match runtime.run(&image, &request.spec).await {
                    Ok(inst) => inst,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(e.to_string())),
                        )
                    }
                },
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            }
        }
        RuntimeKind::KubeVirt => {
            let runtime = match KubeVirtRuntime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            };
            match runtime.build(&request.spec).await {
                Ok(image) => match runtime.run(&image, &request.spec).await {
                    Ok(inst) => inst,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(e.to_string())),
                        )
                    }
                },
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            }
        }
        RuntimeKind::Metal3 => {
            let runtime = match Metal3Runtime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            };
            match runtime.build(&request.spec).await {
                Ok(image) => match runtime.run(&image, &request.spec).await {
                    Ok(inst) => inst,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(e.to_string())),
                        )
                    }
                },
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            }
        }
    };

    // Save state
    let mut state = app_state.state.write().await;
    state.upsert(
        request.spec.metadata.name.clone(),
        WorkloadState {
            name: request.spec.metadata.name.clone(),
            runtime: runtime_kind,
            instance,
            spec_path: PathBuf::from("api_created"),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    );

    // Persist to disk
    if let Err(e) = state.save(&StateStore::default_path()) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        );
    }

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(format!(
            "Workload {} created",
            request.spec.metadata.name
        ))),
    )
}

/// GET /api/workloads/:name - Get workload details
async fn get_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let state = app_state.state.read().await;

    match state.get(&name) {
        Some(workload) => {
            let response = WorkloadResponse {
                name: workload.name.clone(),
                runtime: format!("{:?}", workload.runtime),
                image: workload.instance.image.clone(),
                status: format!("deployed ({})", workload.runtime),
                created_at: workload.created_at.clone(),
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<WorkloadResponse>::error(format!(
                "Workload {} not found",
                name
            ))),
        ),
    }
}

/// DELETE /api/workloads/:name - Delete a workload
async fn delete_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut state = app_state.state.write().await;

    match state.remove(&name) {
        Some(_) => {
            // Persist to disk
            if let Err(e) = state.save(&StateStore::default_path()) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<String>::error(e.to_string())),
                );
            }

            (
                StatusCode::OK,
                Json(ApiResponse::success(format!("Workload {} deleted", name))),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<String>::error(format!("Workload {} not found", name))),
        ),
    }
}

/// GET /api/workloads/:name/logs - Get workload logs
async fn get_logs(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let state = app_state.state.read().await;

    match state.get(&name) {
        Some(workload) => {
            let logs = match workload.runtime {
                RuntimeKind::Podman => {
                    let runtime = match PodmanRuntime::new() {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    };
                    match runtime.logs(&workload.instance, false).await {
                        Ok(logs) => logs,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    }
                }
                RuntimeKind::Kubernetes => {
                    let runtime = match KubernetesRuntime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    };
                    match runtime.logs(&workload.instance, false).await {
                        Ok(logs) => logs,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    }
                }
                RuntimeKind::KubeVirt => {
                    let runtime = match KubeVirtRuntime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    };
                    match runtime.logs(&workload.instance, false).await {
                        Ok(logs) => logs,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    }
                }
                RuntimeKind::Metal3 => {
                    let runtime = match Metal3Runtime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    };
                    match runtime.logs(&workload.instance, false).await {
                        Ok(logs) => logs,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    }
                }
            };

            (StatusCode::OK, Json(ApiResponse::success(logs)))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<String>::error(format!(
                "Workload {} not found",
                name
            ))),
        ),
    }
}

/// POST /api/workloads/:name/start - Start a workload
async fn start_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let state = app_state.state.read().await;

    let workload_state = match state.get(&name) {
        Some(w) => w.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<String>::error(format!(
                    "Workload {} not found",
                    name
                ))),
            )
        }
    };

    // Load workload spec from the stored path
    let spec = match Workload::from_file(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(format!(
                    "Failed to load workload spec: {}",
                    e
                ))),
            )
        }
    };

    // Build and run on the same runtime
    let instance = match workload_state.runtime {
        RuntimeKind::Podman => {
            let runtime = match PodmanRuntime::new() {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            };
            match runtime.build(&spec).await {
                Ok(image) => match runtime.run(&image, &spec).await {
                    Ok(inst) => inst,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(e.to_string())),
                        )
                    }
                },
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            }
        }
        RuntimeKind::Kubernetes => {
            let runtime = match KubernetesRuntime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            };
            match runtime.build(&spec).await {
                Ok(image) => match runtime.run(&image, &spec).await {
                    Ok(inst) => inst,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(e.to_string())),
                        )
                    }
                },
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            }
        }
        RuntimeKind::KubeVirt => {
            let runtime = match KubeVirtRuntime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            };
            match runtime.build(&spec).await {
                Ok(image) => match runtime.run(&image, &spec).await {
                    Ok(inst) => inst,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(e.to_string())),
                        )
                    }
                },
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            }
        }
        RuntimeKind::Metal3 => {
            let runtime = match Metal3Runtime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            };
            match runtime.build(&spec).await {
                Ok(image) => match runtime.run(&image, &spec).await {
                    Ok(inst) => inst,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(e.to_string())),
                        )
                    }
                },
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<String>::error(e.to_string())),
                    )
                }
            }
        }
    };

    // Update state with the new instance
    drop(state);
    let mut state = app_state.state.write().await;
    state.upsert(
        name.clone(),
        WorkloadState {
            name: name.clone(),
            runtime: workload_state.runtime,
            instance,
            spec_path: workload_state.spec_path,
            created_at: workload_state.created_at,
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    );

    if let Err(e) = state.save(&StateStore::default_path()) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(format!("Workload {} started", name))),
    )
}

/// POST /api/workloads/:name/stop - Stop a workload
async fn stop_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let state = app_state.state.read().await;

    match state.get(&name) {
        Some(workload) => {
            let result = match workload.runtime {
                RuntimeKind::Podman => {
                    let runtime = match PodmanRuntime::new() {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    };
                    runtime.stop(&workload.instance).await
                }
                RuntimeKind::Kubernetes => {
                    let runtime = match KubernetesRuntime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    };
                    runtime.stop(&workload.instance).await
                }
                RuntimeKind::KubeVirt => {
                    let runtime = match KubeVirtRuntime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    };
                    runtime.stop(&workload.instance).await
                }
                RuntimeKind::Metal3 => {
                    let runtime = match Metal3Runtime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<String>::error(e.to_string())),
                            )
                        }
                    };
                    runtime.stop(&workload.instance).await
                }
            };

            match result {
                Ok(_) => (
                    StatusCode::OK,
                    Json(ApiResponse::success(format!("Workload {} stopped", name))),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<String>::error(e.to_string())),
                ),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<String>::error(format!(
                "Workload {} not found",
                name
            ))),
        ),
    }
}

/// POST /api/cost - Estimate costs for a workload
async fn estimate_cost(Json(spec): Json<Workload>) -> impl IntoResponse {
    match cost::estimate_all_providers(&spec) {
        Ok(estimates) => (StatusCode::OK, Json(ApiResponse::success(estimates))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<cost::CostEstimate>>::error(e.to_string())),
        ),
    }
}

/// GET /api/backups - List all backups
async fn list_backups() -> impl IntoResponse {
    let manager = backup::BackupManager::new(backup::BackupManager::default_dir());

    match manager.list_backups() {
        Ok(backups) => (StatusCode::OK, Json(ApiResponse::success(backups))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<PathBuf>>::error(e.to_string())),
        ),
    }
}

/// POST /api/backups - Create a new backup
#[derive(Debug, Deserialize)]
struct CreateBackupRequest {
    name: Option<String>,
    description: Option<String>,
}

async fn create_backup(
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        ),
    }
}

/// POST /api/ai/recommend - AI-powered runtime recommendation
async fn ai_recommend(Json(spec): Json<Workload>) -> impl IntoResponse {
    use crate::ai::scoring::ScoringEngine;
    use crate::config::Config;

    let config = Config::load();
    let engine = ScoringEngine::new(config.engine);
    let result = engine.score(&spec);

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

/// GET /api/ai/profile/:name - Profile a deployed workload
async fn ai_profile(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::ai::profiler::Profiler;

    let state = app_state.state.read().await;

    match state.get(&name) {
        Some(workload_state) => {
            let spec = match Workload::from_file(&workload_state.spec_path) {
                Ok(s) => s,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<serde_json::Value>::error(format!(
                            "Failed to load spec: {}",
                            e
                        ))),
                    )
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
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(format!(
                "Workload {} not found",
                name
            ))),
        ),
    }
}

/// GET /api/ai/analyze/:name - Analyze workload logs
async fn ai_analyze_logs(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::ai::analyzer::LogAnalyzer;

    let state = app_state.state.read().await;

    match state.get(&name) {
        Some(workload) => {
            // Fetch logs
            let logs = match workload.runtime {
                RuntimeKind::Podman => {
                    let runtime = match PodmanRuntime::new() {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                            )
                        }
                    };
                    match runtime.logs(&workload.instance, false).await {
                        Ok(l) => l,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                            )
                        }
                    }
                }
                RuntimeKind::Kubernetes => {
                    let runtime = match KubernetesRuntime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                            )
                        }
                    };
                    match runtime.logs(&workload.instance, false).await {
                        Ok(l) => l,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                            )
                        }
                    }
                }
                RuntimeKind::KubeVirt => {
                    let runtime = match KubeVirtRuntime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                            )
                        }
                    };
                    match runtime.logs(&workload.instance, false).await {
                        Ok(l) => l,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                            )
                        }
                    }
                }
                RuntimeKind::Metal3 => {
                    let runtime = match Metal3Runtime::new().await {
                        Ok(r) => r,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                            )
                        }
                    };
                    match runtime.logs(&workload.instance, false).await {
                        Ok(l) => l,
                        Err(e) => {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                            )
                        }
                    }
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
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(format!(
                "Workload {} not found",
                name
            ))),
        ),
    }
}

/// GET /api/drift/:name - Check drift for a workload
async fn api_drift_check(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::drift::DriftDetector;

    let state = app_state.state.read().await;

    match state.get(&name) {
        Some(workload_state) => {
            let spec = match Workload::from_file(&workload_state.spec_path) {
                Ok(s) => s,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<serde_json::Value>::error(format!(
                            "Failed to load spec: {}",
                            e
                        ))),
                    )
                }
            };

            let detector = DriftDetector::new();
            let report = detector.detect(&spec, workload_state);

            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    serde_json::to_value(report).unwrap_or_default(),
                )),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(format!(
                "Workload {} not found",
                name
            ))),
        ),
    }
}

/// POST /api/policy/check - Check workload against policies
#[derive(Debug, Deserialize)]
struct PolicyCheckRequest {
    spec: Workload,
    policy_set: Option<String>,
}

async fn api_policy_check(Json(request): Json<PolicyCheckRequest>) -> impl IntoResponse {
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
async fn api_deps_show() -> impl IntoResponse {
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
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// POST /api/dependencies - Add a dependency
#[derive(Debug, Deserialize)]
struct AddDependencyRequest {
    workload: String,
    dependency: String,
}

async fn api_deps_add(Json(request): Json<AddDependencyRequest>) -> impl IntoResponse {
    use crate::dependencies::DependencyGraph;

    let graph_path = DependencyGraph::default_path();
    match DependencyGraph::load(&graph_path) {
        Ok(mut graph) => {
            graph.add_dependency(&request.workload, &request.dependency);
            if let Err(e) = graph.save(&graph_path) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<String>::error(e.to_string())),
                );
            }
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(format!(
                    "Dependency added: {} -> {}",
                    request.workload, request.dependency
                ))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        ),
    }
}

/// GET /api/audit - List audit events
async fn api_audit_list() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/templates - List available templates
async fn api_template_list() -> impl IntoResponse {
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
#[derive(Debug, Deserialize)]
struct TemplateRequest {
    workload_name: Option<String>,
    owner: Option<String>,
    project: Option<String>,
    registry: Option<String>,
    cpu: Option<String>,
    memory: Option<String>,
    port: Option<u16>,
    replicas: Option<u32>,
}

async fn api_template_generate(
    Path(name): Path<String>,
    Json(request): Json<TemplateRequest>,
) -> impl IntoResponse {
    use crate::templates::{self, TemplateKind, TemplateParams};

    let kind = match name.as_str() {
        "web-app" => TemplateKind::WebApp,
        "rest-api" => TemplateKind::RestApi,
        "database" => TemplateKind::Database,
        "cache" => TemplateKind::Cache,
        "worker" => TemplateKind::Worker,
        "cron-job" => TemplateKind::CronJob,
        "ml-training" => TemplateKind::MlTraining,
        "microservice" => TemplateKind::Microservice,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(format!(
                    "Unknown template: {}",
                    name
                ))),
            )
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
async fn api_sla_check(Path(workload): Path<String>) -> impl IntoResponse {
    use crate::sla::{SlaEngine, SlaTarget};

    let sla_path = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".orchestr8/sla.json")
    };

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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
            )
        }
    };

    let targets: Vec<SlaTarget> = match serde_json::from_str(&content) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("hello".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("hello".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<String>::error("something went wrong".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("something went wrong".to_string()));
    }

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "0.2.0".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("0.2.0"));
    }

    #[test]
    fn test_workload_response_serialization() {
        let response = WorkloadResponse {
            name: "my-app".to_string(),
            runtime: "Podman".to_string(),
            image: "my-app:latest".to_string(),
            status: "running".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("my-app"));
        assert!(json.contains("Podman"));
    }
}
