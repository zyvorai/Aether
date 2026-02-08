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
use axum::http::header;
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

/// Migrate workload request
#[derive(Debug, Deserialize)]
struct MigrateWorkloadRequest {
    /// Target runtime (podman, kubernetes, kubevirt, metal3)
    target_runtime: String,
    /// Migration strategy (immediate, blue-green, rolling)
    #[serde(default = "default_strategy")]
    strategy: String,
}

fn default_strategy() -> String {
    "blue-green".to_string()
}

/// Validate workload request
#[derive(Debug, Deserialize)]
struct ValidateRequest {
    yaml: String,
}

/// Validation response
#[derive(Debug, Serialize)]
struct ValidateResponse {
    valid: bool,
    workload_name: Option<String>,
    errors: Vec<String>,
}

/// Build response
#[derive(Debug, Serialize)]
struct BuildResponse {
    image_name: String,
    image_tag: String,
    full_name: String,
    runtime: String,
}

/// Secret metadata response (no raw values exposed)
#[derive(Debug, Serialize)]
struct SecretMetadataResponse {
    name: String,
    namespace: String,
    key_count: usize,
    keys: Vec<String>,
    created_at: String,
    updated_at: String,
    needs_rotation: bool,
    rotation_policy: Option<SecretRotationInfo>,
}

/// Rotation info for API response
#[derive(Debug, Serialize)]
struct SecretRotationInfo {
    interval_days: u32,
    max_age_days: u32,
    notify_before_days: u32,
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
        .route("/api/workloads/:name/migrate", post(migrate_workload))
        .route("/api/workloads/:name/build", post(build_workload))
        .route("/api/validate", post(validate_workload))
        .route("/api/secrets/:name", get(get_secret))
        .route("/api/secrets/:name", delete(delete_secret))
        .route("/api/metrics", get(get_metrics))
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
        .route("/api/secrets", get(api_secrets_list))
        .route("/api/events", get(api_events_list))
        .route("/api/events/summary", get(api_events_summary))
        .route("/api/environments", get(api_env_list))
        .route("/api/scheduler/utilization", get(api_scheduler_utilization))
        .route("/api/scheduler/optimize", get(api_scheduler_optimize))
        .route("/api/orchestrator/status", get(api_orchestrator_status))
        .route("/api/orchestrator/summary", get(api_orchestrator_summary))
        .route("/api/affinity/:class", get(api_affinity_recommend))
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

/// GET /api/secrets - List secrets
async fn api_secrets_list() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/events - List recent events
async fn api_events_list() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/events/summary - Event summary
async fn api_events_summary() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/environments - List environments
async fn api_env_list() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/scheduler/utilization - Runtime utilization
async fn api_scheduler_utilization() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/scheduler/optimize - Optimization suggestions
async fn api_scheduler_optimize() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/orchestrator/status - Managed workload statuses
async fn api_orchestrator_status() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/orchestrator/summary - Health summary
async fn api_orchestrator_summary() -> impl IntoResponse {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// GET /api/affinity/:class - Runtime affinity recommendation
async fn api_affinity_recommend(Path(class): Path<String>) -> impl IntoResponse {
    use crate::ai::affinity::{AffinityEngine, WorkloadClass};

    let wl_class = match class.as_str() {
        "web-service" => WorkloadClass::WebService,
        "api-backend" => WorkloadClass::ApiBackend,
        "database" => WorkloadClass::Database,
        "cache" => WorkloadClass::Cache,
        "batch-job" => WorkloadClass::BatchJob,
        "ml-training" => WorkloadClass::MlTraining,
        "worker" => WorkloadClass::Worker,
        "microservice" => WorkloadClass::Microservice,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(format!(
                    "Unknown workload class: {}",
                    class
                ))),
            )
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// POST /api/workloads/:name/migrate - Migrate a workload to a different runtime
async fn migrate_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
    Json(request): Json<MigrateWorkloadRequest>,
) -> impl IntoResponse {
    use crate::migration::{MigrationEngine, MigrationPlan, MigrationStrategy};

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
    drop(state);

    let source_runtime = workload_state.runtime;

    let target_runtime = match request.target_runtime.to_lowercase().as_str() {
        "podman" | "container" => RuntimeKind::Podman,
        "kube" | "kubernetes" => RuntimeKind::Kubernetes,
        "kubevirt" | "vm" => RuntimeKind::KubeVirt,
        "metal" | "metal3" => RuntimeKind::Metal3,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<String>::error(format!(
                    "Unknown target runtime: {}",
                    request.target_runtime
                ))),
            )
        }
    };

    let strategy = match request.strategy.to_lowercase().as_str() {
        "immediate" => MigrationStrategy::Immediate,
        "blue-green" => MigrationStrategy::BlueGreen,
        "rolling" => MigrationStrategy::Rolling,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<String>::error(format!(
                    "Unknown strategy: {}. Use immediate, blue-green, or rolling.",
                    request.strategy
                ))),
            )
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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(format!(
                    "Migration failed: {}",
                    e
                ))),
            )
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
async fn build_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let state = app_state.state.read().await;

    let workload_state = match state.get(&name) {
        Some(w) => w.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<BuildResponse>::error(format!(
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
                Json(ApiResponse::<BuildResponse>::error(format!(
                    "Failed to load workload spec: {}",
                    e
                ))),
            )
        }
    };

    // Build based on runtime
    let image = match workload_state.runtime {
        RuntimeKind::Podman => {
            let runtime = match PodmanRuntime::new() {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<BuildResponse>::error(e.to_string())),
                    )
                }
            };
            runtime.build(&spec).await
        }
        RuntimeKind::Kubernetes => {
            let runtime = match KubernetesRuntime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<BuildResponse>::error(e.to_string())),
                    )
                }
            };
            runtime.build(&spec).await
        }
        RuntimeKind::KubeVirt => {
            let runtime = match KubeVirtRuntime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<BuildResponse>::error(e.to_string())),
                    )
                }
            };
            runtime.build(&spec).await
        }
        RuntimeKind::Metal3 => {
            let runtime = match Metal3Runtime::new().await {
                Ok(r) => r,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<BuildResponse>::error(e.to_string())),
                    )
                }
            };
            runtime.build(&spec).await
        }
    };

    match image {
        Ok(img) => {
            let response = BuildResponse {
                image_name: img.name.clone(),
                image_tag: img.tag.clone(),
                full_name: img.full_name(),
                runtime: format!("{}", workload_state.runtime),
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<BuildResponse>::error(format!(
                "Build failed: {}",
                e
            ))),
        ),
    }
}

/// POST /api/validate - Validate a workload YAML specification
async fn validate_workload(
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
                    (StatusCode::OK, Json(ApiResponse::success(response)))
                }
                Err(e) => {
                    let response = ValidateResponse {
                        valid: false,
                        workload_name: Some(workload.metadata.name),
                        errors: vec![e.to_string()],
                    };
                    (StatusCode::OK, Json(ApiResponse::success(response)))
                }
            }
        }
        Err(e) => {
            let response = ValidateResponse {
                valid: false,
                workload_name: None,
                errors: vec![format!("YAML parse error: {}", e)],
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
    }
}

/// GET /api/secrets/:name - Get a specific secret's metadata (not raw values)
async fn get_secret(
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
        ),
    }
}

/// DELETE /api/secrets/:name - Delete a specific secret
async fn delete_secret(
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
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<String>::error(format!(
                                "Failed to save secret store: {}",
                                e
                            ))),
                        );
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        ),
    }
}

/// GET /api/metrics - Export Prometheus metrics as text/plain
async fn get_metrics() -> impl IntoResponse {
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
    use serde_json;

    // ---------------------------------------------------------------
    // ApiConfig defaults
    // ---------------------------------------------------------------

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_api_config_default_state_path() {
        let config = ApiConfig::default();
        assert_eq!(config.state_path, StateStore::default_path());
    }

    #[test]
    fn test_api_config_custom_values() {
        let config = ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            state_path: PathBuf::from("/tmp/test-state.json"),
        };
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.state_path, PathBuf::from("/tmp/test-state.json"));
    }

    // ---------------------------------------------------------------
    // ApiResponse construction
    // ---------------------------------------------------------------

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
    fn test_api_response_success_with_integer() {
        let response = ApiResponse::success(42u64);
        assert!(response.success);
        assert_eq!(response.data, Some(42u64));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_success_with_vec() {
        let data = vec!["a".to_string(), "b".to_string()];
        let response = ApiResponse::success(data.clone());
        assert!(response.success);
        assert_eq!(response.data, Some(data));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error_with_empty_message() {
        let response = ApiResponse::<String>::error(String::new());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some(String::new()));
    }

    // ---------------------------------------------------------------
    // ApiResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_success_serialization() {
        let response = ApiResponse::success("data here".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"], "data here");
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_api_response_error_serialization() {
        let response = ApiResponse::<String>::error("bad request".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "bad request");
    }

    #[test]
    fn test_api_response_success_serialization_has_all_fields() {
        let response = ApiResponse::success("ok".to_string());
        let json_str = serde_json::to_string(&response).unwrap();

        assert!(json_str.contains("\"success\""));
        assert!(json_str.contains("\"data\""));
        assert!(json_str.contains("\"error\""));
    }

    #[test]
    fn test_api_response_nested_struct_serialization() {
        let health = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
        };
        let response = ApiResponse::success(health);
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "ok");
        assert_eq!(json["data"]["version"], "1.0.0");
        assert!(json["error"].is_null());
    }

    // ---------------------------------------------------------------
    // HealthResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "0.2.0".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"version\":\"0.2.0\""));
    }

    #[test]
    fn test_health_response_serialization_roundtrip_via_value() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "3.1.4".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(value["status"], "healthy");
        assert_eq!(value["version"], "3.1.4");
    }

    #[test]
    fn test_health_response_contains_exactly_two_fields() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("status"));
        assert!(obj.contains_key("version"));
    }

    // ---------------------------------------------------------------
    // WorkloadResponse serialization
    // ---------------------------------------------------------------

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

    #[test]
    fn test_workload_response_all_fields_present() {
        let response = WorkloadResponse {
            name: "web-server".to_string(),
            runtime: "Kubernetes".to_string(),
            image: "registry.io/web:v2".to_string(),
            status: "deployed (kubernetes)".to_string(),
            created_at: "2026-06-15T12:30:00Z".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();

        assert_eq!(obj.len(), 5);
        assert_eq!(value["name"], "web-server");
        assert_eq!(value["runtime"], "Kubernetes");
        assert_eq!(value["image"], "registry.io/web:v2");
        assert_eq!(value["status"], "deployed (kubernetes)");
        assert_eq!(value["created_at"], "2026-06-15T12:30:00Z");
    }

    #[test]
    fn test_workload_response_serialization_produces_valid_json() {
        let response = WorkloadResponse {
            name: "test".to_string(),
            runtime: "Metal3".to_string(),
            image: "img:latest".to_string(),
            status: "deployed".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let json_str = serde_json::to_string(&response).unwrap();
        let reparsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(reparsed["name"], "test");
    }

    // ---------------------------------------------------------------
    // BuildResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_build_response_serialization() {
        let response = BuildResponse {
            image_name: "my-app".to_string(),
            image_tag: "v1.0".to_string(),
            full_name: "my-app:v1.0".to_string(),
            runtime: "podman".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["image_name"], "my-app");
        assert_eq!(value["image_tag"], "v1.0");
        assert_eq!(value["full_name"], "my-app:v1.0");
        assert_eq!(value["runtime"], "podman");
    }

    #[test]
    fn test_build_response_has_four_fields() {
        let response = BuildResponse {
            image_name: "svc".to_string(),
            image_tag: "latest".to_string(),
            full_name: "svc:latest".to_string(),
            runtime: "kubernetes".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 4);
    }

    // ---------------------------------------------------------------
    // ValidateRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_request_deserialize() {
        let json = serde_json::json!({
            "yaml": "apiVersion: orchestr8/v1\nkind: Workload\n"
        });
        let request: ValidateRequest = serde_json::from_value(json).unwrap();
        assert!(request.yaml.contains("orchestr8/v1"));
    }

    #[test]
    fn test_validate_request_missing_yaml_fails() {
        let json = serde_json::json!({});
        let result = serde_json::from_value::<ValidateRequest>(json);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // ValidateResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_response_valid_serialization() {
        let response = ValidateResponse {
            valid: true,
            workload_name: Some("my-app".to_string()),
            errors: vec![],
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["valid"], true);
        assert_eq!(value["workload_name"], "my-app");
        assert_eq!(value["errors"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_validate_response_invalid_serialization() {
        let response = ValidateResponse {
            valid: false,
            workload_name: None,
            errors: vec!["YAML parse error: unexpected token".to_string()],
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["valid"], false);
        assert!(value["workload_name"].is_null());
        let errors = value["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].as_str().unwrap().contains("YAML parse error"));
    }

    #[test]
    fn test_validate_response_multiple_errors() {
        let response = ValidateResponse {
            valid: false,
            workload_name: Some("bad-spec".to_string()),
            errors: vec![
                "Invalid apiVersion".to_string(),
                "Missing metadata.name".to_string(),
            ],
        };
        let value = serde_json::to_value(&response).unwrap();
        let errors = value["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 2);
    }

    // ---------------------------------------------------------------
    // SecretMetadataResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_secret_metadata_response_serialization() {
        let response = SecretMetadataResponse {
            name: "db-credentials".to_string(),
            namespace: "production".to_string(),
            key_count: 2,
            keys: vec!["username".to_string(), "password".to_string()],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-15T00:00:00Z".to_string(),
            needs_rotation: false,
            rotation_policy: None,
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["name"], "db-credentials");
        assert_eq!(value["namespace"], "production");
        assert_eq!(value["key_count"], 2);
        assert_eq!(value["keys"].as_array().unwrap().len(), 2);
        assert_eq!(value["needs_rotation"], false);
        assert!(value["rotation_policy"].is_null());
    }

    #[test]
    fn test_secret_metadata_response_with_rotation_policy() {
        let response = SecretMetadataResponse {
            name: "api-key".to_string(),
            namespace: "default".to_string(),
            key_count: 1,
            keys: vec!["token".to_string()],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            needs_rotation: true,
            rotation_policy: Some(SecretRotationInfo {
                interval_days: 30,
                max_age_days: 90,
                notify_before_days: 7,
            }),
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["needs_rotation"], true);
        assert_eq!(value["rotation_policy"]["interval_days"], 30);
        assert_eq!(value["rotation_policy"]["max_age_days"], 90);
        assert_eq!(value["rotation_policy"]["notify_before_days"], 7);
    }

    // ---------------------------------------------------------------
    // SecretRotationInfo serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_secret_rotation_info_serialization() {
        let info = SecretRotationInfo {
            interval_days: 60,
            max_age_days: 180,
            notify_before_days: 14,
        };
        let value = serde_json::to_value(&info).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 3);
        assert_eq!(value["interval_days"], 60);
        assert_eq!(value["max_age_days"], 180);
        assert_eq!(value["notify_before_days"], 14);
    }

    // ---------------------------------------------------------------
    // CreateWorkloadRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_create_workload_request_deserialize_with_runtime() {
        let json = serde_json::json!({
            "spec": {
                "apiVersion": "orchestr8/v1",
                "kind": "Workload",
                "metadata": {
                    "name": "test-app",
                    "owner": "dev",
                    "project": "demo"
                },
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile",
                    "registry": "ghcr.io/test"
                },
                "requirements": {
                    "cpu": "1",
                    "memory": "512Mi",
                    "storage": "1Gi"
                },
                "runtime": {
                    "preferred": "auto",
                    "allow": ["container"]
                }
            },
            "runtime": "podman"
        });

        let request: CreateWorkloadRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.runtime, Some("podman".to_string()));
        assert_eq!(request.spec.metadata.name, "test-app");
    }

    #[test]
    fn test_create_workload_request_deserialize_without_runtime() {
        let json = serde_json::json!({
            "spec": {
                "apiVersion": "orchestr8/v1",
                "kind": "Workload",
                "metadata": {
                    "name": "auto-app",
                    "owner": "dev",
                    "project": "demo"
                },
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile",
                    "registry": "ghcr.io/test"
                },
                "requirements": {
                    "cpu": "2",
                    "memory": "4Gi",
                    "storage": "10Gi"
                },
                "runtime": {
                    "preferred": "auto",
                    "allow": ["container", "kube"]
                }
            }
        });

        let request: CreateWorkloadRequest = serde_json::from_value(json).unwrap();
        assert!(request.runtime.is_none());
        assert_eq!(request.spec.metadata.name, "auto-app");
    }

    // ---------------------------------------------------------------
    // MigrateWorkloadRequest deserialization and default_strategy
    // ---------------------------------------------------------------

    #[test]
    fn test_default_strategy_value() {
        assert_eq!(default_strategy(), "blue-green");
    }

    #[test]
    fn test_migrate_workload_request_deserialize_with_strategy() {
        let json = serde_json::json!({
            "target_runtime": "kubernetes",
            "strategy": "rolling"
        });

        let request: MigrateWorkloadRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.target_runtime, "kubernetes");
        assert_eq!(request.strategy, "rolling");
    }

    #[test]
    fn test_migrate_workload_request_deserialize_default_strategy() {
        let json = serde_json::json!({
            "target_runtime": "podman"
        });

        let request: MigrateWorkloadRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.target_runtime, "podman");
        assert_eq!(request.strategy, "blue-green");
    }

    #[test]
    fn test_migrate_workload_request_all_strategy_values() {
        for strategy in &["immediate", "blue-green", "rolling"] {
            let json = serde_json::json!({
                "target_runtime": "kubernetes",
                "strategy": strategy
            });
            let request: MigrateWorkloadRequest = serde_json::from_value(json).unwrap();
            assert_eq!(request.strategy, *strategy);
        }
    }

    #[test]
    fn test_migrate_workload_request_missing_target_runtime_fails() {
        let json = serde_json::json!({
            "strategy": "rolling"
        });

        let result = serde_json::from_value::<MigrateWorkloadRequest>(json);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // CreateBackupRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_create_backup_request_all_fields() {
        let json = serde_json::json!({
            "name": "my-backup",
            "description": "Pre-migration backup"
        });

        let request: CreateBackupRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.name, Some("my-backup".to_string()));
        assert_eq!(request.description, Some("Pre-migration backup".to_string()));
    }

    #[test]
    fn test_create_backup_request_empty_object() {
        let json = serde_json::json!({});

        let request: CreateBackupRequest = serde_json::from_value(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.description.is_none());
    }

    #[test]
    fn test_create_backup_request_name_only() {
        let json = serde_json::json!({
            "name": "quick-backup"
        });

        let request: CreateBackupRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.name, Some("quick-backup".to_string()));
        assert!(request.description.is_none());
    }

    // ---------------------------------------------------------------
    // PolicyCheckRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_policy_check_request_with_policy_set() {
        let json = serde_json::json!({
            "spec": {
                "apiVersion": "orchestr8/v1",
                "kind": "Workload",
                "metadata": {
                    "name": "policy-test",
                    "owner": "dev",
                    "project": "demo"
                },
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile",
                    "registry": "ghcr.io/test"
                },
                "requirements": {
                    "cpu": "1",
                    "memory": "1Gi",
                    "storage": "5Gi"
                },
                "runtime": {
                    "preferred": "auto",
                    "allow": ["container"]
                }
            },
            "policy_set": "development"
        });

        let request: PolicyCheckRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.policy_set, Some("development".to_string()));
        assert_eq!(request.spec.metadata.name, "policy-test");
    }

    #[test]
    fn test_policy_check_request_without_policy_set() {
        let json = serde_json::json!({
            "spec": {
                "apiVersion": "orchestr8/v1",
                "kind": "Workload",
                "metadata": {
                    "name": "policy-test",
                    "owner": "dev",
                    "project": "demo"
                },
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile",
                    "registry": "ghcr.io/test"
                },
                "requirements": {
                    "cpu": "1",
                    "memory": "1Gi",
                    "storage": "5Gi"
                },
                "runtime": {
                    "preferred": "auto",
                    "allow": ["container"]
                }
            }
        });

        let request: PolicyCheckRequest = serde_json::from_value(json).unwrap();
        assert!(request.policy_set.is_none());
    }

    // ---------------------------------------------------------------
    // AddDependencyRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_add_dependency_request_deserialize() {
        let json = serde_json::json!({
            "workload": "frontend",
            "dependency": "backend-api"
        });

        let request: AddDependencyRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.workload, "frontend");
        assert_eq!(request.dependency, "backend-api");
    }

    #[test]
    fn test_add_dependency_request_missing_workload_fails() {
        let json = serde_json::json!({
            "dependency": "backend"
        });
        assert!(serde_json::from_value::<AddDependencyRequest>(json).is_err());
    }

    #[test]
    fn test_add_dependency_request_missing_dependency_fails() {
        let json = serde_json::json!({
            "workload": "frontend"
        });
        assert!(serde_json::from_value::<AddDependencyRequest>(json).is_err());
    }

    // ---------------------------------------------------------------
    // TemplateRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_template_request_all_fields() {
        let json = serde_json::json!({
            "workload_name": "my-web-app",
            "owner": "platform-team",
            "project": "main-site",
            "registry": "docker.io/myorg",
            "cpu": "4",
            "memory": "8Gi",
            "port": 8080,
            "replicas": 3
        });

        let request: TemplateRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.workload_name, Some("my-web-app".to_string()));
        assert_eq!(request.owner, Some("platform-team".to_string()));
        assert_eq!(request.project, Some("main-site".to_string()));
        assert_eq!(request.registry, Some("docker.io/myorg".to_string()));
        assert_eq!(request.cpu, Some("4".to_string()));
        assert_eq!(request.memory, Some("8Gi".to_string()));
        assert_eq!(request.port, Some(8080));
        assert_eq!(request.replicas, Some(3));
    }

    #[test]
    fn test_template_request_empty_object() {
        let json = serde_json::json!({});

        let request: TemplateRequest = serde_json::from_value(json).unwrap();
        assert!(request.workload_name.is_none());
        assert!(request.owner.is_none());
        assert!(request.project.is_none());
        assert!(request.registry.is_none());
        assert!(request.cpu.is_none());
        assert!(request.memory.is_none());
        assert!(request.port.is_none());
        assert!(request.replicas.is_none());
    }

    #[test]
    fn test_template_request_partial_fields() {
        let json = serde_json::json!({
            "workload_name": "svc",
            "port": 3000
        });

        let request: TemplateRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.workload_name, Some("svc".to_string()));
        assert_eq!(request.port, Some(3000));
        assert!(request.owner.is_none());
        assert!(request.replicas.is_none());
    }

    #[test]
    fn test_template_request_invalid_port_type_fails() {
        let json = serde_json::json!({
            "port": "not-a-number"
        });
        let result = serde_json::from_value::<TemplateRequest>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_template_request_invalid_replicas_type_fails() {
        let json = serde_json::json!({
            "replicas": -1
        });
        let result = serde_json::from_value::<TemplateRequest>(json);
        assert!(result.is_err());
    }

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

    // ---------------------------------------------------------------
    // Full round-trip: ApiResponse wrapping HealthResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_health_response_json() {
        let health = HealthResponse {
            status: "ok".to_string(),
            version: "0.3.0".to_string(),
        };
        let api_resp = ApiResponse::success(health);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert!(json["error"].is_null());
        assert_eq!(json["data"]["status"], "ok");
        assert_eq!(json["data"]["version"], "0.3.0");
    }

    // ---------------------------------------------------------------
    // Full round-trip: ApiResponse wrapping WorkloadResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_workload_response_json() {
        let workload = WorkloadResponse {
            name: "api-svc".to_string(),
            runtime: "KubeVirt".to_string(),
            image: "registry.io/api-svc:v1".to_string(),
            status: "deployed (kubevirt)".to_string(),
            created_at: "2026-03-10T08:00:00Z".to_string(),
        };
        let api_resp = ApiResponse::success(workload);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["name"], "api-svc");
        assert_eq!(json["data"]["runtime"], "KubeVirt");
        assert_eq!(json["data"]["image"], "registry.io/api-svc:v1");
        assert_eq!(json["data"]["status"], "deployed (kubevirt)");
        assert_eq!(json["data"]["created_at"], "2026-03-10T08:00:00Z");
    }

    // ---------------------------------------------------------------
    // Full round-trip: ApiResponse wrapping Vec<WorkloadResponse>
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_workload_list_json() {
        let workloads = vec![
            WorkloadResponse {
                name: "app-a".to_string(),
                runtime: "Podman".to_string(),
                image: "app-a:latest".to_string(),
                status: "running".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
            WorkloadResponse {
                name: "app-b".to_string(),
                runtime: "Kubernetes".to_string(),
                image: "app-b:v2".to_string(),
                status: "deployed".to_string(),
                created_at: "2026-02-01T00:00:00Z".to_string(),
            },
        ];
        let api_resp = ApiResponse::success(workloads);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0]["name"], "app-a");
        assert_eq!(data[1]["name"], "app-b");
    }

    // ---------------------------------------------------------------
    // ApiResponse error wrapping for various types
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_error_typed_workload_response() {
        let response = ApiResponse::<WorkloadResponse>::error("not found".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "not found");
    }

    #[test]
    fn test_api_response_error_typed_vec() {
        let response = ApiResponse::<Vec<String>>::error("internal error".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "internal error");
    }

    #[test]
    fn test_api_response_error_typed_build_response() {
        let response = ApiResponse::<BuildResponse>::error("build failed".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "build failed");
    }

    // ---------------------------------------------------------------
    // Edge cases for deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_migrate_workload_request_extra_fields_ignored() {
        let json = serde_json::json!({
            "target_runtime": "metal3",
            "strategy": "immediate",
            "unknown_field": "should be ignored"
        });
        let result = serde_json::from_value::<MigrateWorkloadRequest>(json);
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.target_runtime, "metal3");
        assert_eq!(request.strategy, "immediate");
    }

    #[test]
    fn test_create_backup_request_null_fields() {
        let json = serde_json::json!({
            "name": null,
            "description": null
        });
        let request: CreateBackupRequest = serde_json::from_value(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.description.is_none());
    }

    // ---------------------------------------------------------------
    // Full ApiResponse wrapping BuildResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_build_response_json() {
        let build = BuildResponse {
            image_name: "my-svc".to_string(),
            image_tag: "abc123".to_string(),
            full_name: "my-svc:abc123".to_string(),
            runtime: "podman".to_string(),
        };
        let api_resp = ApiResponse::success(build);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["image_name"], "my-svc");
        assert_eq!(json["data"]["image_tag"], "abc123");
        assert_eq!(json["data"]["full_name"], "my-svc:abc123");
        assert_eq!(json["data"]["runtime"], "podman");
    }

    // ---------------------------------------------------------------
    // Full ApiResponse wrapping ValidateResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_validate_response_json() {
        let validate = ValidateResponse {
            valid: true,
            workload_name: Some("good-app".to_string()),
            errors: vec![],
        };
        let api_resp = ApiResponse::success(validate);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["valid"], true);
        assert_eq!(json["data"]["workload_name"], "good-app");
        assert_eq!(json["data"]["errors"].as_array().unwrap().len(), 0);
    }
}
