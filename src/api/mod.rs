//! REST API Server for Aether
//!
//! Provides HTTP endpoints for workload management.
//! Supports optional API key authentication via AETHER_API_KEY environment variable.

mod types;
mod handlers;

pub use types::ApiConfig;

use types::AppState;
use handlers::*;
use crate::state::StateStore;
use axum::{
    extract::DefaultBodyLimit,
    http::{Request, StatusCode, HeaderValue, Method},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, Any};

/// API key authentication middleware.
/// If AETHER_API_KEY is set, all /api/* requests must include a matching
/// `Authorization: Bearer <key>` header. The /health endpoint and dashboard
/// (/) are always public.
async fn auth_middleware(req: Request<axum::body::Body>, next: Next) -> Result<Response, StatusCode> {
    let api_key = std::env::var("AETHER_API_KEY").ok();

    // If no API key is configured, allow all requests (local development)
    let Some(expected_key) = api_key else {
        return Ok(next.run(req).await);
    };

    if expected_key.is_empty() {
        return Ok(next.run(req).await);
    }

    // Public endpoints: health check and dashboard
    let path = req.uri().path();
    if path == "/health" || path == "/" {
        return Ok(next.run(req).await);
    }

    // Check Authorization header
    if let Some(auth_header) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if token == expected_key {
                    return Ok(next.run(req).await);
                }
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Start the API server
pub async fn start_server(config: ApiConfig) -> anyhow::Result<()> {
    // Load state
    let state_store = StateStore::load(&config.state_path)?;
    let app_state = AppState {
        state: Arc::new(RwLock::new(state_store)),
    };

    // CORS configuration: restrict to same-origin by default
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any)
        .allow_origin({
            let origin_str = format!("http://{}:{}", config.host, config.port);
            origin_str.parse::<HeaderValue>().unwrap_or_else(|e| {
                tracing::warn!("Failed to parse CORS origin '{}': {}, using default", origin_str, e);
                HeaderValue::from_static("http://127.0.0.1:5090")
            })
        });

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
        .route("/api/ai/migration-advice/:name/:target", get(ai_migration_advice))
        .route("/api/ai/scaling-advice", get(ai_scaling_advice))
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
        .route("/api/plugins", get(api_plugins_list))
        .route("/api/plugins/discover", post(api_plugins_discover))
        .route("/api/health/:workload", get(api_health_summary))
        .route("/api/compose/validate", post(api_compose_validate))
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024)) // 2 MB max request body
                .layer(cors)
                .layer(middleware::from_fn(auth_middleware))
        )
        .with_state(app_state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port)
        .parse::<SocketAddr>()?;

    let has_api_key = std::env::var("AETHER_API_KEY")
        .map(|k| !k.is_empty())
        .unwrap_or(false);

    println!("🌐 Starting API server on http://{}", addr);
    println!("📊 Dashboard: http://{}", addr);
    println!("📋 API Health: http://{}/health", addr);
    if has_api_key {
        println!("🔐 API authentication enabled (AETHER_API_KEY)");
    } else {
        println!("⚠️  No AETHER_API_KEY set — API is unauthenticated. Set AETHER_API_KEY for production use.");
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
