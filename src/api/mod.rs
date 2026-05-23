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
    body::Body,
    extract::DefaultBodyLimit,
    http::{header::HeaderName, HeaderMap, HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, Any};

/// Monotonic id generator for `x-request-id` when the client does not supply one.
static HTTP_REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);

fn mutation_confirm_env_enabled() -> bool {
    std::env::var("AETHER_REQUIRE_MUTATION_CONFIRM")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// DELETE routes that remove durable user data or workloads.
fn path_requires_mutation_confirm(method: &Method, path: &str) -> bool {
    if *method != Method::DELETE {
        return false;
    }
    let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
    matches!(
        segs.as_slice(),
        ["api", "workloads", _] | ["api", "secrets", _] | ["api", "backups", _]
    )
}

fn mutation_confirm_header_ok(headers: &HeaderMap) -> bool {
    headers
        .get("x-aether-confirm")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            let t = s.trim();
            t == "1"
                || t.eq_ignore_ascii_case("true")
                || t.eq_ignore_ascii_case("yes")
                || t.eq_ignore_ascii_case("delete")
        })
        .unwrap_or(false)
}

fn ensure_mutation_confirm(method: &Method, path: &str, headers: &HeaderMap) -> Result<(), StatusCode> {
    if !mutation_confirm_env_enabled() {
        return Ok(());
    }
    if !path.starts_with("/api/") {
        return Ok(());
    }
    if !path_requires_mutation_confirm(method, path) {
        return Ok(());
    }
    if mutation_confirm_header_ok(headers) {
        Ok(())
    } else {
        Err(StatusCode::PRECONDITION_FAILED)
    }
}

/// Baseline security headers for dashboard + API responses (TLS termination may add HSTS upstream).
async fn security_headers_middleware(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    let _ = h.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    let _ = h.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("SAMEORIGIN"),
    );
    let _ = h.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    let _ = h.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()"),
    );
    res
}

async fn observability_middleware(req: Request<Body>, next: Next) -> Response {
    let hdr = HeaderName::from_static("x-request-id");
    let rid = req
        .headers()
        .get(&hdr)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty() && s.len() <= 128)
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| {
            format!(
                "aether-{}",
                HTTP_REQUEST_SEQ.fetch_add(1, Ordering::Relaxed)
            )
        });

    let method = req.method().as_str().to_string();
    let mut res = next.run(req).await;
    let status = res.status().as_u16().to_string();
    crate::metrics::record_api_http(&method, &status);
    if let Ok(val) = HeaderValue::try_from(rid.as_str()) {
        res.headers_mut().insert(hdr, val);
    }
    res
}

/// API key authentication middleware with RBAC support.
///
/// Authentication is checked in this order:
/// 1. Public endpoints (/health, `/`, OIDC bootstrap, `/api/auth/providers`, `/api/system/ready`, /assets/*) bypass auth.
///    `/api/events/stream` requires the same auth as other API routes (bearer, query token, or OIDC cookie).
/// 2. If no `AETHER_API_KEY`, no RBAC keys, and OIDC is not configured — allow all requests (local dev).
/// 3. Bearer token: RBAC store, then legacy `AETHER_API_KEY`.
/// 4. When OIDC is configured: signed `aether_session` cookie (same permission model as RBAC roles).
async fn auth_middleware(
    axum::extract::State(app_state): axum::extract::State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Public: static assets, OIDC bootstrap, auth provider advertisement, readiness, dashboard HTML shell.
    let path = req.uri().path();
    let public_unauthenticated = path.starts_with("/assets")
        || path == "/api/system/ready"
        || path == "/api/auth/providers"
        || path == "/api/auth/oidc/login"
        || path == "/api/auth/oidc/callback"
        || path == "/api/auth/oidc/logout"
        || path == "/api/auth/saml/login"
        || path == "/api/auth/saml/acs"
        || path == "/api/auth/saml/logout"
        || path.starts_with("/api/mock-idp/")
        || (*req.method() == Method::GET && !path.starts_with("/api"));
    if public_unauthenticated {
        return Ok(next.run(req).await);
    }

    let legacy_key = std::env::var("AETHER_API_KEY").ok().filter(|k| !k.is_empty());
    let oidc_enabled = app_state.oidc.is_some();
    let saml_enabled = app_state.saml.is_some();
    let session_auth_enabled = oidc_enabled || saml_enabled;

    // Extract Bearer token before acquiring lock
    let header_token = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    let query_token = req.uri().query().and_then(|query| {
        query
            .split('&')
            .filter_map(|part| part.split_once('='))
            .find(|(key, _)| *key == "token")
            .map(|(_, value)| value.replace("%20", " "))
    });
    let token = header_token.or(query_token);

    let http_method = req.method().to_string();

    // Acquire RBAC lock briefly — do all lookups, then drop immediately.
    let rbac_result = {
        let rbac_store = app_state.rbac.read().await;
        let has_rbac_keys = !rbac_store.list_keys().is_empty();
        let verified = token.as_deref()
            .and_then(|t| rbac_store.verify_key(t))
            .map(|entry| (entry.role.clone(), crate::rbac::check_permission(&entry.role, &http_method, path)));
        (has_rbac_keys, verified)
    };

    let (has_rbac_keys, verified) = rbac_result;

    // If no auth configured at all, allow everything (local development)
    if legacy_key.is_none() && !has_rbac_keys && !session_auth_enabled {
        ensure_mutation_confirm(req.method(), path, req.headers())?;
        return Ok(next.run(req).await);
    }

    if let Some(ref token) = token {
        if let Some((_role, permitted)) = verified {
            if !permitted {
                return Err(StatusCode::FORBIDDEN);
            }
            ensure_mutation_confirm(req.method(), path, req.headers())?;
            return Ok(next.run(req).await);
        }

        if let Some(ref expected) = legacy_key {
            use sha2::{Digest, Sha256};
            let token_hash = Sha256::digest(token.as_bytes());
            let expected_hash = Sha256::digest(expected.as_bytes());
            if token_hash == expected_hash {
                ensure_mutation_confirm(req.method(), path, req.headers())?;
                return Ok(next.run(req).await);
            }
        }
        return Err(StatusCode::UNAUTHORIZED);
    }

    if session_auth_enabled {
        if let Some(oidc) = app_state.oidc.as_ref() {
            if let Some((role, _username)) = oidc.verify_session_cookie(req.headers()) {
                if !crate::rbac::check_permission(&role, &http_method, path) {
                    return Err(StatusCode::FORBIDDEN);
                }
                ensure_mutation_confirm(req.method(), path, req.headers())?;
                return Ok(next.run(req).await);
            }
        }
        if let Some(saml) = app_state.saml.as_ref() {
            if let Some((role, _username)) = saml.verify_session_cookie(req.headers()) {
                if !crate::rbac::check_permission(&role, &http_method, path) {
                    return Err(StatusCode::FORBIDDEN);
                }
                ensure_mutation_confirm(req.method(), path, req.headers())?;
                return Ok(next.run(req).await);
            }
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

    run_alert_evaluation(state).await;

    Ok(())
}

/// Evaluate alert rules against current health, cost, and policy metrics.
async fn run_alert_evaluation(state: &Arc<RwLock<StateStore>>) {
    let events_path = crate::events::EventBus::default_path();
    let Ok(mut bus) = crate::events::EventBus::load(&events_path) else {
        return;
    };
    let policy = crate::config::Config::load().policy;
    let workloads: Vec<_> = {
        let store = state.read().await;
        store.list().into_iter().cloned().collect()
    };
    let metrics = crate::events::SystemMetrics::collect(&workloads, &policy);
    let fired = bus.evaluate_rules(&metrics);
    if !fired.is_empty() {
        tracing::info!("Alert evaluation: {} rule(s) triggered", fired.len());
        let _ = bus.save(&events_path);
    }
}

/// Start the API server
pub async fn start_server(config: ApiConfig) -> anyhow::Result<()> {
    let state_path = config.state_path.clone();

    let workload_state_pg: Option<std::sync::Arc<crate::state_postgres::WorkloadStatePool>> =
        match std::env::var("AETHER_STATE_DATABASE_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            Some(url) => {
                let pool = crate::state_postgres::WorkloadStatePool::connect(&url).await?;
                pool.bootstrap_from_file_if_empty(&state_path).await?;
                tracing::info!("workload state: PostgreSQL backend enabled (AETHER_STATE_DATABASE_URL)");
                Some(std::sync::Arc::new(pool))
            }
            None => None,
        };

    let state_store = if let Some(ref pg) = workload_state_pg {
        pg.load().await?
    } else {
        StateStore::load(&state_path)?
    };

    let (event_tx, _) = broadcast::channel::<String>(256);

    // Load RBAC store (create empty if not found)
    let rbac_store = crate::rbac::RbacStore::load(&crate::rbac::RbacStore::default_path())
        .unwrap_or_default();

    let tls_enabled = config.tls_cert.is_some() && config.tls_key.is_some();
    let scheme = if tls_enabled { "https" } else { "http" };
    let base_url = format!("{}://{}:{}", scheme, config.host, config.port);
    if crate::mock_idp::enabled() {
        crate::mock_idp::apply_env_defaults(&base_url);
    }

    let redis_url = std::env::var("AETHER_REDIS_URL").ok();
    let shared_cache = crate::ha::SharedCache::connect(redis_url.as_deref()).await?;
    let oidc = crate::oidc::OidcRuntime::new(shared_cache.clone())?;
    let saml = crate::saml::SamlRuntime::new(shared_cache.clone())?;

    let app_state = AppState {
        state: Arc::new(RwLock::new(state_store)),
        event_tx,
        rbac: Arc::new(RwLock::new(rbac_store)),
        port_forwards: Arc::new(Mutex::new(std::collections::HashMap::new())),
        shared_cache,
        oidc,
        saml,
        tls_active: tls_enabled,
        state_path,
        workload_state_pg: workload_state_pg.clone(),
    };

    if let Some(pg) = workload_state_pg {
        crate::state_postgres::spawn_workload_state_poller(pg, app_state.state.clone());
    }

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

    let scheme = if tls_enabled { "https" } else { "http" };

    // CORS: allow any origin since the dashboard is served from this same server.
    // When accessed via NodePort or load balancer, the external origin differs
    // from the bind address, so restricting to self would block the dashboard.
    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::PATCH,
        ])
        .allow_headers(Any)
        .allow_origin(Any);

    // Build router — dashboard assets are embedded in the binary via include_str!
    let mut app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/assets/aether-dashboard.css", get(serve_dashboard_css))
        .route("/assets/aether-dashboard.js", get(serve_dashboard_js))
        .route("/health", get(health_check))
        .route("/api/events/stream", get(sse_events))
        .route("/api/auth/me", get(api_auth_me))
        .route("/api/auth/providers", get(api_auth_providers))
        .route("/api/auth/oidc/login", get(api_oidc_login))
        .route("/api/auth/oidc/callback", get(api_oidc_callback))
        .route("/api/auth/oidc/logout", get(api_oidc_logout))
        .route("/api/auth/saml/login", get(api_saml_login))
        .route("/api/auth/saml/acs", post(api_saml_acs))
        .route("/api/auth/saml/logout", get(api_saml_logout))
        .route("/api/system/ready", get(api_system_ready))
        .route("/api/workloads", get(list_workloads))
        .route("/api/workloads", post(create_workload))
        .route(
            "/api/workloads/:name",
            get(get_workload)
                .put(update_workload)
                .patch(update_workload)
                .delete(delete_workload),
        )
        .route("/api/workloads/:name/logs", get(get_logs))
        .route("/api/workloads/:name/start", post(start_workload))
        .route("/api/workloads/:name/stop", post(stop_workload))
        .route("/api/workloads/:name/restart", post(restart_workload))
        .route("/api/workloads/:name/migrate", post(migrate_workload))
        .route("/api/workloads/:name/build", post(build_workload))
        .route("/api/validate", post(validate_workload))
        .route("/api/secrets", get(api_secrets_list).post(create_secret))
        .route("/api/secrets/:name", get(get_secret))
        .route("/api/secrets/:name", delete(delete_secret))
        .route("/api/metrics", get(get_metrics))
        .route("/api/cost", post(estimate_cost))
        .route("/api/cost/pricing", get(api_cost_pricing))
        .route("/api/cost/chargeback", get(api_cost_chargeback))
        .route("/api/backups", get(list_backups))
        .route("/api/backups", post(create_backup))
        .route("/api/backups/restore", post(restore_backup))
        .route(
            "/api/backups/:name",
            get(get_backup).delete(delete_backup_api),
        )
        .route("/api/ai/recommend", post(ai_recommend))
        .route("/api/ai/profile/:name", get(ai_profile))
        .route("/api/ai/analyze/:name", get(ai_analyze_logs))
        .route("/api/ai/migration-advice/:name/:target", get(ai_migration_advice))
        .route("/api/ai/scaling-advice", get(ai_scaling_advice))
        .route("/api/ai/intent-optimize", post(ai_intent_optimize))
        .route("/api/ai/right-size", post(ai_right_size))
        .route("/api/ai/tradeoff", post(ai_tradeoff))
        .route("/api/drift/:name", get(api_drift_check))
        .route("/api/drift/:name/reconcile", post(api_drift_reconcile))
        .route("/api/alerts/status", get(api_alerts_status))
        .route("/api/policy/check", post(api_policy_check))
        .route("/api/policy/opa", post(api_policy_opa))
        .route(
            "/api/dependencies",
            get(api_deps_show).post(api_deps_add).delete(api_deps_remove),
        )
        .route("/api/audit", get(api_audit_list))
        .route("/api/audit/events", post(api_audit_append))
        .route("/api/templates", get(api_template_list))
        .route("/api/templates/:name", post(api_template_generate))
        .route("/api/sla/:workload", get(api_sla_check))
        .route("/api/events", get(api_events_list))
        .route("/api/events/summary", get(api_events_summary))
        .route("/api/cluster/summary", get(api_cluster_summary))
        .route("/api/cluster/namespaces", get(api_cluster_namespaces))
        .route("/api/cluster/browse", get(api_cluster_browse))
        .route("/api/cluster/apply", post(api_cluster_apply))
        .route("/api/cluster/events", get(api_cluster_events))
        .route("/api/cluster/logs", get(api_cluster_logs))
        .route("/api/cluster/resource", get(api_cluster_resource))
        .route("/api/cluster/health", get(api_cluster_health))
        .route("/api/cluster/action", post(api_cluster_action))
        .route("/api/cluster/ws/exec", get(api_cluster_exec_ws))
        .route("/api/cluster/ws/watch", get(api_cluster_watch_ws))
        .route("/api/cluster/port-forward", post(api_cluster_port_forward_start))
        .route("/api/cluster/port-forward/stop", post(api_cluster_port_forward_stop))
        .route("/api/cluster/top", get(api_cluster_top))
        .route("/api/cluster/metrics/summary", get(api_cluster_metrics_summary))
        .route("/api/cluster/diff", post(api_cluster_diff))
        .route("/api/cluster/rollout", get(api_cluster_rollout))
        .route("/api/cluster/rollout/action", post(api_cluster_rollout_action))
        .route("/api/cluster/helm/history", get(api_cluster_helm_history))
        .route("/api/cluster/helm/action", post(api_cluster_helm_action))
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
        .route("/api/gitops/status", get(api_gitops_status))
        .route("/api/gitops/sync", post(api_gitops_sync))
        .route("/api/webhooks/test", post(api_webhook_test))
        .route("/api/webhooks/channels", post(api_webhook_channel_create))
        .route("/api/webhooks/channels/:name", delete(api_webhook_channel_delete))
        .route("/api/dashboard/version", get(api_dashboard_version))
        .route("/api/server", get(api_server_info))
        .route("/api/openapi.json", get(serve_openapi))
        .route("/api/rbac/keys", get(rbac_list_keys))
        .route("/api/rbac/keys", post(rbac_create_key))
        .route("/api/rbac/keys/revoke", post(rbac_revoke_key));

    if crate::mock_idp::enabled() {
        app = app
            .route("/api/mock-idp/saml/sso", get(crate::mock_idp::saml_sso))
            .route(
                "/api/mock-idp/oidc/.well-known/openid-configuration",
                get(crate::mock_idp::oidc_discovery),
            )
            .route("/api/mock-idp/oidc/authorize", get(crate::mock_idp::oidc_authorize))
            .route("/api/mock-idp/oidc/token", post(crate::mock_idp::oidc_token))
            .route("/api/mock-idp/oidc/jwks", get(crate::mock_idp::oidc_jwks));
    }

    let app = app
        .fallback(get(serve_dashboard_spa_fallback))
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024)) // 2 MB max request body
                .layer(tower::limit::ConcurrencyLimitLayer::new(200))
                .layer(cors)
                .layer(middleware::from_fn_with_state(app_state.clone(), auth_middleware))
                .layer(middleware::from_fn(observability_middleware))
                .layer(middleware::from_fn(security_headers_middleware))
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
    if std::env::var("AETHER_STATE_DATABASE_URL")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        println!("🗄️  Shared workload state: PostgreSQL (multi-replica API ready)");
    }
    if std::env::var("AETHER_REDIS_URL")
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        println!("🔗 OIDC session cache: Redis (AETHER_REDIS_URL)");
    }
    if std::env::var("AETHER_REQUIRE_MUTATION_CONFIRM")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        println!("🛡️  AETHER_REQUIRE_MUTATION_CONFIRM enabled — destructive DELETEs need header X-Aether-Confirm: 1");
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
