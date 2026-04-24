//! REST API Server for Aether
//!
//! Provides HTTP endpoints for workload management.
//! Supports optional API key authentication via AETHER_API_KEY environment variable.
//! Supports optional HTTPS via --tls-cert and --tls-key flags.

mod types;
mod handlers;

pub use types::ApiConfig;

use types::AppState;
use handlers::*;
use crate::state::StateStore;
use axum::{
    extract::DefaultBodyLimit,
    http::{Request, StatusCode, Method},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, Any};

/// API key authentication middleware with RBAC support.
///
/// Authentication is checked in this order:
/// 1. Public endpoints (/health, /, /api/events/stream, /assets/*) bypass auth entirely.
/// 2. If no AETHER_API_KEY is set AND the RBAC store is empty, allow all requests (local dev).
/// 3. Extract Bearer token from Authorization header.
/// 4. Try RBAC store first — if token matches a registered key, enforce role-based permissions.
/// 5. Fall back to AETHER_API_KEY env var for backward compatibility (grants Admin role).
async fn auth_middleware(
    axum::extract::State(app_state): axum::extract::State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Public endpoints: health check, dashboard, SSE, and static assets
    let path = req.uri().path();
    if path == "/health" || path == "/" || path == "/api/events/stream" || path.starts_with("/assets") {
        return Ok(next.run(req).await);
    }

    let legacy_key = std::env::var("AETHER_API_KEY").ok().filter(|k| !k.is_empty());

    // Extract Bearer token before acquiring lock
    let token = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let method = req.method().to_string();

    // Acquire RBAC lock briefly — do all lookups, then drop immediately.
    // verify_key() performs SHA-256 hashing, so minimize lock hold time.
    let rbac_result = {
        let rbac_store = app_state.rbac.read().await;
        let has_rbac_keys = !rbac_store.list_keys().is_empty();
        let verified = token.as_deref()
            .and_then(|t| rbac_store.verify_key(t))
            .map(|entry| (entry.role.clone(), crate::rbac::check_permission(&entry.role, &method, path)));
        (has_rbac_keys, verified)
    };
    // Lock dropped here

    let (has_rbac_keys, verified) = rbac_result;

    // If no auth configured at all, allow everything (local development)
    if legacy_key.is_none() && !has_rbac_keys {
        return Ok(next.run(req).await);
    }

    let Some(ref token) = token else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    // Try RBAC result first
    if let Some((_role, permitted)) = verified {
        if !permitted {
            return Err(StatusCode::FORBIDDEN);
        }
        return Ok(next.run(req).await);
    }

    // Fall back to legacy AETHER_API_KEY (grants Admin-equivalent access).
    // Use constant-time comparison via SHA-256 to prevent timing attacks.
    if let Some(ref expected) = legacy_key {
        use sha2::{Digest, Sha256};
        let token_hash = Sha256::digest(token.as_bytes());
        let expected_hash = Sha256::digest(expected.as_bytes());
        if token_hash == expected_hash {
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Create a shutdown signal that listens for Ctrl+C and SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n🛑 Received shutdown signal, draining connections...");
}

/// Run a single background health check cycle.
async fn run_background_health_check(state: &Arc<RwLock<StateStore>>) -> anyhow::Result<()> {
    let orch_path = crate::orchestrator::Orchestrator::default_path();
    let mut orch = crate::orchestrator::Orchestrator::load(&orch_path)?;

    if orch.list_workloads().is_empty() {
        return Ok(());
    }

    let state_store = state.read().await;

    // Build health statuses by querying runtimes
    let mut statuses = std::collections::HashMap::new();
    for mw in orch.list_workloads() {
        if let Some(ws) = state_store.get(&mw.name) {
            match crate::runtime::create_runtime(&ws.runtime).await {
                Ok(rt) => {
                    match rt.status(&ws.instance).await {
                        Ok(status) => {
                            let hs = match status.state {
                                crate::runtime::InstanceState::Running if status.ready => {
                                    crate::orchestrator::HealthStatus::Healthy
                                }
                                crate::runtime::InstanceState::Running => {
                                    crate::orchestrator::HealthStatus::Degraded
                                }
                                crate::runtime::InstanceState::Failed => {
                                    crate::orchestrator::HealthStatus::Unhealthy
                                }
                                _ => crate::orchestrator::HealthStatus::Unknown,
                            };
                            statuses.insert(mw.name.clone(), hs);
                        }
                        Err(_) => {
                            statuses.insert(mw.name.clone(), crate::orchestrator::HealthStatus::Unknown);
                        }
                    }
                }
                Err(_) => {
                    statuses.insert(mw.name.clone(), crate::orchestrator::HealthStatus::Unknown);
                }
            }
        }
    }
    drop(state_store);

    let actions = orch.run_health_checks_from_statuses(&statuses);
    orch.save(&orch_path)?;

    if !actions.is_empty() {
        tracing::info!("Background health check: {} action(s)", actions.len());
    }

    Ok(())
}

/// Start the API server
pub async fn start_server(config: ApiConfig) -> anyhow::Result<()> {
    // Load state
    let state_store = StateStore::load(&config.state_path)?;
    let (event_tx, _) = broadcast::channel::<String>(256);

    // Load RBAC store (create empty if not found)
    let rbac_store = crate::rbac::RbacStore::load(&crate::rbac::RbacStore::default_path())
        .unwrap_or_default();

    let app_state = AppState {
        state: Arc::new(RwLock::new(state_store)),
        event_tx,
        rbac: Arc::new(RwLock::new(rbac_store)),
    };

    // Spawn background health check loop
    {
        let health_state = app_state.state.clone();
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(30);
            loop {
                tokio::time::sleep(interval).await;
                if let Err(e) = run_background_health_check(&health_state).await {
                    tracing::debug!("Background health check cycle: {}", e);
                }
            }
        });
    }

    let tls_enabled = config.tls_cert.is_some() && config.tls_key.is_some();
    let scheme = if tls_enabled { "https" } else { "http" };

    // CORS: allow any origin since the dashboard is served from this same server.
    // When accessed via NodePort or load balancer, the external origin differs
    // from the bind address, so restricting to self would block the dashboard.
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any)
        .allow_origin(Any);

    // Build router — dashboard assets are embedded in the binary via include_str!
    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/assets/index-DiVd-Lmd.css", get(serve_dashboard_css))
        .route("/assets/index-qv9Tu_QH.js", get(serve_dashboard_js))
        .route("/health", get(health_check))
        .route("/api/events/stream", get(sse_events))
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
        .route("/api/cluster/summary", get(api_cluster_summary))
        .route("/api/cluster/namespaces", get(api_cluster_namespaces))
        .route("/api/cluster/browse", get(api_cluster_browse))
        .route("/api/cluster/apply", post(api_cluster_apply))
        .route("/api/cluster/logs", get(api_cluster_logs))
        .route("/api/cluster/resource", get(api_cluster_resource))
        .route("/api/cluster/action", post(api_cluster_action))
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
        .route("/api/audit/verify", get(api_audit_verify))
        .route("/api/rbac/keys", get(rbac_list_keys))
        .route("/api/rbac/keys", post(rbac_create_key))
        .route("/api/rbac/keys/revoke", post(rbac_revoke_key))
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024)) // 2 MB max request body
                .layer(tower::limit::ConcurrencyLimitLayer::new(200))
                .layer(cors)
                .layer(middleware::from_fn_with_state(app_state.clone(), auth_middleware))
        )
        .with_state(app_state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port)
        .parse::<SocketAddr>()?;

    let has_api_key = std::env::var("AETHER_API_KEY")
        .map(|k| !k.is_empty())
        .unwrap_or(false);

    println!("🌐 Starting API server on {}://{}", scheme, addr);
    println!("📊 Dashboard: {}://{}", scheme, addr);
    println!("📋 API Health: {}://{}/health", scheme, addr);
    if tls_enabled {
        println!("🔒 TLS enabled");
    }
    if has_api_key {
        println!("🔐 API authentication enabled (AETHER_API_KEY)");
    } else {
        println!("⚠️  No AETHER_API_KEY set — API is unauthenticated. Set AETHER_API_KEY for production use.");
    }

    if tls_enabled {
        // HTTPS with TLS — validate files exist before attempting to load
        let cert_path = config.tls_cert.as_ref().unwrap();
        let key_path = config.tls_key.as_ref().unwrap();
        if !cert_path.exists() {
            anyhow::bail!("TLS certificate file not found: {}", cert_path.display());
        }
        if !key_path.exists() {
            anyhow::bail!("TLS key file not found: {}", key_path.display());
        }

        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            cert_path,
            key_path,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load TLS certificate/key: {e}"))?;

        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();

        // Spawn shutdown signal handler
        tokio::spawn(async move {
            shutdown_signal().await;
            shutdown_handle.graceful_shutdown(Some(std::time::Duration::from_secs(30)));
        });

        axum_server::bind_rustls(addr, tls_config)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
    } else {
        // Plain HTTP
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    }

    println!("🛑 Server stopped");
    Ok(())
}
