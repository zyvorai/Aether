//! REST API Server for Aether
//!
//! Provides HTTP endpoints for workload management

mod types;
mod handlers;

pub use types::ApiConfig;

use types::AppState;
use handlers::*;
use crate::state::StateStore;
use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

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
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024)) // 2 MB max request body
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
