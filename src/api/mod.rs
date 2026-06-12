// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! REST API Server for Aether
//!
//! Provides HTTP endpoints for workload management.
//! Supports optional API key authentication via AETHER_API_KEY environment variable.
//! Supports optional HTTPS via --tls-cert and --tls-key flags.

mod confidential_handlers;
mod ecosystem_handlers;
mod fleet_handlers;
mod handlers;
mod hosted_handlers;
mod intelligence_handlers;
mod migration_handlers;
mod ops_handlers;
mod platform_recommendations;
mod security_handlers;
mod types;
mod zeus_handlers;

pub use types::ApiConfig;

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
use confidential_handlers::*;
use ecosystem_handlers::*;
use fleet_handlers::*;
use handlers::*;
use hosted_handlers::*;
use intelligence_handlers::*;
use migration_handlers::*;
use ops_handlers::*;
use security_handlers::*;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use types::AppState;
use zeus_handlers::*;

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

fn ensure_mutation_confirm(
    method: &Method,
    path: &str,
    headers: &HeaderMap,
) -> Result<(), StatusCode> {
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

fn resolve_tenant_id(
    headers: &axum::http::HeaderMap,
    key_tenant: Option<String>,
) -> Option<String> {
    key_tenant.or_else(|| crate::hosted::tenant::tenant_from_headers(headers))
}

async fn authorized_next(
    req: Request<Body>,
    next: Next,
    tenant_id: Option<String>,
) -> Result<Response, StatusCode> {
    if let Some(ref tid) = tenant_id {
        if crate::hosted::metering::UsageMeter::quota_exceeded(tid) {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }
    let path = req.uri().path().to_string();
    let res = next.run(req).await;
    if let Some(tid) = tenant_id {
        crate::hosted::metering::UsageMeter::record(&tid, &path);
    }
    Ok(res)
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
        || path == "/api/hosted/billing/stripe/webhook"
        || path.starts_with("/api/mock-idp/")
        || (*req.method() == Method::GET && !path.starts_with("/api"));
    if public_unauthenticated {
        return Ok(next.run(req).await);
    }

    let legacy_key = std::env::var("AETHER_API_KEY")
        .ok()
        .filter(|k| !k.is_empty());
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
        let verified = token
            .as_deref()
            .and_then(|t| rbac_store.verify_key(t))
            .map(|entry| {
                (
                    entry.role,
                    crate::rbac::check_permission(&entry.role, &http_method, path),
                )
            });
        (has_rbac_keys, verified)
    };

    let (has_rbac_keys, verified) = rbac_result;

    // If no auth configured at all, allow everything (local development)
    if legacy_key.is_none() && !has_rbac_keys && !session_auth_enabled {
        ensure_mutation_confirm(req.method(), path, req.headers())?;
        let tenant = resolve_tenant_id(req.headers(), None);
        return authorized_next(req, next, tenant).await;
    }

    if let Some(ref token) = token {
        if let Some((_role, permitted)) = verified {
            if !permitted {
                return Err(StatusCode::FORBIDDEN);
            }
            ensure_mutation_confirm(req.method(), path, req.headers())?;
            let tenant = resolve_tenant_id(req.headers(), None);
            return authorized_next(req, next, tenant).await;
        }

        if let Some(key_rec) = crate::hosted::keys::TenantKeyStore::load().verify(token) {
            let role = crate::rbac::Role::Operator;
            if !crate::rbac::check_permission(&role, &http_method, path) {
                return Err(StatusCode::FORBIDDEN);
            }
            ensure_mutation_confirm(req.method(), path, req.headers())?;
            let tenant = resolve_tenant_id(req.headers(), Some(key_rec.tenant_id.clone()));
            return authorized_next(req, next, tenant).await;
        }

        if let Some(ref expected) = legacy_key {
            use sha2::{Digest, Sha256};
            let token_hash = Sha256::digest(token.as_bytes());
            let expected_hash = Sha256::digest(expected.as_bytes());
            if token_hash == expected_hash {
                ensure_mutation_confirm(req.method(), path, req.headers())?;
                let tenant = resolve_tenant_id(req.headers(), None);
                return authorized_next(req, next, tenant).await;
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
                let tenant = resolve_tenant_id(req.headers(), None);
                return authorized_next(req, next, tenant).await;
            }
        }
        if let Some(saml) = app_state.saml.as_ref() {
            if let Some((role, _username)) = saml.verify_session_cookie(req.headers()) {
                if !crate::rbac::check_permission(&role, &http_method, path) {
                    return Err(StatusCode::FORBIDDEN);
                }
                ensure_mutation_confirm(req.method(), path, req.headers())?;
                let tenant = resolve_tenant_id(req.headers(), None);
                return authorized_next(req, next, tenant).await;
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
                Ok(rt) => match rt.status(&ws.instance).await {
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
                        statuses
                            .insert(mw.name.clone(), crate::orchestrator::HealthStatus::Unknown);
                    }
                },
                Err(_) => {
                    statuses.insert(mw.name.clone(), crate::orchestrator::HealthStatus::Unknown);
                }
            }
        }
    }
    drop(state_store);

    let actions = orch.run_health_checks_from_statuses(&statuses);
    orch.save(&orch_path)?;

    let config = crate::config::Config::load();
    let policy = crate::intelligence::policy::AutonomyPolicy::from_config_and_workload(
        config.reconciliation.auto_reconcile,
        None,
    );
    if !actions.is_empty() {
        let healer_result = crate::intelligence::healer::execute_orchestrator_actions(
            &actions,
            &policy,
            state,
            &crate::state::StateStore::default_path(),
            "api-health-loop",
        )
        .await;
        if !healer_result.executed.is_empty() {
            tracing::info!("Autonomous healing: {:?}", healer_result.executed);
        }
    }

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
                tracing::info!(
                    "workload state: PostgreSQL backend enabled (AETHER_STATE_DATABASE_URL)"
                );
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
    let rbac_store =
        crate::rbac::RbacStore::load(&crate::rbac::RbacStore::default_path()).unwrap_or_default();

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
        .route(
            "/api/workloads/:name/snapshots",
            get(api_workload_snapshots),
        )
        .route("/api/workloads/:name/rollback", post(api_workload_rollback))
        .route("/api/workloads/:name/migrate", post(migrate_workload))
        .route("/api/workloads/:name/build", post(build_workload))
        .route("/api/validate", post(validate_workload))
        .route("/api/secrets", get(api_secrets_list).post(create_secret))
        .route("/api/secrets/:name", get(get_secret))
        .route("/api/secrets/:name", delete(delete_secret))
        .route("/api/metrics", get(get_metrics))
        .route("/api/observability/summary", get(api_observability_summary))
        .route(
            "/api/observability/prometheus/query",
            get(api_observability_prometheus_query),
        )
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
        .route(
            "/api/ai/migration-advice/:name/:target",
            get(ai_migration_advice),
        )
        .route("/api/ai/scaling-advice", get(ai_scaling_advice))
        .route("/api/ai/intent-optimize", post(ai_intent_optimize))
        .route("/api/ai/right-size", post(ai_right_size))
        .route("/api/ai/tradeoff", post(ai_tradeoff))
        .route(
            "/api/ai/migration-plan/:name/:target",
            get(api_ai_migration_plan),
        )
        .route(
            "/api/command-center/briefing",
            get(api_command_center_briefing),
        )
        .route(
            "/api/command-center/next-actions",
            get(api_command_center_next_actions),
        )
        .route(
            "/api/command-center/notifications",
            get(api_command_center_notifications),
        )
        .route("/api/context/snapshot", get(api_context_snapshot))
        .route(
            "/api/intelligence/predictions",
            get(api_intelligence_predictions),
        )
        .route(
            "/api/intelligence/predictions/:name",
            get(api_intelligence_prediction_workload),
        )
        .route(
            "/api/intelligence/cost-optimize",
            get(api_intelligence_cost_optimize),
        )
        .route("/api/intelligence/threats", get(api_intelligence_threats))
        .route(
            "/api/intelligence/security/policies",
            get(api_intelligence_security_policies),
        )
        .route(
            "/api/intelligence/digital-twin/simulate",
            post(api_intelligence_digital_twin_simulate),
        )
        .route(
            "/api/intelligence/autonomy/status",
            get(api_intelligence_autonomy_status),
        )
        .route(
            "/api/intelligence/knowledge-graph",
            get(api_intelligence_knowledge_graph),
        )
        .route(
            "/api/intelligence/healer/preview",
            get(api_intelligence_healer_preview),
        )
        .route(
            "/api/intelligence/intent-pipeline",
            post(api_intelligence_intent_pipeline),
        )
        .route(
            "/api/intelligence/sre/runbook",
            get(api_intelligence_sre_runbook),
        )
        .route(
            "/api/intelligence/multicloud/posture",
            get(api_intelligence_multicloud_posture),
        )
        .route(
            "/api/intelligence/autonomous/placement",
            get(api_intelligence_autonomous_placement),
        )
        .route(
            "/api/intelligence/agents/status",
            get(api_intelligence_agents_status),
        )
        .route(
            "/api/intelligence/healer/execute",
            post(api_intelligence_healer_execute),
        )
        .route(
            "/api/intelligence/evolution/execute",
            post(api_intelligence_evolution_execute),
        )
        .route(
            "/api/intelligence/gitops/agent/plan",
            get(api_intelligence_gitops_agent_plan),
        )
        .route(
            "/api/intelligence/gitops/agent/sync",
            post(api_intelligence_gitops_agent_sync),
        )
        .route(
            "/api/intelligence/cost-optimize/apply",
            post(api_intelligence_cost_optimize_apply),
        )
        .route(
            "/api/intelligence/security/remediate",
            post(api_intelligence_security_remediate),
        )
        .route(
            "/api/intelligence/capacity/scale-suggestions",
            get(api_intelligence_capacity_scale_suggestions),
        )
        .route(
            "/api/intelligence/capacity/scale/execute",
            post(api_intelligence_capacity_scale_execute),
        )
        .route(
            "/api/intelligence/intent/nl-parse",
            post(api_intelligence_intent_nl_parse),
        )
        .route(
            "/api/intelligence/intent-pipeline/deploy",
            post(api_intelligence_intent_pipeline_deploy),
        )
        .route(
            "/api/intelligence/intent/violations",
            get(api_intelligence_intent_violations),
        )
        .route(
            "/api/intelligence/intent/sla-breaches",
            get(api_intelligence_intent_sla_breaches),
        )
        .route(
            "/api/intelligence/intent/budget/enforce",
            post(api_intelligence_intent_budget_enforce),
        )
        .route(
            "/api/intelligence/intent/compliance/check",
            post(api_intelligence_intent_compliance_check),
        )
        .route(
            "/api/intelligence/intent/templates",
            get(api_intelligence_intent_templates),
        )
        .route(
            "/api/intelligence/intent/bundles",
            post(api_intelligence_intent_bundles),
        )
        .route(
            "/api/intelligence/intent/gitops-diff",
            get(api_intelligence_intent_gitops_diff),
        )
        .route(
            "/api/intelligence/intent/versions/:name",
            get(api_intelligence_intent_versions),
        )
        .route(
            "/api/intelligence/intent/versions/:name/rollback",
            post(api_intelligence_intent_version_rollback),
        )
        .route(
            "/api/intelligence/federation/execute",
            post(api_intelligence_federation_execute),
        )
        .route(
            "/api/intelligence/multicloud/cost-arbitrage",
            get(api_intelligence_multicloud_cost_arbitrage),
        )
        .route(
            "/api/intelligence/federation/health-mesh",
            get(api_intelligence_federation_health_mesh),
        )
        .route(
            "/api/intelligence/federation/unified-fabric",
            get(api_intelligence_federation_unified_fabric),
        )
        .route(
            "/api/intelligence/migration/volume-status",
            get(api_intelligence_migration_volume_status),
        )
        .route(
            "/api/intelligence/migration/wave-plan",
            get(api_intelligence_migration_wave_plan),
        )
        .route(
            "/api/intelligence/federation/geo-placement",
            get(api_intelligence_federation_geo_placement),
        )
        .route(
            "/api/intelligence/multicloud/cloud-accounts",
            get(api_intelligence_multicloud_cloud_accounts),
        )
        .route(
            "/api/intelligence/federation/region-lock",
            get(api_intelligence_federation_region_lock),
        )
        .route(
            "/api/intelligence/federation/packetwolf-guard",
            get(api_intelligence_federation_packetwolf_guard),
        )
        .route(
            "/api/intelligence/federation/packetwolf-guard/apply",
            post(api_intelligence_federation_packetwolf_guard_apply),
        )
        .route(
            "/api/intelligence/sre/schedule",
            get(api_intelligence_sre_schedule),
        )
        .route(
            "/api/intelligence/sre/incident-timeline",
            get(api_intelligence_sre_incident_timeline),
        )
        .route(
            "/api/intelligence/sre/on-call",
            get(api_intelligence_sre_on_call),
        )
        .route(
            "/api/intelligence/sre/on-call/test",
            post(api_intelligence_sre_on_call_test),
        )
        .route(
            "/api/intelligence/sre/postmortem",
            get(api_intelligence_sre_postmortem),
        )
        .route(
            "/api/intelligence/sre/error-budgets",
            get(api_intelligence_sre_error_budgets),
        )
        .route(
            "/api/intelligence/sre/chaos/experiments",
            get(api_intelligence_sre_chaos_experiments),
        )
        .route(
            "/api/intelligence/sre/chaos/run",
            post(api_intelligence_sre_chaos_run),
        )
        .route(
            "/api/intelligence/sre/game-days",
            get(api_intelligence_sre_game_days),
        )
        .route(
            "/api/intelligence/sre/runbook/execute",
            post(api_intelligence_sre_runbook_execute),
        )
        .route(
            "/api/intelligence/sre/escalation",
            get(api_intelligence_sre_escalation),
        )
        .route("/api/intelligence/sre/mttr", get(api_intelligence_sre_mttr))
        .route(
            "/api/intelligence/graph/interactive",
            get(api_intelligence_graph_interactive),
        )
        .route(
            "/api/intelligence/fabric/topology",
            get(api_intelligence_fabric_topology),
        )
        .route(
            "/api/intelligence/graph/impact",
            get(api_intelligence_graph_impact),
        )
        .route(
            "/api/intelligence/graph/blast-radius",
            get(api_intelligence_graph_blast_radius),
        )
        .route(
            "/api/intelligence/graph/import-k8s",
            post(api_intelligence_graph_import_k8s),
        )
        .route(
            "/api/intelligence/graph/threat-paths",
            get(api_intelligence_graph_threat_paths),
        )
        .route(
            "/api/intelligence/graph/search",
            get(api_intelligence_graph_search),
        )
        .route(
            "/api/intelligence/graph/snapshots",
            get(api_intelligence_graph_snapshots),
        )
        .route(
            "/api/intelligence/graph/snapshots/capture",
            post(api_intelligence_graph_snapshots_capture),
        )
        .route(
            "/api/intelligence/graph/cmdb",
            get(api_intelligence_graph_cmdb),
        )
        .route(
            "/api/intelligence/graph/cmdb/sync",
            post(api_intelligence_graph_cmdb_sync),
        )
        .route(
            "/api/intelligence/graph/placement",
            get(api_intelligence_graph_placement),
        )
        .route(
            "/api/intelligence/graph/export",
            get(api_intelligence_graph_export),
        )
        .route(
            "/api/intelligence/macos/tray-sparkline",
            get(api_intelligence_macos_tray_sparkline),
        )
        .route(
            "/api/intelligence/macos/live-activity",
            get(api_intelligence_macos_live_activity),
        )
        .route(
            "/api/intelligence/macos/dock-badge",
            get(api_intelligence_macos_dock_badge),
        )
        .route(
            "/api/intelligence/macos/notifications",
            get(api_intelligence_macos_notifications),
        )
        .route(
            "/api/intelligence/macos/spotlight",
            get(api_intelligence_macos_spotlight),
        )
        .route(
            "/api/intelligence/macos/shortcuts",
            get(api_intelligence_macos_shortcuts),
        )
        .route(
            "/api/intelligence/macos/menu-extras",
            get(api_intelligence_macos_menu_extras),
        )
        .route(
            "/api/intelligence/macos/offline-cache",
            get(api_intelligence_macos_offline_cache_get)
                .post(api_intelligence_macos_offline_cache_write),
        )
        .route(
            "/api/intelligence/macos/universal-links",
            get(api_intelligence_macos_universal_links),
        )
        .route(
            "/api/intelligence/macos/universal-links/resolve",
            get(api_intelligence_macos_universal_links_resolve),
        )
        .route(
            "/api/intelligence/macos/release-pipeline",
            get(api_intelligence_macos_release_pipeline),
        )
        .route(
            "/api/intelligence/evolution/status",
            get(api_intelligence_evolution_status),
        )
        .route(
            "/api/intelligence/runtime-evolution/:name",
            get(api_intelligence_runtime_evolution),
        )
        .route("/api/intelligence/place", post(api_intelligence_place))
        .route(
            "/api/intelligence/remediation/plan",
            get(api_intelligence_remediation_plan),
        )
        .route(
            "/api/intelligence/remediation/execute",
            post(api_intelligence_remediation_execute),
        )
        .route("/api/zeus/chat", post(api_zeus_chat))
        .route("/api/zeus/troubleshoot", post(api_zeus_troubleshoot))
        .route(
            "/api/zeus/troubleshoot/fleet",
            get(api_zeus_troubleshoot_fleet),
        )
        .route("/api/zeus/sessions/:id", get(api_zeus_session))
        .route("/api/zeus/confirm/:action_id", post(api_zeus_confirm))
        .route("/api/zeus/confirm-batch", post(api_zeus_confirm_batch))
        .route("/api/zeus/insights", get(api_zeus_insights))
        .route(
            "/api/zeus/providers",
            get(api_zeus_providers_list).post(api_zeus_providers_upsert),
        )
        .route("/api/zeus/providers/status", get(api_zeus_providers_status))
        .route(
            "/api/zeus/providers/registry",
            post(api_zeus_providers_save_registry),
        )
        .route(
            "/api/zeus/providers/:id",
            axum::routing::delete(api_zeus_providers_delete),
        )
        .route(
            "/api/zeus/providers/:id/test",
            post(api_zeus_providers_test),
        )
        .route(
            "/api/zeus/prompts",
            get(api_zeus_prompts_list).post(api_zeus_prompts_upsert),
        )
        .route("/api/zeus/prompts/export", get(api_zeus_prompts_export))
        .route("/api/zeus/prompts/import", post(api_zeus_prompts_import))
        .route(
            "/api/zeus/prompts/:id",
            axum::routing::delete(api_zeus_prompts_delete),
        )
        .route("/api/zeus/marketplace", get(api_zeus_marketplace))
        .route(
            "/api/zeus/marketplace/install",
            post(api_zeus_marketplace_install),
        )
        .route(
            "/api/zeus/marketplace/uninstall",
            post(api_zeus_marketplace_uninstall),
        )
        .route("/api/zeus/agents", get(api_zeus_agents_list))
        .route(
            "/api/intelligence/zeus/memory",
            get(api_intelligence_zeus_memory),
        )
        .route(
            "/api/intelligence/zeus/memory/settings",
            post(api_intelligence_zeus_memory_settings),
        )
        .route(
            "/api/intelligence/zeus/memory/purge",
            post(api_intelligence_zeus_memory_purge),
        )
        .route(
            "/api/intelligence/zeus/route",
            post(api_intelligence_zeus_route),
        )
        .route(
            "/api/intelligence/zeus/llm-status",
            get(api_intelligence_zeus_llm_status),
        )
        .route(
            "/api/intelligence/zeus/voice-lab",
            get(api_intelligence_zeus_voice_lab),
        )
        .route(
            "/api/intelligence/zeus/runbook",
            post(api_intelligence_zeus_runbook),
        )
        .route(
            "/api/intelligence/zeus/policy-explain",
            post(api_intelligence_zeus_policy_explain),
        )
        .route(
            "/api/intelligence/zeus/audit",
            get(api_intelligence_zeus_audit),
        )
        .route(
            "/api/intelligence/zeus/rbac-scopes",
            get(api_intelligence_zeus_rbac_scopes),
        )
        .route("/api/copilot/chat", post(api_copilot_chat))
        .route("/api/copilot/troubleshoot", post(api_copilot_troubleshoot))
        .route(
            "/api/copilot/troubleshoot/fleet",
            get(api_copilot_troubleshoot_fleet),
        )
        .route("/api/copilot/sessions/:id", get(api_copilot_session))
        .route("/api/copilot/confirm/:action_id", post(api_copilot_confirm))
        .route(
            "/api/copilot/confirm-batch",
            post(api_copilot_confirm_batch),
        )
        .route(
            "/api/intelligence/copilot/memory",
            get(api_intelligence_copilot_memory),
        )
        .route(
            "/api/intelligence/copilot/route",
            post(api_intelligence_copilot_route),
        )
        .route(
            "/api/intelligence/copilot/llm-status",
            get(api_intelligence_copilot_llm_status),
        )
        .route(
            "/api/intelligence/copilot/voice-lab",
            get(api_intelligence_copilot_voice_lab),
        )
        .route(
            "/api/intelligence/copilot/runbook",
            post(api_intelligence_copilot_runbook),
        )
        .route(
            "/api/intelligence/copilot/policy-explain",
            post(api_intelligence_copilot_policy_explain),
        )
        .route(
            "/api/intelligence/copilot/audit",
            get(api_intelligence_copilot_audit),
        )
        .route(
            "/api/intelligence/copilot/rbac-scopes",
            get(api_intelligence_copilot_rbac_scopes),
        )
        .route(
            "/api/intelligence/finops/chargeback",
            get(api_intelligence_finops_chargeback),
        )
        .route(
            "/api/intelligence/finops/spot-advisor",
            get(api_intelligence_finops_spot_advisor),
        )
        .route(
            "/api/intelligence/finops/reserved-planner",
            get(api_intelligence_finops_reserved_planner),
        )
        .route(
            "/api/intelligence/finops/anomalies",
            get(api_intelligence_finops_anomalies),
        )
        .route(
            "/api/intelligence/finops/unit-economics",
            get(api_intelligence_finops_unit_economics),
        )
        .route(
            "/api/intelligence/finops/execute",
            post(api_intelligence_finops_execute),
        )
        .route(
            "/api/intelligence/finops/multicloud-compare",
            get(api_intelligence_finops_multicloud_compare),
        )
        .route(
            "/api/intelligence/finops/carbon",
            get(api_intelligence_finops_carbon),
        )
        .route(
            "/api/intelligence/finops/budget-webhook",
            post(api_intelligence_finops_budget_webhook),
        )
        .route(
            "/api/intelligence/finops/trends",
            get(api_intelligence_finops_trends),
        )
        .route(
            "/api/intelligence/security/policy-apply",
            post(api_intelligence_security_policy_apply),
        )
        .route(
            "/api/intelligence/security/sbom-drift",
            get(api_intelligence_security_sbom_drift),
        )
        .route(
            "/api/intelligence/security/confidential-fleet",
            get(api_intelligence_security_confidential_fleet),
        )
        .route(
            "/api/intelligence/security/zero-trust-wizard",
            get(api_intelligence_security_zero_trust_wizard),
        )
        .route(
            "/api/intelligence/security/compliance-report",
            get(api_intelligence_security_compliance_report),
        )
        .route(
            "/api/intelligence/security/rotation-agent",
            post(api_intelligence_security_rotation_agent),
        )
        .route(
            "/api/intelligence/security/image-enforcement",
            get(api_intelligence_security_image_enforcement),
        )
        .route(
            "/api/intelligence/security/threat-hunt",
            post(api_intelligence_security_threat_hunt),
        )
        .route(
            "/api/intelligence/security/sovereign-audit",
            get(api_intelligence_security_sovereign_audit_get)
                .post(api_intelligence_security_sovereign_audit_append),
        )
        .route(
            "/api/intelligence/security/score-trend",
            get(api_intelligence_security_score_trend),
        )
        .route(
            "/v1/intelligence/briefing",
            get(api_v1_intelligence_briefing),
        )
        .route("/v1/intelligence/threats", get(api_v1_intelligence_threats))
        .route(
            "/v1/intelligence/cost-optimize",
            get(api_v1_intelligence_cost_optimize),
        )
        .route(
            "/v1/intelligence/predictions",
            get(api_v1_intelligence_predictions),
        )
        .route(
            "/v1/intelligence/autonomy",
            get(api_v1_intelligence_autonomy),
        )
        .route(
            "/api/intelligence/platform/saas-tenants",
            get(api_intelligence_platform_saas_tenants),
        )
        .route(
            "/api/intelligence/platform/plugin-marketplace",
            get(api_intelligence_platform_plugin_marketplace),
        )
        .route(
            "/api/intelligence/platform/helm-v2",
            post(api_intelligence_platform_helm_v2),
        )
        .route(
            "/api/intelligence/platform/terraform-export",
            get(api_intelligence_platform_terraform_export),
        )
        .route(
            "/api/intelligence/platform/pulumi-bridge",
            get(api_intelligence_platform_pulumi_bridge),
        )
        .route(
            "/api/intelligence/platform/public-api",
            get(api_intelligence_platform_public_api),
        )
        .route(
            "/api/intelligence/platform/mobile-companion",
            get(api_intelligence_platform_mobile_companion),
        )
        .route(
            "/api/intelligence/platform/ide-extensions",
            get(api_intelligence_platform_ide_extensions),
        )
        .route(
            "/api/intelligence/platform/community-intents",
            get(api_intelligence_platform_community_intents),
        )
        .route(
            "/api/intelligence/platform/autonomous-sre",
            get(api_intelligence_platform_autonomous_sre),
        )
        .route(
            "/api/intelligence/platform/autonomous-sre/execute",
            post(api_intelligence_platform_autonomous_sre_execute),
        )
        .route(
            "/api/intelligence/labs/overview",
            get(api_intelligence_labs_overview),
        )
        .route(
            "/api/intelligence/labs/terraform-export",
            post(api_intelligence_labs_terraform_export),
        )
        .route(
            "/api/intelligence/labs/pulumi-bridge",
            post(api_intelligence_labs_pulumi_bridge),
        )
        .route(
            "/api/intelligence/labs/mobile-companion",
            get(api_intelligence_labs_mobile_companion),
        )
        .route(
            "/api/intelligence/labs/ide-extensions",
            get(api_intelligence_labs_ide_extensions),
        )
        .route(
            "/api/intelligence/labs/community-intents",
            get(api_intelligence_labs_community_intents),
        )
        .route(
            "/api/intelligence/labs/community-intents/import",
            post(api_intelligence_labs_community_intents_import),
        )
        .route(
            "/api/intelligence/labs/carbon",
            get(api_intelligence_labs_carbon),
        )
        .route(
            "/api/intelligence/labs/compliance-report",
            get(api_intelligence_labs_compliance_report),
        )
        .route(
            "/api/intelligence/labs/voice-copilot",
            get(api_intelligence_labs_voice_copilot),
        )
        .route(
            "/api/intelligence/labs/graph-export",
            get(api_intelligence_labs_graph_export),
        )
        .route(
            "/api/intelligence/extensions/overview",
            get(api_intelligence_extensions_overview),
        )
        .route(
            "/api/intelligence/extensions/chaos/experiments",
            get(api_intelligence_extensions_chaos_experiments),
        )
        .route(
            "/api/intelligence/extensions/chaos/run",
            post(api_intelligence_extensions_chaos_run),
        )
        .route(
            "/api/intelligence/extensions/game-days",
            get(api_intelligence_extensions_game_days),
        )
        .route(
            "/api/intelligence/extensions/game-days/execute",
            post(api_intelligence_extensions_game_days_execute),
        )
        .route(
            "/api/intelligence/extensions/live-activity",
            get(api_intelligence_extensions_live_activity),
        )
        .route(
            "/api/intelligence/extensions/spotlight",
            get(api_intelligence_extensions_spotlight),
        )
        .route(
            "/api/intelligence/extensions/shortcuts",
            get(api_intelligence_extensions_shortcuts),
        )
        .route(
            "/api/intelligence/extensions/menu-extras",
            get(api_intelligence_extensions_menu_extras),
        )
        .route(
            "/api/intelligence/extensions/native-bundle",
            get(api_intelligence_extensions_native_bundle),
        )
        .route(
            "/api/intelligence/extensions/sre-bundle",
            get(api_intelligence_extensions_sre_bundle),
        )
        .route(
            "/api/intelligence/production/overview",
            get(api_intelligence_production_overview),
        )
        .route(
            "/api/intelligence/production/scorecard",
            get(api_intelligence_production_scorecard),
        )
        .route(
            "/api/intelligence/production/auth-plane",
            get(api_intelligence_production_auth_plane),
        )
        .route(
            "/api/intelligence/production/opa-plane",
            get(api_intelligence_production_opa_plane),
        )
        .route(
            "/api/intelligence/production/ha-plane",
            get(api_intelligence_production_ha_plane),
        )
        .route(
            "/api/intelligence/production/durability-plane",
            get(api_intelligence_production_durability_plane),
        )
        .route(
            "/api/intelligence/production/hosted-plane",
            get(api_intelligence_production_hosted_plane),
        )
        .route(
            "/api/intelligence/production/edge-fleet",
            get(api_intelligence_production_edge_fleet),
        )
        .route(
            "/api/intelligence/production/post-deploy-manifest",
            get(api_intelligence_production_post_deploy_manifest),
        )
        .route(
            "/api/intelligence/production/ci-smoke-manifest",
            get(api_intelligence_production_ci_smoke_manifest),
        )
        .route(
            "/api/intelligence/livelabs/overview",
            get(api_intelligence_livelabs_overview),
        )
        .route(
            "/api/intelligence/livelabs/reference-runner",
            get(api_intelligence_livelabs_reference_runner),
        )
        .route(
            "/api/intelligence/livelabs/kind-fixture",
            get(api_intelligence_livelabs_kind_fixture),
        )
        .route(
            "/api/intelligence/livelabs/live-smoke",
            get(api_intelligence_livelabs_live_smoke),
        )
        .route(
            "/api/intelligence/livelabs/post-deploy-verify",
            get(api_intelligence_livelabs_post_deploy_verify),
        )
        .route(
            "/api/intelligence/livelabs/kubernetes-lab",
            get(api_intelligence_livelabs_kubernetes_lab),
        )
        .route(
            "/api/intelligence/livelabs/advanced-runtime-labs",
            get(api_intelligence_livelabs_advanced_runtime_labs),
        )
        .route(
            "/api/intelligence/livelabs/ci-pipeline",
            get(api_intelligence_livelabs_ci_pipeline),
        )
        .route(
            "/api/intelligence/livelabs/cluster-exec",
            get(api_intelligence_livelabs_cluster_exec),
        )
        .route(
            "/api/intelligence/livelabs/confidential-lab",
            get(api_intelligence_livelabs_confidential_lab),
        )
        .route(
            "/api/confidential/capabilities",
            get(api_confidential_capabilities),
        )
        .route(
            "/api/confidential/security-profiles",
            get(api_confidential_security_profiles),
        )
        .route(
            "/api/confidential/attestation/verify",
            post(api_attestation_verify),
        )
        .route(
            "/api/confidential/attestation/:vm_id/status",
            get(api_attestation_status),
        )
        .route(
            "/api/confidential/attestation/:vm_id/explain",
            get(api_attestation_explain),
        )
        .route(
            "/api/confidential/trust-score/:workload",
            get(api_confidential_trust_score),
        )
        .route(
            "/api/confidential/trust-score",
            get(api_confidential_trust_fleet),
        )
        .route("/api/confidential/fleet", get(api_confidential_fleet))
        .route(
            "/api/confidential/workload/:workload",
            get(api_confidential_workload_row),
        )
        .route(
            "/api/confidential/secrets/release",
            post(api_confidential_secret_release),
        )
        .route(
            "/api/confidential/secrets/:workload/status",
            get(api_confidential_secret_status),
        )
        .route(
            "/api/confidential/migration-plan/:name/:target",
            get(api_confidential_migration_plan),
        )
        .route(
            "/api/confidential/migration/:workload/status",
            get(api_confidential_migration_status),
        )
        .route(
            "/api/confidential/guestkit/inspect",
            post(api_guestkit_inspect),
        )
        .route(
            "/api/confidential/guestkit/:vm_id/history",
            get(api_guestkit_history),
        )
        .route(
            "/api/confidential/sovereign/status",
            get(api_confidential_sovereign_status),
        )
        .route(
            "/api/confidential/sovereign/evaluate/:workload",
            get(api_confidential_sovereign_evaluate),
        )
        .route(
            "/api/confidential/kata/status",
            get(api_confidential_kata_status),
        )
        .route(
            "/api/confidential/network/:workload",
            get(api_confidential_network_status),
        )
        .route(
            "/api/confidential/intelligence/:workload",
            get(api_confidential_intelligence_workload),
        )
        .route(
            "/api/confidential/intelligence",
            get(api_confidential_intelligence_fleet),
        )
        .route(
            "/api/confidential/images",
            get(api_confidential_image_catalog),
        )
        .route(
            "/api/confidential/images/sign",
            post(api_confidential_image_sign),
        )
        .route(
            "/api/confidential/isolation/:workload",
            get(api_confidential_isolation),
        )
        .route(
            "/api/confidential/placement/:workload",
            get(api_confidential_placement),
        )
        .route(
            "/api/confidential/images/verify",
            post(api_confidential_image_verify),
        )
        .route("/api/drift/:name", get(api_drift_check))
        .route("/api/drift/:name/reconcile", post(api_drift_reconcile))
        .route("/api/alerts/status", get(api_alerts_status))
        .route("/api/policy/check", post(api_policy_check))
        .route("/api/policy/opa", post(api_policy_opa))
        .route(
            "/api/dependencies",
            get(api_deps_show)
                .post(api_deps_add)
                .delete(api_deps_remove),
        )
        .route("/api/audit", get(api_audit_list))
        .route("/api/audit/events", post(api_audit_append))
        .route("/api/templates", get(api_template_list))
        .route("/api/templates/:name", post(api_template_generate))
        .route("/api/sla", get(api_sla_list).post(api_sla_add))
        .route("/api/sla/:workload", get(api_sla_check))
        .route("/api/events", get(api_events_list))
        .route("/api/events/summary", get(api_events_summary))
        .route(
            "/api/platform/recommendations",
            get(api_platform_recommendations),
        )
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
        .route(
            "/api/cluster/port-forward",
            post(api_cluster_port_forward_start),
        )
        .route(
            "/api/cluster/port-forward/stop",
            post(api_cluster_port_forward_stop),
        )
        .route("/api/cluster/top", get(api_cluster_top))
        .route(
            "/api/cluster/metrics/summary",
            get(api_cluster_metrics_summary),
        )
        .route("/api/cluster/cilium/status", get(api_cluster_cilium_status))
        .route(
            "/api/cluster/cilium/connectivity/probe",
            post(api_cluster_cilium_connectivity_probe),
        )
        .route("/api/cluster/cilium/hubble", get(api_cluster_cilium_hubble))
        .route("/api/cluster/diff", post(api_cluster_diff))
        .route("/api/cluster/rollout", get(api_cluster_rollout))
        .route(
            "/api/cluster/rollout/action",
            post(api_cluster_rollout_action),
        )
        .route("/api/cluster/helm/history", get(api_cluster_helm_history))
        .route("/api/cluster/helm/action", post(api_cluster_helm_action))
        .route("/api/environments", get(api_env_list).post(api_env_create))
        .route("/api/environments/promote", post(api_env_promote))
        .route("/api/environments/parity", get(api_env_parity))
        .route("/api/scheduler/utilization", get(api_scheduler_utilization))
        .route("/api/scheduler/optimize", get(api_scheduler_optimize))
        .route("/api/scheduler/placements", get(api_scheduler_placements))
        .route("/api/orchestrator/status", get(api_orchestrator_status))
        .route("/api/orchestrator/summary", get(api_orchestrator_summary))
        .route(
            "/api/orchestrator/register",
            post(api_orchestrator_register),
        )
        .route(
            "/api/orchestrator/health-check",
            post(api_orchestrator_health_check),
        )
        .route(
            "/api/orchestrator/reset-circuit",
            post(api_orchestrator_reset_circuit),
        )
        .route(
            "/api/orchestrator/rolling-update",
            post(api_orchestrator_rolling_update),
        )
        .route("/api/affinity/matrix", get(api_affinity_matrix))
        .route("/api/affinity/stats", get(api_affinity_stats))
        .route("/api/affinity/:class", get(api_affinity_recommend))
        .route("/api/plugins", get(api_plugins_list))
        .route("/api/plugins/register", post(api_plugins_register))
        .route("/api/plugins/discover", post(api_plugins_discover))
        .route("/api/plugins/:name", delete(api_plugins_remove))
        .route("/api/health/:workload", get(api_health_summary))
        .route("/api/compose/validate", post(api_compose_validate))
        .route("/api/compose/up", post(api_compose_up))
        .route("/api/compose/down", post(api_compose_down))
        .route("/api/helm/catalog", get(api_helm_catalog))
        .route("/api/helm/export", post(api_helm_export))
        .route("/api/audit/verify", get(api_audit_verify))
        .route("/api/gitops/status", get(api_gitops_status))
        .route("/api/gitops/init", post(api_gitops_init))
        .route("/api/gitops/preview", post(api_gitops_preview))
        .route("/api/gitops/sync", post(api_gitops_sync))
        .route("/api/webhooks/test", post(api_webhook_test))
        .route("/api/webhooks/queue", get(api_webhooks_queue))
        .route("/api/webhooks/flush", post(api_webhooks_flush))
        .route("/api/webhooks/channels", post(api_webhook_channel_create))
        .route(
            "/api/webhooks/channels/:name",
            delete(api_webhook_channel_delete),
        )
        .route("/api/dashboard/version", get(api_dashboard_version))
        .route("/api/server", get(api_server_info))
        .route("/api/openapi.json", get(serve_openapi))
        .route("/api/rbac/keys", get(rbac_list_keys))
        .route("/api/rbac/keys", post(rbac_create_key))
        .route("/api/rbac/keys/revoke", post(rbac_revoke_key))
        .route("/api/security/sbom", get(api_security_sbom))
        .route("/api/security/images", get(api_security_images))
        .route(
            "/api/security/images/verify",
            post(api_security_images_verify),
        )
        .route(
            "/api/ecosystem/packetwolf/status",
            get(api_packetwolf_status),
        )
        .route(
            "/api/ecosystem/packetwolf/flows/stats",
            get(api_packetwolf_flow_stats),
        )
        .route(
            "/api/ecosystem/packetwolf/verify-egress",
            post(api_packetwolf_verify_egress),
        )
        .route(
            "/api/ecosystem/packetwolf/anomalies",
            get(api_packetwolf_anomalies),
        )
        .route(
            "/api/ecosystem/packetwolf/deeplink",
            get(api_packetwolf_deeplink),
        )
        .route("/api/fleet/edge/register", post(api_fleet_edge_register))
        .route("/api/fleet/edge/heartbeat", post(api_fleet_edge_heartbeat))
        .route("/api/fleet/edge/agents", get(api_fleet_edge_agents))
        .route("/api/fleet/edge/enqueue", post(api_fleet_edge_enqueue))
        .route("/api/fleet/edge/queue", get(api_fleet_edge_queue))
        .route(
            "/api/fleet/federation/policies",
            get(api_fleet_federation_policies),
        )
        .route(
            "/api/fleet/federation/plan",
            post(api_fleet_federation_plan),
        )
        .route("/api/fleet/drift", get(api_fleet_drift))
        .route(
            "/api/gitops/resolve-target",
            post(api_gitops_resolve_target),
        )
        .route(
            "/api/migration/volume/plan",
            post(api_migration_volume_plan),
        )
        .route(
            "/api/migration/volume/execute",
            post(api_migration_volume_execute),
        )
        .route("/api/migration/fleet/plan", post(api_migration_fleet_plan))
        .route(
            "/api/hosted/tenants",
            get(api_hosted_tenants_list).post(api_hosted_tenants_create),
        )
        .route(
            "/api/hosted/tenants/:id/deactivate",
            post(api_hosted_tenants_deactivate),
        )
        .route(
            "/api/hosted/tenants/:id/upgrade",
            post(api_hosted_tenants_upgrade),
        )
        .route("/api/hosted/upgrades", get(api_hosted_upgrades_status))
        .route(
            "/api/hosted/tenants/:id/keys",
            get(api_hosted_tenant_keys_list).post(api_hosted_tenant_keys_issue),
        )
        .route(
            "/api/hosted/tenants/:id/keys/:name/revoke",
            post(api_hosted_tenant_keys_revoke),
        )
        .route("/api/hosted/billing/usage", get(api_hosted_billing_usage))
        .route(
            "/api/hosted/billing/metering",
            get(api_hosted_metering_usage),
        )
        .route(
            "/api/hosted/billing/stripe/checkout",
            post(api_hosted_billing_stripe_checkout),
        )
        .route(
            "/api/hosted/billing/stripe/webhook",
            post(api_hosted_billing_stripe_webhook),
        );

    if crate::mock_idp::enabled() {
        app = app
            .route("/api/mock-idp/saml/sso", get(crate::mock_idp::saml_sso))
            .route(
                "/api/mock-idp/oidc/.well-known/openid-configuration",
                get(crate::mock_idp::oidc_discovery),
            )
            .route(
                "/api/mock-idp/oidc/authorize",
                get(crate::mock_idp::oidc_authorize),
            )
            .route(
                "/api/mock-idp/oidc/token",
                post(crate::mock_idp::oidc_token),
            )
            .route("/api/mock-idp/oidc/jwks", get(crate::mock_idp::oidc_jwks));
    }

    let app = app
        .fallback(get(serve_dashboard_spa_fallback))
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024)) // 2 MB max request body
                .layer(tower::limit::ConcurrencyLimitLayer::new(200))
                .layer(cors)
                .layer(middleware::from_fn_with_state(
                    app_state.clone(),
                    auth_middleware,
                ))
                .layer(middleware::from_fn(observability_middleware))
                .layer(middleware::from_fn(security_headers_middleware)),
        )
        .with_state(app_state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port).parse::<SocketAddr>()?;

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

        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
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
