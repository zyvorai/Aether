// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! API handler functions

use super::types::*;
use anyhow::Context;
use crate::config::Config;
use crate::engine::Engine;
use crate::kubecluster::ClusterLogsRequest;
use crate::runtime::{self, RuntimeKind};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use crate::{backup, cost, Runtime};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Form, Path, Query, RawQuery, State as AxumState,
    },
    body::Body,
    http::HeaderMap,
    http::HeaderValue,
    http::StatusCode,
    http::{Method, Uri},
    response::{Html, IntoResponse, Json, Response},
};
use axum::http::header;
use futures::{SinkExt, stream::StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    process::Command,
    sync::mpsc,
    time::{sleep, Duration},
};

fn clamp_page(limit: Option<usize>, offset: Option<usize>, default_limit: usize, max_limit: usize) -> (usize, usize) {
    let lim = limit.unwrap_or(default_limit).min(max_limit).max(1);
    let off = offset.unwrap_or(0).min(100_000);
    (lim, off)
}

/// Persist workload state to Postgres (if enabled) and the configured JSON file.
pub(crate) async fn persist_workload_api(app: &AppState, store: &StateStore) -> anyhow::Result<()> {
    crate::state_postgres::persist_workload_state(
        app.workload_state_pg.as_ref(),
        &app.state_path,
        store,
    )
    .await
}

fn workload_spec_file(name: &str) -> PathBuf {
    crate::resources::aether_path(&format!("specs/{name}.yaml"))
}

/// Validate backup stem or filename and return path under `backup_dir`.
fn resolve_backup_path(backup_dir: &std::path::Path, name: &str) -> Result<PathBuf, String> {
    let stem = name.strip_suffix(".json").unwrap_or(name);
    if stem.is_empty() || stem.len() > 200 {
        return Err("invalid backup name".into());
    }
    if !stem
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("invalid backup name: use alphanumeric, dot, underscore, hyphen only".into());
    }
    Ok(backup_dir.join(format!("{stem}.json")))
}

fn parse_audit_action_from_str(s: &str) -> crate::audit::AuditAction {
    match s.to_ascii_lowercase().as_str() {
        "build" => crate::audit::AuditAction::Build,
        "deploy" => crate::audit::AuditAction::Deploy,
        "start" => crate::audit::AuditAction::Start,
        "stop" => crate::audit::AuditAction::Stop,
        "delete" => crate::audit::AuditAction::Delete,
        "migrate" => crate::audit::AuditAction::Migrate,
        "scale" => crate::audit::AuditAction::Scale,
        "config" | "configchange" => crate::audit::AuditAction::ConfigChange,
        "backup" | "backupcreate" => crate::audit::AuditAction::BackupCreate,
        "restore" | "backuprestore" => crate::audit::AuditAction::BackupRestore,
        "policy" | "policycheck" => crate::audit::AuditAction::PolicyCheck,
        "drift" | "driftdetected" => crate::audit::AuditAction::DriftDetected,
        "external" => crate::audit::AuditAction::External,
        _ => crate::audit::AuditAction::External,
    }
}

fn parse_action_result_from_str(s: &str) -> crate::audit::ActionResult {
    match s.to_ascii_lowercase().as_str() {
        "success" | "ok" => crate::audit::ActionResult::Success,
        "failure" | "fail" | "error" => crate::audit::ActionResult::Failure,
        "warning" | "warn" => crate::audit::ActionResult::Warning,
        _ => crate::audit::ActionResult::Success,
    }
}

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

fn parse_workload_payload<T: serde::Serialize>(
    value: serde_json::Value,
) -> Result<Workload, (StatusCode, Json<ApiResponse<T>>)> {
    if let Ok(spec) = serde_json::from_value::<Workload>(value.clone()) {
        return Ok(spec);
    }

    if let Some(spec_value) = value.get("spec") {
        if let Ok(spec) = serde_json::from_value::<Workload>(spec_value.clone()) {
            return Ok(spec);
        }
    }

    if let Some(yaml) = value.get("yaml").and_then(|v| v.as_str()) {
        return crate::legacy_workload_yaml::parse_workload_yaml(yaml)
            .map_err(|e| err_bad_request(format!("Invalid YAML: {}", e)));
    }

    Err(err_bad_request("Request body must be a workload object, { spec: ... }, or { yaml: ... }"))
}

fn parse_create_workload_payload<T: serde::Serialize>(
    value: serde_json::Value,
) -> Result<CreateWorkloadRequest, (StatusCode, Json<ApiResponse<T>>)> {
    if let Ok(request) = serde_json::from_value::<CreateWorkloadRequest>(value.clone()) {
        return Ok(request);
    }

    if let Some(spec_yaml) = value.get("spec_yaml").and_then(|v| v.as_str()) {
        let spec = crate::legacy_workload_yaml::parse_workload_yaml(spec_yaml)
            .map_err(|e| err_bad_request(format!("Invalid workload YAML: {}", e)))?;
        let runtime = value
            .get("runtime")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned);

        return Ok(CreateWorkloadRequest { spec, runtime });
    }

    let runtime = value
        .get("runtime")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let spec = parse_workload_payload::<T>(value)?;
    Ok(CreateWorkloadRequest { spec, runtime })
}

/// Look up a workload by name from state, returning a cloned WorkloadState
/// or an HTTP 404 error response.
pub(crate) async fn lookup_workload<T: serde::Serialize>(
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
pub(crate) fn ok_json<T: serde::Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
    (StatusCode::OK, Json(ApiResponse::success(data)))
}

/// Shorthand for a 201 Created JSON response.
fn created_json<T: serde::Serialize>(data: T) -> (StatusCode, Json<ApiResponse<T>>) {
    (StatusCode::CREATED, Json(ApiResponse::success(data)))
}

/// Shorthand for an internal-server-error JSON response.
pub(crate) fn err_internal<T: serde::Serialize>(e: impl std::fmt::Display) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiResponse::error(e.to_string())),
    )
}

/// Shorthand for a bad-request JSON response.
pub(crate) fn err_bad_request<T: serde::Serialize>(e: impl std::fmt::Display) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::error(e.to_string())),
    )
}

/// Shorthand for a not-found JSON response.
pub(crate) fn err_not_found<T: serde::Serialize>(msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error(msg.into())),
    )
}

/// Shorthand for a forbidden JSON response.
fn err_forbidden<T: serde::Serialize>(msg: impl Into<String>) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::FORBIDDEN,
        Json(ApiResponse::error(msg.into())),
    )
}

/// When `AETHER_OPA_ENFORCE` is set, reject workload deploy/update on OPA deny.
async fn opa_enforce_workload<T: serde::Serialize>(
    spec: &Workload,
    workload_ref: &str,
) -> Option<(StatusCode, Json<ApiResponse<T>>)> {
    if !crate::opa::enforce_enabled() {
        return None;
    }
    match crate::opa::evaluate_workload_optional(spec).await {
        Ok(ev) if !ev.allowed => {
            let msg = if ev.denials.is_empty() {
                "OPA policy denied this workload".to_string()
            } else {
                format!("OPA denied: {}", ev.denials.join("; "))
            };
            record_audit_event(
                crate::audit::AuditAction::PolicyCheck,
                workload_ref,
                None,
                crate::audit::ActionResult::Failure,
                &msg,
                None,
            );
            Some(err_forbidden(msg))
        }
        Err(e) => Some(err_internal(format!("OPA evaluation failed: {e}"))),
        _ => None,
    }
}

fn record_audit_event(
    action: crate::audit::AuditAction,
    workload: &str,
    runtime: Option<&str>,
    result: crate::audit::ActionResult,
    message: &str,
    details: Option<&str>,
) {
    let path = crate::audit::AuditLog::default_path();
    let mut log = crate::audit::AuditLog::load(&path).unwrap_or_default();
    log.record(action, workload, runtime, result, message, details);
    let _ = log.save(&path);
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn kubectl_resource_name(kind: &str) -> Option<&'static str> {
    match kind {
        "Namespace" => Some("namespaces"),
        "Pod" => Some("pods"),
        "ServiceAccount" => Some("serviceaccounts"),
        "Secret" => Some("secrets"),
        "PersistentVolumeClaim" => Some("persistentvolumeclaims"),
        "Deployment" => Some("deployments.apps"),
        "StatefulSet" => Some("statefulsets.apps"),
        "DaemonSet" => Some("daemonsets.apps"),
        "Job" => Some("jobs.batch"),
        "CronJob" => Some("cronjobs.batch"),
        "HorizontalPodAutoscaler" => Some("horizontalpodautoscalers.autoscaling"),
        "Service" => Some("services"),
        "Ingress" => Some("ingresses.networking.k8s.io"),
        "NetworkPolicy" => Some("networkpolicies.networking.k8s.io"),
        "ConfigMap" => Some("configmaps"),
        "Event" => Some("events"),
        "Node" => Some("nodes"),
        "PersistentVolume" => Some("persistentvolumes"),
        "ResourceQuota" => Some("resourcequotas"),
        "LimitRange" => Some("limitranges"),
        "EndpointSlice" => Some("endpointslices.discovery.k8s.io"),
        "StorageClass" => Some("storageclasses.storage.k8s.io"),
        "DataVolume" => Some("datavolumes.cdi.kubevirt.io"),
        "VirtualMachine" => Some("virtualmachines.kubevirt.io"),
        "VirtualMachineInstance" => Some("virtualmachineinstances.kubevirt.io"),
        _ => None,
    }
}

fn build_kubectl_command(context: &str) -> Command {
    let mut command = Command::new("kubectl");
    command.arg("--context").arg(context);
    command
}

async fn read_child_stream_to_channel<T>(
    stream: T,
    tx: mpsc::UnboundedSender<String>,
) where
    T: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(stream);
    let mut buffer = vec![0_u8; 4096];

    loop {
        match tokio::io::AsyncReadExt::read(&mut reader, &mut buffer).await {
            Ok(0) => break,
            Ok(bytes) => {
                let chunk = String::from_utf8_lossy(&buffer[..bytes]).to_string();
                if tx.send(chunk).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = tx.send(format!("\n[aether] stream error: {error}\n"));
                break;
            }
        }
    }
}

async fn allocate_local_port(requested: Option<u16>) -> anyhow::Result<u16> {
    let bind_addr = format!("127.0.0.1:{}", requested.unwrap_or(0));
    let listener = TcpListener::bind(&bind_addr).await?;
    Ok(listener.local_addr()?.port())
}

async fn run_kubectl(args: Vec<String>) -> anyhow::Result<String> {
    let output = Command::new("kubectl").args(args).output().await?;
    if !output.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn rollout_resource(kind: &str) -> Option<&'static str> {
    match kind {
        "Deployment" => Some("deployment"),
        "StatefulSet" => Some("statefulset"),
        "DaemonSet" => Some("daemonset"),
        _ => None,
    }
}

/// Embedded dashboard HTML (built from web/dashboard/ React app)
pub(crate) const DASHBOARD_HTML: &str = include_str!("../../web/dashboard/dist/index.html");
// Run `npm run build` in web/dashboard/ after UI changes (stable asset names in vite.config.ts).
const DASHBOARD_CSS: &str = include_str!("../../web/dashboard/dist/assets/aether-dashboard.css");
const DASHBOARD_JS: &str = include_str!("../../web/dashboard/dist/assets/aether-dashboard.js");
const DASHBOARD_CACHE_CONTROL: &str = "no-store, no-cache, must-revalidate, max-age=0";

/// GET / - Serve the web dashboard
pub(crate) async fn serve_dashboard() -> impl IntoResponse {
    (
        [
            (header::CACHE_CONTROL, DASHBOARD_CACHE_CONTROL),
            (header::PRAGMA, "no-cache"),
        ],
        Html(DASHBOARD_HTML),
    )
}

/// GET /assets/*.css - Serve embedded dashboard CSS
pub(crate) async fn serve_dashboard_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css"),
            (header::CACHE_CONTROL, DASHBOARD_CACHE_CONTROL),
            (header::PRAGMA, "no-cache"),
        ],
        DASHBOARD_CSS,
    )
}

/// GET /assets/*.js - Serve embedded dashboard JS
pub(crate) async fn serve_dashboard_js() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/javascript"),
            (header::CACHE_CONTROL, DASHBOARD_CACHE_CONTROL),
            (header::PRAGMA, "no-cache"),
        ],
        DASHBOARD_JS,
    )
}

/// SPA fallback: non-API GET requests serve the dashboard (deep links, refresh).
pub(crate) async fn serve_dashboard_spa_fallback(method: Method, uri: Uri) -> impl IntoResponse {
    if method != Method::GET {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = uri.path();
    if path.starts_with("/api") {
        return StatusCode::NOT_FOUND.into_response();
    }
    (
        [
            (header::CACHE_CONTROL, DASHBOARD_CACHE_CONTROL),
            (header::PRAGMA, "no-cache"),
        ],
        Html(DASHBOARD_HTML),
    ).into_response()
}

/// GET /health - Health check endpoint
pub(crate) async fn health_check() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Json(ApiResponse::success(response))
}

/// GET /api/auth/me - Return the effective authenticated role for the current request.
pub(crate) async fn api_auth_me(
    AxumState(app_state): AxumState<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_bearer_token(&headers);
    let legacy_key = std::env::var("AETHER_API_KEY").ok().filter(|value| !value.is_empty());

    {
        let rbac_store = app_state.rbac.read().await;
        if let Some(token) = token.as_deref() {
            if let Some(entry) = rbac_store.verify_key(token) {
                return ok_json(AuthStatusResponse {
                    authenticated: true,
                    username: entry.name.clone(),
                    role: entry.role.to_string(),
                });
            }
        }

        if legacy_key.is_none() && rbac_store.list_keys().is_empty() && app_state.oidc.is_none() {
            return ok_json(AuthStatusResponse {
                authenticated: true,
                username: "local-dev".to_string(),
                role: "admin".to_string(),
            });
        }
    }

    if let Some(oidc) = app_state.oidc.as_ref() {
        if let Some((role, username)) = oidc.verify_session_cookie(&headers) {
            return ok_json(AuthStatusResponse {
                authenticated: true,
                username,
                role: role.to_string(),
            });
        }
    }

    if let (Some(expected), Some(token)) = (legacy_key, token) {
        use sha2::{Digest, Sha256};
        let token_hash = Sha256::digest(token.as_bytes());
        let expected_hash = Sha256::digest(expected.as_bytes());
        if token_hash == expected_hash {
            return ok_json(AuthStatusResponse {
                authenticated: true,
                username: "api-key".to_string(),
                role: "admin".to_string(),
            });
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(ApiResponse::error("not authenticated".to_string())),
    )
}

/// GET /api/auth/oidc/login — redirect to the configured IdP (requires OIDC env).
pub(crate) async fn api_oidc_login(
    AxumState(app_state): AxumState<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let Some(oidc) = app_state.oidc.as_ref() else {
        return (StatusCode::NOT_FOUND, "OIDC not configured").into_response();
    };
    let next = params.get("next").map(|s| s.as_str());
    match oidc.begin_login(next).await {
        Ok(r) => r.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/auth/oidc/callback — OAuth redirect target; sets session cookie and returns to `next`.
pub(crate) async fn api_oidc_callback(
    AxumState(app_state): AxumState<AppState>,
    RawQuery(raw): RawQuery,
) -> impl IntoResponse {
    let Some(oidc) = app_state.oidc.as_ref() else {
        return (StatusCode::NOT_FOUND, "OIDC not configured").into_response();
    };
    let q = raw.unwrap_or_default();
    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(q.as_bytes())
        .into_owned()
        .collect();
    match oidc.finish_login(&pairs, app_state.tls_active).await {
        Ok(r) => r.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

/// GET /api/auth/oidc/logout — clear OIDC session cookie and redirect to `/`.
pub(crate) async fn api_oidc_logout(AxumState(app_state): AxumState<AppState>) -> impl IntoResponse {
    let mut res = axum::response::Redirect::to("/").into_response();
    if app_state.oidc.is_some() {
        res.headers_mut().insert(
            header::SET_COOKIE,
            crate::oidc::OidcRuntime::clear_session_cookie(app_state.tls_active),
        );
    }
    if app_state.saml.is_some() {
        res.headers_mut().insert(
            header::SET_COOKIE,
            crate::saml::SamlRuntime::clear_session_cookie(app_state.tls_active),
        );
    }
    res
}

/// GET /api/auth/saml/login — redirect to IdP SSO URL with AuthnRequest.
pub(crate) async fn api_saml_login(
    AxumState(app_state): AxumState<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let Some(saml) = app_state.saml.as_ref() else {
        return (StatusCode::NOT_FOUND, "SAML not configured").into_response();
    };
    let next = params.get("next").map(|s| s.as_str());
    match saml.begin_login(next).await {
        Ok(r) => r.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub(crate) struct SamlAcsForm {
    #[serde(rename = "SAMLResponse")]
    saml_response: String,
    #[serde(default, rename = "RelayState")]
    relay_state: Option<String>,
}

/// POST /api/auth/saml/acs — consume SAMLResponse and set session cookie.
pub(crate) async fn api_saml_acs(
    AxumState(app_state): AxumState<AppState>,
    Form(form): Form<SamlAcsForm>,
) -> impl IntoResponse {
    let Some(saml) = app_state.saml.as_ref() else {
        return (StatusCode::NOT_FOUND, "SAML not configured").into_response();
    };
    match saml
        .finish_acs(
            &form.saml_response,
            form.relay_state.as_deref(),
            app_state.tls_active,
        )
        .await
    {
        Ok(r) => r.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

/// GET /api/auth/saml/logout — clear SAML session cookie and redirect to `/`.
pub(crate) async fn api_saml_logout(AxumState(app_state): AxumState<AppState>) -> impl IntoResponse {
    let mut res = axum::response::Redirect::to("/").into_response();
    if app_state.saml.is_some() {
        res.headers_mut().insert(
            header::SET_COOKIE,
            crate::saml::SamlRuntime::clear_session_cookie(app_state.tls_active),
        );
    }
    res
}

/// GET /api/system/ready — combined readiness (Postgres workload state + Redis OIDC cache when configured).
pub(crate) async fn api_system_ready(AxumState(app_state): AxumState<AppState>) -> impl IntoResponse {
    let ha_redis = app_state.shared_cache.uses_redis();
    let redis_ok = app_state.shared_cache.redis_ping_ok().await;
    let postgres_required = app_state.workload_state_pg.is_some();
    let postgres_ok = if let Some(ref pg) = app_state.workload_state_pg {
        pg.ping_ok().await
    } else {
        true
    };
    let workload_backend = if postgres_required {
        "postgresql"
    } else {
        "local-json"
    };
    let ready = (!ha_redis || redis_ok) && (!postgres_required || postgres_ok);
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(serde_json::json!({
            "ready": ready,
            "redis": redis_ok,
            "ha_shared_cache": ha_redis,
            "postgres": postgres_ok,
            "workload_state_backend": workload_backend,
            "checks": {
                "process": true,
                "redis": { "required": ha_redis, "ok": redis_ok },
                "postgres": { "required": postgres_required, "ok": postgres_ok },
            },
        })),
    )
        .into_response()
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
                let managed = labels.is_some_and(|l| l.get("managed-by").is_some_and(|v| v == "aether"));

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
                let managed = labels.is_some_and(|l| l.get("managed-by").is_some_and(|v| v == "aether"));

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
    Query(page): Query<PaginationQuery>,
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

    let total = workloads.len();
    let use_page = page.limit.is_some() || page.offset.is_some();
    let workloads = if use_page {
        let (lim, off) = clamp_page(page.limit, page.offset, 50, 500);
        workloads.into_iter().skip(off).take(lim).collect()
    } else {
        workloads
    };

    if use_page {
        let json = match serde_json::to_string(&ApiResponse::success(workloads)) {
            Ok(s) => s,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<WorkloadResponse>>::error(e.to_string())),
                )
                    .into_response();
            }
        };
        let mut res = Response::new(Body::from(json));
        *res.status_mut() = StatusCode::OK;
        if let Ok(v) = HeaderValue::from_str(&total.to_string()) {
            res.headers_mut().insert("x-total-count", v);
        }
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        res.into_response()
    } else {
        Json(ApiResponse::success(workloads)).into_response()
    }
}

/// Deploy a workload spec through the API (shared by create and compose-up).
async fn deploy_workload_spec(
    app_state: &AppState,
    request: CreateWorkloadRequest,
) -> Result<String, (StatusCode, Json<ApiResponse<String>>)> {
    if let Some(resp) = opa_enforce_workload::<String>(&request.spec, &request.spec.metadata.name).await {
        return Err(resp);
    }

    let runtime_kind = if let Some(runtime_name) = request.runtime {
        match runtime_name.parse::<RuntimeKind>() {
            Ok(rt) => rt,
            Err(e) => return Err(err_bad_request(e)),
        }
    } else {
        let engine = Engine::new();
        match engine.decide(&request.spec) {
            Ok(runtime) => runtime,
            Err(e) => return Err(err_internal(e)),
        }
    };

    let name = request.spec.metadata.name.clone();

    if request.spec.confidential.as_ref().is_some_and(|c| c.enabled) {
        crate::ragnarok::image::deploy_image_gate(
            &request.spec,
            &crate::ragnarok::image::ImageCatalog::load(
                &crate::ragnarok::client::RagnarokClient::attestation_data_dir(),
            ),
        )
        .map_err(err_bad_request)?;
        crate::ragnarok::isolation::deploy_isolation_gate(&request.spec)
            .map_err(err_bad_request)?;
        crate::ragnarok::sovereign::deploy_sovereign_gate(&request.spec)
            .map_err(err_bad_request)?;
        crate::ragnarok::kata::deploy_kata_gate(&request.spec)
            .map_err(err_bad_request)?;
    }

    let runtime = make_runtime::<String>(&runtime_kind).await?;
    let image = runtime.build(&request.spec).await.map_err(err_internal)?;
    let instance = runtime.run(&image, &request.spec).await.map_err(err_internal)?;

    if request.spec.confidential.as_ref().is_some_and(|c| c.enabled) {
        if let Err(e) =
            crate::ragnarok::attestation_gate_for_workload(&request.spec, &name).await
        {
            return Err(err_bad_request(e));
        }
        let broker = crate::ragnarok::secrets::SecretBroker::new(
            std::sync::Arc::new(crate::ragnarok::attestation::AttestationService::new(
                crate::ragnarok::client::RagnarokClient::attestation_data_dir(),
            )),
            &crate::ragnarok::client::RagnarokClient::attestation_data_dir(),
        );
        if let Err(e) = broker.register_workload(&request.spec) {
            tracing::warn!(workload = %name, error = %e, "failed to register attest-gated secrets");
        }
        tracing::info!(
            workload = %name,
            "Confidential workload deployed; attestation gate evaluated"
        );
    }

    let mut state = app_state.state.write().await;
    state.upsert(
        name.clone(),
        WorkloadState::new(name.clone(), runtime_kind, instance, PathBuf::from("api_created")),
    );

    if let Err(e) = persist_workload_api(app_state, &state).await {
        return Err(err_internal(e));
    }

    emit_sse(
        app_state,
        &ServerEvent::WorkloadChanged {
            name: name.clone(),
            action: "created".to_string(),
        },
    );

    Ok(format!("Workload {} created", name))
}

/// POST /api/workloads - Create and deploy a workload
pub(crate) async fn create_workload(
    AxumState(app_state): AxumState<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let request = match parse_create_workload_payload::<String>(payload) {
        Ok(request) => request,
        Err(response) => return response,
    };

    match deploy_workload_spec(&app_state, request).await {
        Ok(msg) => created_json(msg),
        Err(resp) => resp,
    }
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
            if let Err(e) = persist_workload_api(&app_state, &state).await {
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

    if let Err(e) = persist_workload_api(&app_state, &state).await {
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

/// Best-effort stop for compose-down and batch operations (returns error message on failure).
pub(crate) async fn stop_workload_if_exists(
    app_state: &AppState,
    name: &str,
) -> Result<(), String> {
    let workload = match lookup_workload::<String>(app_state, name).await {
        Ok(w) => w,
        Err((_, json)) => {
            let msg = json.0.error.unwrap_or_else(|| "workload not found".into());
            if msg.to_lowercase().contains("not found") {
                return Ok(());
            }
            return Err(msg);
        }
    };

    let rt = match make_runtime::<String>(&workload.runtime).await {
        Ok(r) => r,
        Err((_, json)) => return Err(json.0.error.unwrap_or_else(|| "runtime error".into())),
    };

    rt.stop(&workload.instance)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
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

/// PUT/PATCH /api/workloads/:name — replace workload spec on disk and redeploy.
pub(crate) async fn update_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = validate_api_name::<String>(&name) {
        return e;
    }

    let spec = match parse_workload_payload::<String>(payload) {
        Ok(s) => s,
        Err(err) => return err,
    };
    if spec.metadata.name != name {
        return err_bad_request::<String>(format!(
            "spec.metadata.name ('{}') must match URL name ('{}')",
            spec.metadata.name, name
        ));
    }

    if let Some(resp) = opa_enforce_workload::<String>(&spec, &name).await {
        return resp;
    }

    let workload_state = match lookup_workload::<String>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e,
    };

    let mut spec_path = workload_state.spec_path.clone();
    if spec_path.as_path() == std::path::Path::new("api_created") || Workload::from_file(&spec_path).is_err() {
        spec_path = workload_spec_file(&name);
        if let Some(parent) = spec_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return err_internal::<String>(format!("create specs dir: {}", e));
            }
        }
    }

    if let Err(e) = validate_spec_path::<String>(&spec_path) {
        return e;
    }

    let yaml = match serde_yaml::to_string(&spec) {
        Ok(y) => y,
        Err(e) => return err_internal::<String>(e),
    };
    if let Err(e) = std::fs::write(&spec_path, yaml) {
        return err_internal::<String>(e);
    }

    let rt = match make_runtime::<String>(&workload_state.runtime).await {
        Ok(r) => r,
        Err(e) => return e,
    };
    let _ = rt.stop(&workload_state.instance).await;

    let image = match rt.build(&spec).await {
        Ok(img) => img,
        Err(e) => return err_internal::<String>(e),
    };
    let instance = match rt.run(&image, &spec).await {
        Ok(inst) => inst,
        Err(e) => return err_internal::<String>(e),
    };

    let mut state = app_state.state.write().await;
    state.upsert(
        name.clone(),
        WorkloadState {
            name: name.clone(),
            runtime: workload_state.runtime,
            instance,
            spec_path,
            created_at: workload_state.created_at,
            updated_at: crate::resources::now_rfc3339(),
            os_version: workload_state.os_version,
            node_labels: workload_state.node_labels,
        },
    );
    if let Err(e) = persist_workload_api(&app_state, &state).await {
        return err_internal::<String>(e);
    }

    emit_sse(&app_state, &ServerEvent::WorkloadChanged {
        name: name.clone(),
        action: "updated".to_string(),
    });

    record_audit_event(
        crate::audit::AuditAction::ConfigChange,
        &name,
        None,
        crate::audit::ActionResult::Success,
        "workload updated via API",
        None,
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(format!("Workload {} updated", name))),
    )
}

/// POST /api/workloads/:name/restart — stop then start.
pub(crate) async fn restart_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let stop_resp = stop_workload(AxumState(app_state.clone()), Path(name.clone())).await.into_response();
    if !stop_resp.status().is_success() {
        return stop_resp;
    }
    start_workload(AxumState(app_state), Path(name)).await.into_response()
}

/// POST /api/cost - Estimate costs for a workload
pub(crate) async fn estimate_cost(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let spec = match parse_workload_payload::<Vec<cost::CostEstimate>>(payload) {
        Ok(spec) => spec,
        Err(error) => return error,
    };

    match cost::estimate_all_providers(&spec) {
        Ok(estimates) => ok_json(estimates),
        Err(e) => err_internal::<Vec<cost::CostEstimate>>(e),
    }
}

/// GET /api/cost/pricing — Active pricing source, region, and purchase-model discounts.
pub(crate) async fn api_cost_pricing() -> impl IntoResponse {
    ok_json(cost::pricing_config())
}

/// GET /api/cost/chargeback — Fleet showback/chargeback by owner and project.
pub(crate) async fn api_cost_chargeback(
    AxumState(app_state): AxumState<AppState>,
    Query(query): Query<CostChargebackQuery>,
) -> impl IntoResponse {
    let provider = query
        .provider
        .as_deref()
        .unwrap_or("aws")
        .parse::<cost::CloudProvider>()
        .unwrap_or(cost::CloudProvider::AWS);
    let rows: Vec<(String, PathBuf)> = {
        let state = app_state.state.read().await;
        state
            .list()
            .iter()
            .map(|ws| (ws.name.clone(), ws.spec_path.clone()))
            .collect()
    };
    let refs: Vec<(&str, &PathBuf)> = rows
        .iter()
        .map(|(n, p)| (n.as_str(), p))
        .collect();
    match cost::chargeback_report(&refs, provider) {
        Ok(report) => ok_json(report),
        Err(e) => err_internal::<cost::ChargebackReport>(e),
    }
}

/// GET /api/backups - List all backups
pub(crate) async fn list_backups(Query(page): Query<PaginationQuery>) -> impl IntoResponse {
    let manager = backup::BackupManager::new(backup::BackupManager::default_dir());

    match manager.list_backups() {
        Ok(mut backups) => {
            let total = backups.len();
            let use_page = page.limit.is_some() || page.offset.is_some();
            if use_page {
                let (lim, off) = clamp_page(page.limit, page.offset, 50, 500);
                backups = backups.into_iter().skip(off).take(lim).collect();
            }
            if use_page {
                let json = match serde_json::to_string(&ApiResponse::success(backups)) {
                    Ok(s) => s,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<Vec<PathBuf>>::error(e.to_string())),
                        )
                            .into_response();
                    }
                };
                let mut res = Response::new(Body::from(json));
                *res.status_mut() = StatusCode::OK;
                if let Ok(v) = HeaderValue::from_str(&total.to_string()) {
                    res.headers_mut().insert("x-total-count", v);
                }
                res.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                res.into_response()
            } else {
                Json(ApiResponse::success(backups)).into_response()
            }
        }
        Err(e) => err_internal::<Vec<PathBuf>>(e).into_response(),
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
        Ok(path) => {
            let upload_path = path.clone();
            tokio::spawn(async move {
                crate::backup_remote::upload_backup_file_if_configured(&upload_path).await;
            });
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(format!("Backup created: {:?}", path))),
            )
        }
        Err(e) => err_internal::<String>(e),
    }
}

/// POST /api/ai/recommend - AI-powered runtime recommendation
pub(crate) async fn ai_recommend(
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    use crate::ai::scoring::ScoringEngine;
    use crate::config::Config;

    let explain = payload
        .get("explain")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let spec = match parse_workload_payload::<serde_json::Value>(payload) {
        Ok(spec) => spec,
        Err(error) => return error,
    };

    let config = Config::load();
    let intel = crate::intelligence::store::IntelligenceStore::load(
        &crate::intelligence::store::IntelligenceStore::default_path(),
    )
    .unwrap_or_default();
    let engine = ScoringEngine::new(config.engine).with_history(intel.runtime_history_map());
    let result = engine.score(&spec);

    if explain {
        let value = serde_json::json!({
            "recommended": format!("{}", result.recommended),
            "confidence": result.confidence,
            "workload_class": format!("{:?}", result.workload_class),
            "explain": true,
            "scores": result.scores.iter().map(|s| serde_json::json!({
                "runtime": format!("{}", s.runtime),
                "total_score": s.total_score,
                "cost_score": s.cost_score,
                "performance_score": s.performance_score,
                "reliability_score": s.reliability_score,
                "availability_score": s.availability_score,
                "reasons": s.reasons,
                "warnings": s.warnings,
            })).collect::<Vec<_>>(),
        });
        return ok_json(value);
    }

    match serde_json::to_value(result) {
        Ok(value) => ok_json(value),
        Err(e) => err_internal(format!("Failed to serialize scoring result: {}", e)),
    }
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
    use crate::ai::scaling::ScalingEngine;
    use crate::intelligence::metrics::fetch_fleet_utilization_series;

    let config = Config::load();
    let engine = ScalingEngine::new(config.scaling);

    let (cpu_series, mem_series) = fetch_fleet_utilization_series().await;

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

fn intent_goal_label(goal: &crate::spec::IntentGoal) -> &'static str {
    match goal {
        crate::spec::IntentGoal::LowLatency => "low-latency",
        crate::spec::IntentGoal::HighThroughput => "high-throughput",
        crate::spec::IntentGoal::CostOptimized => "cost-optimized",
        crate::spec::IntentGoal::Balanced => "balanced",
    }
}

async fn workload_spec_for_ai_request(
    app_state: &AppState,
    payload: serde_json::Value,
) -> Result<Workload, (StatusCode, Json<ApiResponse<serde_json::Value>>)> {
    if let Some(name) = payload.get("workload").and_then(|v| v.as_str()) {
        let workload_state = lookup_workload::<serde_json::Value>(app_state, name).await?;
        return load_spec_safe::<serde_json::Value>(&workload_state.spec_path);
    }
    parse_workload_payload(payload)
}

/// POST /api/ai/intent-optimize — Recommend intent goal from scoring across goals.
pub(crate) async fn ai_intent_optimize(
    AxumState(app_state): AxumState<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    use crate::ai::scoring::ScoringEngine;
    use crate::config::Config;
    use crate::spec::{IntentGoal, IntentSpec};

    let spec = match workload_spec_for_ai_request(&app_state, payload).await {
        Ok(spec) => spec,
        Err(error) => return error,
    };

    let current_intent = spec
        .intent
        .as_ref()
        .map(|i| intent_goal_label(&i.goal).to_string());

    let config = Config::load();
    let engine = ScoringEngine::new(config.engine);

    let goals = [
        IntentGoal::Balanced,
        IntentGoal::LowLatency,
        IntentGoal::HighThroughput,
        IntentGoal::CostOptimized,
    ];

    let mut best_goal = IntentGoal::Balanced;
    let mut best_score = 0.0_f64;
    let mut scores_by_goal = Vec::new();

    for goal in goals {
        let mut trial = spec.clone();
        match &mut trial.intent {
            Some(intent) => intent.goal = goal.clone(),
            None => {
                trial.intent = Some(IntentSpec {
                    goal: goal.clone(),
                    sla: None,
                    budget: None,
                    resilience: None,
                    compliance: None,
                    trust: None,
                });
            }
        }
        let result = engine.score(&trial);
        let top = result
            .scores
            .first()
            .map(|s| s.total_score)
            .unwrap_or(0.0);
        scores_by_goal.push(json!({
            "goal": intent_goal_label(&goal),
            "top_runtime_score": top,
            "recommended_runtime": format!("{}", result.recommended),
        }));
        if top > best_score {
            best_score = top;
            best_goal = goal.clone();
        }
    }

    let baseline = engine.score(&spec);
    let reason = if current_intent.as_deref() == Some(intent_goal_label(&best_goal)) {
        format!(
            "Current intent '{}' already aligns with the best scoring profile (top score {:.0}%).",
            intent_goal_label(&best_goal),
            best_score * 100.0
        )
    } else {
        format!(
            "Switching intent to '{}' improves the top runtime score from {:.0}% to {:.0}% (recommended runtime: {}).",
            intent_goal_label(&best_goal),
            baseline.scores.first().map(|s| s.total_score * 100.0).unwrap_or(0.0),
            best_score * 100.0,
            baseline.recommended
        )
    };

    ok_json(json!({
        "recommended_intent": intent_goal_label(&best_goal),
        "reason": reason,
        "current_intent": current_intent,
        "scores_by_goal": scores_by_goal,
    }))
}

/// POST /api/ai/right-size — Right-sizing suggestions from workload profiler.
pub(crate) async fn ai_right_size(
    AxumState(app_state): AxumState<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    use crate::ai::profiler::{Profiler, RecommendationCategory};
    use crate::config::Config;

    let name = payload
        .get("workload")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let (spec, runtime) = if let Some(name) = name {
        let workload_state = match lookup_workload::<serde_json::Value>(&app_state, &name).await {
            Ok(w) => w,
            Err(e) => return e,
        };
        let spec = match load_spec_safe::<serde_json::Value>(&workload_state.spec_path) {
            Ok(s) => s,
            Err(e) => return e,
        };
        (spec, Some(workload_state.runtime))
    } else {
        let spec = match parse_workload_payload::<serde_json::Value>(payload) {
            Ok(s) => s,
            Err(e) => return e,
        };
        (spec, None)
    };

    let config = Config::load();
    let profiler = Profiler::new(config.profiler.waste_threshold);
    let profile = profiler.profile(&spec, runtime);

    let right_sizing = profile
        .recommendations
        .iter()
        .find(|r| r.category == RecommendationCategory::RightSizing);

    let (suggestion, savings) = if let Some(rec) = right_sizing {
        (
            rec.description.clone(),
            format!("{:.0}%", rec.estimated_savings_pct),
        )
    } else if profile.resource_analysis.waste_detected {
        (
            "Resources appear over-provisioned; review CPU and memory requests.".to_string(),
            format!(
                "{:.0}%",
                (1.0 - profile.resource_analysis.overall_efficiency) * 100.0
            ),
        )
    } else {
        (
            "Current resource requests look well matched.".to_string(),
            "0%".to_string(),
        )
    };

    ok_json(json!({
        "suggestion": suggestion,
        "savings": savings,
        "optimization_score": profile.optimization_score,
        "overall_efficiency": profile.resource_analysis.overall_efficiency,
    }))
}

/// POST /api/ai/tradeoff — Compare top two runtimes (cost vs performance tradeoff).
pub(crate) async fn ai_tradeoff(
    AxumState(app_state): AxumState<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    use crate::ai::scoring::ScoringEngine;
    use crate::config::Config;

    let spec = match workload_spec_for_ai_request(&app_state, payload).await {
        Ok(spec) => spec,
        Err(error) => return error,
    };

    let config = Config::load();
    let engine = ScoringEngine::new(config.engine);
    let result = engine.score(&spec);
    let mut sorted = result.scores.clone();
    sorted.sort_by(|a, b| {
        b.total_score
            .partial_cmp(&a.total_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let best = sorted.first();
    let runner_up = sorted.get(1);
    let best_runtime = best.map(|s| format!("{}", s.runtime)).unwrap_or_default();
    let score = best.map(|s| (s.total_score * 100.0).round() as u32).unwrap_or(0);

    let summary = match (best, runner_up) {
        (Some(a), Some(b)) => {
            let cost_delta = (a.cost_score - b.cost_score) * 100.0;
            let perf_delta = (a.performance_score - b.performance_score) * 100.0;
            format!(
                "{} leads overall ({:.0}% vs {:.0}%). Cost edge: {:+.0} pts; performance edge: {:+.0} pts vs {}.",
                a.runtime,
                a.total_score * 100.0,
                b.total_score * 100.0,
                cost_delta,
                perf_delta,
                b.runtime
            )
        }
        (Some(a), None) => format!("{} is the only viable runtime ({:.0}% score).", a.runtime, a.total_score * 100.0),
        _ => "No runtime candidates scored.".to_string(),
    };

    ok_json(json!({
        "best_runtime": best_runtime,
        "score": score,
        "runner_up_runtime": runner_up.map(|s| format!("{}", s.runtime)),
        "runner_up_score": runner_up.map(|s| (s.total_score * 100.0).round() as u32),
        "summary": summary,
        "scores": sorted.iter().take(4).map(|s| json!({
            "runtime": format!("{}", s.runtime),
            "total_score": s.total_score,
            "cost_score": s.cost_score,
            "performance_score": s.performance_score,
        })).collect::<Vec<_>>(),
    }))
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

/// POST /api/drift/:name/reconcile — Execute drift reconciliation actions.
pub(crate) async fn api_drift_reconcile(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::drift::{execute_reconciliation, DriftDetector};
    use crate::events::{EventBus, EventCategory, EventSeverity};

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

    if !report.has_drift {
        return ok_json(json!({
            "workload_name": name,
            "reconciled": false,
            "message": "No drift detected",
            "results": [],
        }));
    }

    let mut store = app_state.state.write().await;

    let results = match execute_reconciliation(&report, &mut store).await {
        Ok(r) => r,
        Err(e) => return err_internal::<serde_json::Value>(e.to_string()),
    };

    if let Err(e) = persist_workload_api(&app_state, &store).await {
        return err_internal::<serde_json::Value>(e.to_string());
    }

    let path = EventBus::default_path();
    if let Ok(mut bus) = EventBus::load(&path) {
        bus.emit_simple(
            EventSeverity::Info,
            EventCategory::DriftDetected,
            "api",
            Some(&name),
            "Drift reconciled",
            &format!("{} action(s) executed", results.len()),
        );
        let _ = bus.save(&path);
    }

    let value = json!({
        "workload_name": name,
        "reconciled": true,
        "results": results,
    });
    ok_json(value)
}

/// GET /api/alerts/status — Notification channels and alert rules.
pub(crate) async fn api_alerts_status() -> impl IntoResponse {
    use crate::events::{ChannelType, EventBus};

    let path = EventBus::default_path();
    let bus = match EventBus::load(&path) {
        Ok(b) => b,
        Err(e) => return err_internal::<serde_json::Value>(e),
    };

    let channels: Vec<serde_json::Value> = bus
        .channels()
        .iter()
        .map(|ch| {
            let channel_type = match &ch.channel_type {
                ChannelType::Console => "console",
                ChannelType::File { .. } => "file",
                ChannelType::Webhook { .. } => "webhook",
                ChannelType::Slack { .. } => "slack",
                ChannelType::PagerDuty { .. } => "pagerduty",
                ChannelType::Email { .. } => "email",
                ChannelType::Teams { .. } => "teams",
            };
            json!({
                "name": ch.name,
                "enabled": ch.enabled,
                "channel_type": channel_type,
                "min_severity": format!("{}", ch.min_severity),
                "categories": ch.categories.iter().map(|c| format!("{}", c)).collect::<Vec<_>>(),
            })
        })
        .collect();

    let rules: Vec<serde_json::Value> = bus
        .rules()
        .iter()
        .map(|rule| {
            json!({
                "name": rule.name,
                "enabled": rule.enabled,
                "condition": format!("{}", rule.condition),
                "severity": format!("{}", rule.severity),
                "message_template": rule.message_template,
                "cooldown_seconds": rule.cooldown_seconds,
                "last_triggered": rule.last_triggered,
            })
        })
        .collect();

    ok_json(json!({
        "channels": channels,
        "rules": rules,
    }))
}

/// POST /api/policy/opa — Evaluate manifest or workload YAML against OPA (`AETHER_OPA_URL`).
pub(crate) async fn api_policy_opa(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let input = if let Some(manifest) = payload.get("manifest") {
        crate::opa::evaluate_manifest_optional(manifest).await
    } else {
        let spec = match parse_workload_payload::<serde_json::Value>(payload) {
            Ok(spec) => spec,
            Err(error) => return error,
        };
        crate::opa::evaluate_workload_optional(&spec).await
    };
    match input {
        Ok(ev) => match serde_json::to_value(ev) {
            Ok(v) => ok_json(v),
            Err(e) => err_internal::<serde_json::Value>(e),
        },
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// POST /api/policy/check - Check workload against policies
pub(crate) async fn api_policy_check(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    use crate::policy::PolicyEngine;

    let policy_set = payload
        .get("policy_set")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let spec = match parse_workload_payload::<serde_json::Value>(payload) {
        Ok(spec) => spec,
        Err(error) => return error,
    };

    let engine = match policy_set.as_deref() {
        Some("development") => PolicyEngine::development(),
        _ => PolicyEngine::production(),
    };

    let result = engine.evaluate(&spec);

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
            let nodes = graph.node_names();
            let edges: Vec<serde_json::Value> = graph
                .dependency_edges()
                .into_iter()
                .map(|(from, to)| serde_json::json!({ "from": from, "to": to }))
                .collect();
            let response = serde_json::json!({
                "stats": stats,
                "startup_order": order,
                "nodes": nodes,
                "edges": edges,
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

/// DELETE /api/dependencies — Remove a dependency edge (same JSON body as POST add).
pub(crate) async fn api_deps_remove(Json(request): Json<AddDependencyRequest>) -> impl IntoResponse {
    use crate::dependencies::DependencyGraph;

    let graph_path = DependencyGraph::default_path();
    match DependencyGraph::load(&graph_path) {
        Ok(mut graph) => {
            graph.remove_dependency(&request.workload, &request.dependency);
            if let Err(e) = graph.save(&graph_path) {
                return err_internal::<String>(e);
            }
            (
                StatusCode::OK,
                Json(ApiResponse::success(format!(
                    "Dependency removed: {} -> {}",
                    request.workload, request.dependency
                ))),
            )
        }
        Err(e) => err_internal::<String>(e),
    }
}

/// GET /api/audit - List audit events
pub(crate) async fn api_audit_list(Query(query): Query<AuditListQuery>) -> impl IntoResponse {
    use crate::audit::AuditLog;

    let audit_path = AuditLog::default_path();
    match AuditLog::load(&audit_path) {
        Ok(log) => {
            let summary = log.summary();
            let mut all: Vec<_> = log.events().iter().rev().cloned().collect();
            if let Some(ref filter) = query.workload {
                let needle = filter.to_lowercase();
                if !needle.is_empty() {
                    all.retain(|e| e.workload.to_lowercase().contains(&needle));
                }
            }
            let total = all.len();
            let use_page = query.limit.is_some() || query.offset.is_some();
            let recent: Vec<_> = if use_page {
                let (lim, off) = clamp_page(query.limit, query.offset, 20, 500);
                all.into_iter().skip(off).take(lim).collect()
            } else {
                all.into_iter().take(20).collect()
            };
            let response = serde_json::json!({
                "summary": summary,
                "recent_events": recent,
            });
            if use_page || query.workload.is_some() {
                let json = match serde_json::to_string(&ApiResponse::success(response)) {
                    Ok(s) => s,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                        )
                            .into_response();
                    }
                };
                let mut res = Response::new(Body::from(json));
                *res.status_mut() = StatusCode::OK;
                if let Ok(v) = HeaderValue::from_str(&total.to_string()) {
                    res.headers_mut().insert("x-total-count", v);
                }
                res.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                res.into_response()
            } else {
                (StatusCode::OK, Json(ApiResponse::success(response))).into_response()
            }
        }
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
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

/// POST /api/secrets — Create a secret and optional key/value pairs.
pub(crate) async fn create_secret(Json(request): Json<CreateSecretRequest>) -> impl IntoResponse {
    if let Err(e) = validate_api_name::<String>(&request.name) {
        return e;
    }
    if request.keys.is_empty() {
        return err_bad_request::<String>("keys map must not be empty".to_string());
    }

    use crate::secrets::SecretStore;

    let path = SecretStore::default_path();
    let mut store = match SecretStore::load(&path) {
        Ok(s) => s,
        Err(e) => return err_internal::<String>(e),
    };

    if store.get_secret(&request.name).is_some() {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<String>::error(format!(
                "Secret '{}' already exists",
                request.name
            ))),
        );
    }

    let ns = request.namespace.as_deref().unwrap_or("default");
    store.create_secret(&request.name, ns);
    for (k, v) in &request.keys {
        if let Err(e) = store.set_with_actor(&request.name, k, v, "api") {
            return err_internal::<String>(e);
        }
    }

    if let Err(e) = store.save(&path) {
        return err_internal::<String>(e);
    }

    created_json(format!("Secret '{}' created", request.name))
}

/// GET /api/events - List recent events
pub(crate) async fn api_events_list(Query(query): Query<EventsListQuery>) -> impl IntoResponse {
    use crate::events::{EventBus, EventCategory};

    let path = EventBus::default_path();
    match EventBus::load(&path) {
        Ok(bus) => {
            let mut all: Vec<_> = bus.events().iter().rev().cloned().collect();
            if let Some(ref cat) = query.category {
                let parsed = match cat.to_ascii_lowercase().as_str() {
                    "drift" => Some(EventCategory::DriftDetected),
                    "intent" | "intent-violation" | "intentviolation" => {
                        Some(EventCategory::IntentViolation)
                    }
                    "policy" | "policy-violation" => Some(EventCategory::PolicyViolation),
                    "sla" => Some(EventCategory::SlaViolation),
                    "health" => Some(EventCategory::HealthCheck),
                    "migration" => Some(EventCategory::Migration),
                    "deploy" | "deployment" => Some(EventCategory::Deployment),
                    "alert" | "system" => Some(EventCategory::SystemAlert),
                    _ => None,
                };
                if let Some(category) = parsed {
                    all.retain(|e| e.category == category);
                }
            }
            if let Some(ref workload) = query.workload {
                let w = workload.to_ascii_lowercase();
                all.retain(|e| {
                    e.workload
                        .as_ref()
                        .map(|n| n.to_ascii_lowercase().contains(&w))
                        .unwrap_or(false)
                });
            }
            let total = all.len();
            let use_page = query.limit.is_some() || query.offset.is_some();
            let events: Vec<_> = if use_page {
                let (lim, off) = clamp_page(query.limit, query.offset, 50, 500);
                all.into_iter().skip(off).take(lim).collect()
            } else {
                all.into_iter().take(50).collect()
            };
            if use_page {
                let json = match serde_json::to_string(&ApiResponse::success(
                    match serde_json::to_value(events) {
                        Ok(v) => v,
                        Err(e) => {
                            return err_internal::<serde_json::Value>(format!(
                                "Serialization failed: {}",
                                e
                            ))
                            .into_response();
                        }
                    },
                )) {
                    Ok(s) => s,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<serde_json::Value>::error(e.to_string())),
                        )
                            .into_response();
                    }
                };
                let mut res = Response::new(Body::from(json));
                *res.status_mut() = StatusCode::OK;
                if let Ok(v) = HeaderValue::from_str(&total.to_string()) {
                    res.headers_mut().insert("x-total-count", v);
                }
                res.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                res.into_response()
            } else {
                (
                    StatusCode::OK,
                    Json(ApiResponse::success(
                        match serde_json::to_value(events) {
                            Ok(v) => v,
                            Err(e) => {
                                return err_internal::<serde_json::Value>(format!(
                                    "Serialization failed: {}",
                                    e
                                ))
                                .into_response();
                            }
                        },
                    )),
                )
                    .into_response()
            }
        }
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
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

/// GET /api/platform/recommendations — optional setup items for the Platform page (not inline banners).
pub(crate) async fn api_platform_recommendations(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let items = super::platform_recommendations::collect(&app_state).await;
    ok_json(serde_json::json!({ "items": items }))
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
        api_version: None,
        plural: None,
        namespaced: None,
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
        api_version: query.api_version,
        plural: query.plural,
        namespaced: query.namespaced,
    };

    match crate::kubecluster::workload_detail(&request).await {
        Ok(detail) => ok_json(detail),
        Err(error) => err_internal::<crate::kubecluster::ClusterResourceDetail>(error),
    }
}

/// GET /api/cluster/health - App-style health summary for a resource.
pub(crate) async fn api_cluster_health(
    Query(query): Query<ClusterHealthQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::health_summary(&ClusterLogsRequest {
        cluster: query.cluster,
        namespace: query.namespace,
        kind: query.kind,
        name: query.name,
        api_version: query.api_version,
        plural: query.plural,
        namespaced: query.namespaced,
    }).await {
        Ok(summary) => ok_json(summary),
        Err(error) => err_internal::<crate::kubecluster::ClusterHealthSummary>(error),
    }
}

/// GET /api/cluster/events - List cluster events related to a specific resource.
pub(crate) async fn api_cluster_events(
    Query(query): Query<ClusterEventsQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::related_events(
        &query.cluster,
        &query.namespace,
        &query.kind,
        &query.name,
    )
    .await
    {
        Ok(events) => ok_json(events),
        Err(error) => err_internal::<Vec<crate::kubecluster::ClusterRelatedEvent>>(error),
    }
}

/// POST /api/cluster/action - Execute a native Kubernetes workload action.
pub(crate) async fn api_cluster_action(
    Json(request): Json<ClusterActionRequestBody>,
) -> impl IntoResponse {
    let workload_ref = format!("{}:{}/{}:{}", request.cluster, request.namespace, request.kind, request.name);
    let audit_action = match request.action.as_str() {
        "start" | "resume" | "uncordon" => crate::audit::AuditAction::Start,
        "stop" | "suspend" | "cordon" | "drain" => crate::audit::AuditAction::Stop,
        "delete" => crate::audit::AuditAction::Delete,
        "scale" => crate::audit::AuditAction::Scale,
        _ => crate::audit::AuditAction::ConfigChange,
    };
    match crate::kubecluster::workload_action(&crate::kubecluster::ClusterActionRequest {
        cluster: request.cluster,
        namespace: request.namespace,
        kind: request.kind,
        name: request.name,
        action: request.action,
        replicas: request.replicas,
        api_version: request.api_version,
        plural: request.plural,
        namespaced: request.namespaced,
    }).await {
        Ok(message) => {
            record_audit_event(audit_action, &workload_ref, Some("kubernetes"), crate::audit::ActionResult::Success, &message, None);
            ok_json(message)
        }
        Err(error) => {
            let error_message = error.to_string();
            record_audit_event(audit_action, &workload_ref, Some("kubernetes"), crate::audit::ActionResult::Failure, &error_message, None);
            err_internal::<String>(error_message)
        }
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
        api_version: query.api_version,
        plural: query.plural,
        namespaced: query.namespaced,
    }).await {
        Ok(resources) => ok_json(resources),
        Err(error) => err_internal::<Vec<crate::kubecluster::ClusterResourceSummary>>(error),
    }
}

/// POST /api/cluster/apply - Apply an edited Kubernetes manifest back to the cluster.
pub(crate) async fn api_cluster_apply(
    Json(request): Json<ClusterApplyRequestBody>,
) -> impl IntoResponse {
    let workload_ref = format!("{}:{}/{}:{}", request.cluster, request.namespace, request.kind, request.manifest.get("metadata").and_then(|m| m.get("name")).and_then(|n| n.as_str()).unwrap_or("unknown"));
    if crate::opa::enforce_enabled() {
        match crate::opa::evaluate_manifest_optional(&request.manifest).await {
            Ok(ev) if !ev.allowed => {
                let msg = if ev.denials.is_empty() {
                    "OPA policy denied this manifest".to_string()
                } else {
                    format!("OPA denied: {}", ev.denials.join("; "))
                };
                record_audit_event(
                    crate::audit::AuditAction::PolicyCheck,
                    &workload_ref,
                    Some("kubernetes"),
                    crate::audit::ActionResult::Failure,
                    &msg,
                    None,
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::<String>::error(msg)),
                );
            }
            Err(e) => {
                return err_internal::<String>(format!("OPA evaluation failed: {e}"));
            }
            _ => {}
        }
    }
    match crate::kubecluster::apply_manifest(
        &request.cluster,
        &request.namespace,
        &request.kind,
        request.manifest,
        request.api_version.as_deref(),
        request.plural.as_deref(),
        request.namespaced,
    )
    .await
    {
        Ok(message) => {
            record_audit_event(crate::audit::AuditAction::ConfigChange, &workload_ref, Some("kubernetes"), crate::audit::ActionResult::Success, &message, None);
            ok_json(message)
        }
        Err(error) => {
            let error_message = error.to_string();
            record_audit_event(crate::audit::AuditAction::ConfigChange, &workload_ref, Some("kubernetes"), crate::audit::ActionResult::Failure, &error_message, None);
            err_internal::<String>(error_message)
        }
    }
}

/// GET /api/cluster/ws/exec - Interactive exec stream into a selected pod.
pub(crate) async fn api_cluster_exec_ws(
    ws: WebSocketUpgrade,
    Query(query): Query<ClusterExecQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(error) = handle_cluster_exec_socket(socket, query).await {
            tracing::warn!("cluster exec websocket ended: {}", error);
        }
    })
}

async fn handle_cluster_exec_socket(
    socket: WebSocket,
    query: ClusterExecQuery,
) -> anyhow::Result<()> {
    let mut command = build_kubectl_command(&query.cluster);
    command
        .arg("-n")
        .arg(&query.namespace)
        .arg("exec")
        .arg("-i")
        .arg(&query.pod);

    if let Some(container) = query.container.as_deref().filter(|value| !value.is_empty()) {
        command.arg("-c").arg(container);
    }

    command.arg("--");
    if let Some(shell) = query.command.as_deref().filter(|value| !value.is_empty()) {
        command.arg(shell);
    } else {
        command.arg("/bin/sh");
    }

    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().context("exec session missing stdin")?;
    let stdout = child.stdout.take().context("exec session missing stdout")?;
    let stderr = child.stderr.take().context("exec session missing stderr")?;

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (output_tx, mut output_rx) = mpsc::unbounded_channel::<String>();

    tokio::spawn(read_child_stream_to_channel(stdout, output_tx.clone()));
    tokio::spawn(read_child_stream_to_channel(stderr, output_tx.clone()));

    let writer = tokio::spawn(async move {
        while let Some(chunk) = output_rx.recv().await {
            if ws_sender.send(Message::Text(chunk)).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(Message::Text(input)) => {
                stdin.write_all(input.as_bytes()).await?;
                stdin.flush().await?;
            }
            Ok(Message::Binary(input)) => {
                stdin.write_all(&input).await?;
                stdin.flush().await?;
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            Err(error) => return Err(error.into()),
        }
    }

    let _ = child.kill().await;
    let _ = writer.await;
    Ok(())
}

/// GET /api/cluster/ws/watch - Stream resource-list updates over WebSocket.
pub(crate) async fn api_cluster_watch_ws(
    ws: WebSocketUpgrade,
    Query(query): Query<ClusterWatchQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(error) = handle_cluster_watch_socket(socket, query).await {
            tracing::warn!("cluster watch websocket ended: {}", error);
        }
    })
}

async fn handle_cluster_watch_socket(
    mut socket: WebSocket,
    query: ClusterWatchQuery,
) -> anyhow::Result<()> {
    let request = crate::kubecluster::ClusterBrowseRequest {
        cluster: query.cluster.clone(),
        namespace: query.namespace.clone(),
        kind: query.kind.clone(),
        api_version: query.api_version.clone(),
        plural: query.plural.clone(),
        namespaced: query.namespaced,
    };

    let initial = crate::kubecluster::browse_resources(&request).await?;
    socket
        .send(Message::Text(serde_json::to_string(&initial)?))
        .await?;

    let resource_name = if query.kind == "CustomResource" {
        match query.plural.as_deref() {
            Some(plural) if !plural.is_empty() => plural.to_string(),
            _ => {
                socket
                    .send(Message::Text(
                        json!({ "type": "error", "message": "watch is not supported without plural for CustomResource" })
                            .to_string(),
                    ))
                    .await?;
                return Ok(());
            }
        }
    } else if let Some(resource_name) = kubectl_resource_name(&query.kind) {
        resource_name.to_string()
    } else {
        socket
            .send(Message::Text(
                json!({ "type": "error", "message": format!("watch is not supported for {}", query.kind) })
                    .to_string(),
            ))
            .await?;
        return Ok(());
    };

    let mut command = build_kubectl_command(&query.cluster);
    command.arg("get").arg(&resource_name);
    if query.kind == "Namespace" || (query.kind == "CustomResource" && !query.namespaced.unwrap_or(true)) {
        command.arg("--watch-only").arg("-o").arg("name");
    } else if let Some(namespace) = query.namespace.as_deref().filter(|value| *value != "all") {
        command
            .arg("-n")
            .arg(namespace)
            .arg("--watch-only")
            .arg("-o")
            .arg("name");
    } else {
        command
            .arg("-A")
            .arg("--watch-only")
            .arg("-o")
            .arg("name");
    }

    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);

    let mut child = command.spawn()?;
    let stdout = child.stdout.take().context("watch session missing stdout")?;
    let mut lines = BufReader::new(stdout).lines();

    while let Some(_line) = lines.next_line().await? {
        let resources = crate::kubecluster::browse_resources(&request).await?;
        if socket
            .send(Message::Text(serde_json::to_string(&resources)?))
            .await
            .is_err()
        {
            break;
        }
    }

    let _ = child.kill().await;
    Ok(())
}

/// POST /api/cluster/port-forward - Start a pod port-forward session.
pub(crate) async fn api_cluster_port_forward_start(
    AxumState(app_state): AxumState<AppState>,
    Json(request): Json<ClusterPortForwardRequestBody>,
) -> impl IntoResponse {
    let target_kind = request
        .target_kind
        .clone()
        .or_else(|| request.pod.as_ref().map(|_| "Pod".to_string()))
        .unwrap_or_else(|| "Pod".to_string());
    let target_name = request
        .target_name
        .clone()
        .or_else(|| request.pod.clone());
    let Some(target_name) = target_name else {
        return err_bad_request::<ClusterPortForwardResponse>("target_name or pod is required");
    };
    let local_port = match allocate_local_port(request.local_port).await {
        Ok(port) => port,
        Err(error) => return err_internal::<ClusterPortForwardResponse>(error),
    };

    let mut command = build_kubectl_command(&request.cluster);
    command
        .arg("-n")
        .arg(&request.namespace)
        .arg("port-forward")
        .arg(format!("{}/{}", target_kind.to_lowercase(), target_name))
        .arg(format!("{}:{}", local_port, request.remote_port))
        .arg("--address")
        .arg("127.0.0.1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return err_internal::<ClusterPortForwardResponse>(error),
    };

    sleep(Duration::from_millis(700)).await;
    match child.try_wait() {
        Ok(Some(_)) => {
            return err_internal::<ClusterPortForwardResponse>("port-forward process exited immediately");
        }
        Ok(None) => {}
        Err(error) => return err_internal::<ClusterPortForwardResponse>(error),
    }

    let session_id = format!(
        "pf-{}-{}",
        chrono::Utc::now().timestamp_millis(),
        local_port
    );
    let response = ClusterPortForwardResponse {
        session_id: session_id.clone(),
        cluster: request.cluster.clone(),
        namespace: request.namespace.clone(),
        target_kind: target_kind.clone(),
        target_name: target_name.clone(),
        local_port,
        remote_port: request.remote_port,
        local_url: format!("http://127.0.0.1:{}", local_port),
    };

    app_state.port_forwards.lock().await.insert(
        session_id.clone(),
        PortForwardSession {
            id: session_id,
            cluster: request.cluster,
            namespace: request.namespace,
            pod: target_name,
            local_port,
            remote_port: request.remote_port,
            child,
        },
    );

    created_json(response)
}

/// POST /api/cluster/port-forward/stop - Stop an active port-forward session.
pub(crate) async fn api_cluster_port_forward_stop(
    AxumState(app_state): AxumState<AppState>,
    Json(request): Json<ClusterPortForwardStopRequestBody>,
) -> impl IntoResponse {
    let mut session = match app_state.port_forwards.lock().await.remove(&request.session_id) {
        Some(session) => session,
        None => return err_not_found::<String>(format!("port-forward session {} not found", request.session_id)),
    };

    let summary = format!(
        "stopped port-forward {} {} {}/{}:{} -> 127.0.0.1:{}",
        session.id,
        session.cluster,
        session.namespace,
        session.pod,
        session.remote_port,
        session.local_port
    );
    let _ = session.child.kill().await;
    ok_json(summary)
}

/// GET /api/cluster/top - Pod/container metrics via kubectl top.
pub(crate) async fn api_cluster_top(
    Query(query): Query<ClusterTopQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::top_metrics(&ClusterLogsRequest {
        cluster: query.cluster,
        namespace: query.namespace,
        kind: query.kind,
        name: query.name,
        api_version: None,
        plural: None,
        namespaced: None,
    }).await {
        Ok(metrics) => ok_json(metrics),
        Err(error) => err_internal::<Vec<crate::kubecluster::ClusterTopMetric>>(error),
    }
}

/// GET /api/cluster/metrics/summary - Aggregate pod metrics for a cluster or namespace.
pub(crate) async fn api_cluster_metrics_summary(
    Query(query): Query<ClusterMetricsSummaryQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::metrics_summary(&query.cluster, query.namespace.as_deref()).await {
        Ok(summary) => ok_json(ClusterMetricsSummaryResponse {
            scope: summary.scope,
            pod_count: summary.pod_count,
            total_cpu_millicores: summary.total_cpu_millicores,
            total_memory_mib: summary.total_memory_mib,
            pods: summary.pods,
        }),
        Err(error) => err_internal::<ClusterMetricsSummaryResponse>(error),
    }
}

/// GET /api/cluster/cilium/status - CNI detection and Aether bootstrap policy inventory.
pub(crate) async fn api_cluster_cilium_status(
    Query(query): Query<ClusterCiliumStatusQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::cilium::cilium_status(query.cluster.as_deref(), query.namespace.as_deref()).await {
        Ok(status) => ok_json(status),
        Err(error) => err_internal::<crate::kubecluster::cilium::CiliumStatusResponse>(error),
    }
}

/// GET /api/observability/summary - Aether metrics + optional cluster/Cilium context.
pub(crate) async fn api_observability_summary(
    Query(query): Query<ObservabilitySummaryQuery>,
) -> impl IntoResponse {
    let cluster_ref = query.cluster.as_deref();
    let namespace_ref = query.namespace.as_deref();
    let cluster_metrics = if let Some(cluster) = cluster_ref.filter(|c| !c.is_empty()) {
        crate::kubecluster::metrics_summary(cluster, namespace_ref).await.ok()
    } else {
        None
    };
    let cilium = crate::kubecluster::cilium::cilium_status(cluster_ref, namespace_ref)
        .await
        .ok();
    ok_json(crate::observability::build_summary(cluster_metrics, cilium))
}

/// GET /api/observability/prometheus/query - Whitelisted instant query proxy.
pub(crate) async fn api_observability_prometheus_query(
    Query(query): Query<PrometheusQueryParams>,
) -> impl IntoResponse {
    match crate::observability::prometheus_instant_query(&query.query).await {
        Ok(value) => ok_json(value),
        Err(error) => err_internal::<serde_json::Value>(error),
    }
}

/// POST /api/cluster/diff - Server-side diff preview for an edited manifest.
pub(crate) async fn api_cluster_diff(
    Json(request): Json<ClusterDiffRequestBody>,
) -> impl IntoResponse {
    match crate::kubecluster::manifest_diff(
        &request.cluster,
        &request.namespace,
        &request.kind,
        &request.name,
        request.draft_manifest,
        request.api_version.as_deref(),
        request.plural.as_deref(),
        request.namespaced,
    )
    .await
    {
        Ok(lines) => ok_json(
            lines
                .into_iter()
                .map(|(kind, text)| ClusterDiffLine { kind, text })
                .collect::<Vec<_>>(),
        ),
        Err(error) => err_internal::<Vec<ClusterDiffLine>>(error),
    }
}

/// GET /api/cluster/rollout - Get rollout status and revision history.
pub(crate) async fn api_cluster_rollout(
    Query(query): Query<ClusterRolloutQuery>,
) -> impl IntoResponse {
    let Some(resource) = rollout_resource(&query.kind) else {
        return err_bad_request::<ClusterRolloutResponse>(format!("rollout is not supported for {}", query.kind));
    };

    let status = match run_kubectl(vec![
        "--context".to_string(),
        query.cluster.clone(),
        "-n".to_string(),
        query.namespace.clone(),
        "rollout".to_string(),
        "status".to_string(),
        format!("{}/{}", resource, query.name),
        "--timeout=5s".to_string(),
    ])
    .await
    {
        Ok(output) => output.trim().to_string(),
        Err(error) => error.to_string(),
    };

    let history_raw = match run_kubectl(vec![
        "--context".to_string(),
        query.cluster.clone(),
        "-n".to_string(),
        query.namespace.clone(),
        "rollout".to_string(),
        "history".to_string(),
        format!("{}/{}", resource, query.name),
    ])
    .await
    {
        Ok(output) => output,
        Err(error) => return err_internal::<ClusterRolloutResponse>(error),
    };

    let history = history_raw
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("REVISION"))
        .skip(1)
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let mut parts = trimmed.split_whitespace();
            let revision = parts.next()?.to_string();
            let change_cause = parts.collect::<Vec<_>>().join(" ");
            Some(ClusterRolloutRevision {
                revision,
                change_cause: if change_cause.is_empty() {
                    "none recorded".to_string()
                } else {
                    change_cause
                },
            })
        })
        .collect::<Vec<_>>();

    ok_json(ClusterRolloutResponse {
        cluster: query.cluster,
        namespace: query.namespace,
        kind: query.kind,
        name: query.name,
        status,
        history,
    })
}

/// POST /api/cluster/rollout/action - Execute rollout pause/resume/undo/restart.
pub(crate) async fn api_cluster_rollout_action(
    Json(request): Json<ClusterRolloutActionRequestBody>,
) -> impl IntoResponse {
    let Some(resource) = rollout_resource(&request.kind) else {
        return err_bad_request::<String>(format!("rollout is not supported for {}", request.kind));
    };

    let mut args = vec![
        "--context".to_string(),
        request.cluster.clone(),
        "-n".to_string(),
        request.namespace.clone(),
    ];

    match request.action.as_str() {
        "pause" | "resume" => {
            args.extend([
                "rollout".to_string(),
                request.action.clone(),
                format!("{}/{}", resource, request.name),
            ]);
        }
        "undo" => {
            args.extend([
                "rollout".to_string(),
                "undo".to_string(),
                format!("{}/{}", resource, request.name),
            ]);
            if let Some(revision) = request.revision.as_deref().filter(|value| !value.is_empty()) {
                args.push(format!("--to-revision={revision}"));
            }
        }
        "restart" => {
            args.extend([
                "rollout".to_string(),
                "restart".to_string(),
                format!("{}/{}", resource, request.name),
            ]);
        }
        other => return err_bad_request::<String>(format!("unsupported rollout action {}", other)),
    }

    let workload_ref = format!("{}:{}/{}:{}", request.cluster, request.namespace, request.kind, request.name);
    match run_kubectl(args).await {
        Ok(output) => {
            let message = output.trim().to_string();
            record_audit_event(crate::audit::AuditAction::ConfigChange, &workload_ref, Some("kubernetes"), crate::audit::ActionResult::Success, &message, None);
            ok_json(message)
        }
        Err(error) => {
            let error_message = error.to_string();
            record_audit_event(crate::audit::AuditAction::ConfigChange, &workload_ref, Some("kubernetes"), crate::audit::ActionResult::Failure, &error_message, None);
            err_internal::<String>(error_message)
        }
    }
}

/// GET /api/cluster/helm/history - Helm revision history for a release.
pub(crate) async fn api_cluster_helm_history(
    Query(query): Query<ClusterHelmHistoryQuery>,
) -> impl IntoResponse {
    match crate::kubecluster::helm_history(&query.cluster, &query.namespace, &query.release).await {
        Ok(history) => ok_json(history),
        Err(error) => err_internal::<Vec<crate::kubecluster::HelmRevisionEntry>>(error),
    }
}

/// POST /api/cluster/helm/action - Install, upgrade, or rollback a Helm release.
pub(crate) async fn api_cluster_helm_action(
    Json(request): Json<ClusterHelmActionRequestBody>,
) -> impl IntoResponse {
    let workload_ref = format!("{}:{}/HelmRelease:{}", request.cluster, request.namespace, request.release);
    match crate::kubecluster::helm_action(
        &request.cluster,
        &request.namespace,
        &request.release,
        &request.action,
        request.chart.as_deref(),
        request.values_yaml.as_deref(),
        request.revision.as_deref(),
    ).await {
        Ok(message) => {
            record_audit_event(crate::audit::AuditAction::ConfigChange, &workload_ref, Some("helm"), crate::audit::ActionResult::Success, &message, None);
            ok_json(message)
        }
        Err(error) => {
            let error_message = error.to_string();
            record_audit_event(crate::audit::AuditAction::ConfigChange, &workload_ref, Some("helm"), crate::audit::ActionResult::Failure, &error_message, None);
            err_internal::<String>(error_message)
        }
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
        Ok(mut scheduler) => {
            let affinity = crate::ai::affinity::AffinityEngine::load(
                &crate::ai::affinity::AffinityEngine::default_path(),
            )
            .unwrap_or_default();
            let recs = affinity.recommend(&crate::ai::affinity::WorkloadClass::WebService);
            let scores: std::collections::HashMap<_, _> = recs
                .into_iter()
                .map(|s| (s.runtime, s.composite_score))
                .collect();
            scheduler.set_affinity_scores(scores);
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

// POST /api/compose/up - Validate and deploy workloads in dependency order
pub(crate) async fn api_compose_up(
    AxumState(app_state): AxumState<AppState>,
    body: String,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    use crate::compose;

    let spec = match serde_yaml::from_str::<compose::ComposeSpec>(&body) {
        Ok(s) => s,
        Err(e) => return err_bad_request::<serde_json::Value>(format!("Invalid YAML: {}", e)),
    };

    if let Err(e) = compose::validate(&spec) {
        return err_bad_request::<serde_json::Value>(e.to_string());
    }

    let order = match compose::resolve_order(&spec) {
        Ok(o) => o,
        Err(e) => return err_bad_request::<serde_json::Value>(e.to_string()),
    };

    let mut deployed: Vec<String> = Vec::with_capacity(order.len());
    for name in order {
        let entry = spec.workloads.get(&name).expect("order key must exist");
        let workload = match entry.load_workload(None) {
            Ok(w) => w.with_env(&entry.env),
            Err(e) => {
                return err_bad_request::<serde_json::Value>(format!("{}: {}", name, e));
            }
        };

        let request = CreateWorkloadRequest {
            spec: workload,
            runtime: entry.runtime.clone(),
        };

        match deploy_workload_spec(&app_state, request).await {
            Ok(_) => deployed.push(name),
            Err(resp) => {
                let (status, json) = resp;
                return (
                    status,
                    Json(ApiResponse {
                        success: json.0.success,
                        data: json.0.data.map(|s| serde_json::Value::String(s)),
                        error: json.0.error,
                    }),
                );
            }
        }
    }

    ok_json(serde_json::json!({
        "deployed": deployed,
        "count": deployed.len(),
    }))
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

    let spec = match load_spec_safe::<String>(&workload_state.spec_path) {
        Ok(s) => s,
        Err(e) => return e,
    };

    let strategy = if request.auto_strategy {
        let config = Config::load();
        let advisor = crate::ai::migration::MigrationAdvisor::new(config.migration);
        let plan = advisor.plan_proposal(&spec, source_runtime, target_runtime);
        plan.advice.recommended_strategy
    } else {
        match request.strategy.parse::<MigrationStrategy>() {
            Ok(s) => s,
            Err(e) => {
                return err_bad_request::<String>(e)
            }
        }
    };

    let strategy_str = format!("{:?}", strategy);
    let plan = MigrationPlan::new(
        name.clone(),
        source_runtime,
        target_runtime,
        strategy,
        true,
    );

    let migration_start = std::time::Instant::now();
    let engine = MigrationEngine::new(app_state.state_path.clone());
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
        &strategy_str,
        duration,
        result.success,
        result.rollback_performed,
    );

    tracing::info!(
        workload = %name,
        source = %source_runtime,
        target = %target_runtime,
        strategy = %strategy_str,
        success = result.success,
        rollback = result.rollback_performed,
        duration_sec = duration,
        "migration completed"
    );

    if result.success {
        // Reload state after migration engine updated it
        let new_state = match StateStore::load(&app_state.state_path) {
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
        if let Err(e) = persist_workload_api(&app_state, &state).await {
            return err_internal::<String>(format!(
                "Migration persisted to disk but failed to sync shared state: {}",
                e
            ));
        }

        emit_sse(&app_state, &ServerEvent::WorkloadChanged {
            name: name.clone(),
            action: "migrated".to_string(),
        });

        let _ = crate::intelligence::record::record_migration_outcome(
            &name,
            &spec,
            source_runtime,
            target_runtime,
            true,
        );

        (
            StatusCode::OK,
            Json(ApiResponse::success(format!(
                "Workload {} migrated from {} to {} (strategy: {:?}, duration: {:.1}s)",
                name, source_runtime, target_runtime, strategy_str, duration
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
    // Try to parse the YAML as a Workload spec (v1 or legacy dashboard format)
    let workload_result = crate::legacy_workload_yaml::parse_workload_yaml(&request.yaml);

    match workload_result {
        Ok(workload) => {
            // YAML parsed successfully, now run validation
            match workload.validate() {
                Ok(()) => {
                    let mut errors = Vec::new();
                    if workload.confidential.as_ref().is_some_and(|c| c.enabled) {
                        if let Err(e) = crate::ragnarok::image::deploy_image_gate(
                            &workload,
                            &crate::ragnarok::image::ImageCatalog::load(
                                &crate::ragnarok::client::RagnarokClient::attestation_data_dir(),
                            ),
                        ) {
                            errors.push(e.to_string());
                        }
                        if let Err(e) = crate::ragnarok::isolation::deploy_isolation_gate(&workload) {
                            errors.push(e.to_string());
                        }
                        if let Err(e) = crate::ragnarok::sovereign::deploy_sovereign_gate(&workload) {
                            errors.push(e.to_string());
                        }
                        if let Err(e) = crate::ragnarok::kata::deploy_kata_gate(&workload) {
                            errors.push(e.to_string());
                        }
                    }
                    let response = ValidateResponse {
                        valid: errors.is_empty(),
                        workload_name: Some(workload.metadata.name),
                        errors,
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

fn load_gitops_controller_for_api() -> Result<(crate::gitops::GitOpsController, PathBuf), String> {
    let state_path = crate::resources::aether_path("gitops.json");
    if !state_path.exists() {
        return Err("GitOps not configured: run `aether git-ops init --repo <URL>` first".into());
    }
    let data = std::fs::read_to_string(&state_path).map_err(|e| e.to_string())?;
    if let Ok(config) = serde_json::from_str::<crate::gitops::GitOpsConfig>(&data) {
        if config.repo_url.is_empty() {
            return Err("gitops.json has empty repo_url".into());
        }
        return Ok((crate::gitops::GitOpsController::new(config), state_path));
    }
    if let Ok(status) = serde_json::from_str::<crate::gitops::GitOpsStatus>(&data) {
        if status.repo_url.is_empty() {
            return Err("gitops.json status has empty repo_url".into());
        }
        let config = crate::gitops::GitOpsConfig {
            repo_url: status.repo_url.clone(),
            branch: status.branch.clone(),
            ..Default::default()
        };
        return Ok((crate::gitops::GitOpsController::new(config), state_path));
    }
    Err("Could not parse ~/.aether/gitops.json".into())
}

/// GET /api/gitops/status — Raw `gitops.json` payload (config or last status).
pub(crate) async fn api_gitops_status() -> impl IntoResponse {
    let path = crate::resources::aether_path("gitops.json");
    if !path.exists() {
        return ok_json(serde_json::json!({
            "configured": false,
            "hint": "Run `aether git-ops init --repo <URL>`"
        }));
    }
    match std::fs::read_to_string(&path) {
        Ok(data) => match serde_json::from_str::<serde_json::Value>(&data) {
            Ok(v) => ok_json(v),
            Err(e) => err_internal::<serde_json::Value>(e),
        },
        Err(e) => err_internal::<serde_json::Value>(e),
    }
}

/// POST /api/gitops/preview — Pull repo and list pending YAML changes without updating sync status.
pub(crate) async fn api_gitops_preview() -> impl IntoResponse {
    match load_gitops_controller_for_api() {
        Ok((mut ctrl, _state_path)) => match ctrl.detect_changes() {
            Ok(changes) => {
                let confidential_compliance =
                    crate::gitops::audit_confidential_changes(&ctrl.repo_dir, &changes);
                ok_json(serde_json::json!({
                    "changes": changes,
                    "confidential_compliance": confidential_compliance,
                }))
            }
            Err(e) => err_internal::<serde_json::Value>(e),
        },
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(msg)),
        ),
    }
}

/// POST /api/gitops/sync — Pull repo and detect YAML changes (same as CLI sync).
pub(crate) async fn api_gitops_sync() -> impl IntoResponse {
    match load_gitops_controller_for_api() {
        Ok((mut ctrl, state_path)) => match ctrl.sync() {
            Ok(changes) => {
                let st = ctrl.status().clone();
                let confidential_compliance =
                    crate::gitops::audit_confidential_changes(&ctrl.repo_dir, &changes);
                let status_json = match serde_json::to_string_pretty(&st) {
                    Ok(s) => s,
                    Err(e) => return err_internal::<serde_json::Value>(e.to_string()),
                };
                if let Err(e) = std::fs::write(&state_path, status_json) {
                    return err_internal::<serde_json::Value>(e.to_string());
                }
                ok_json(serde_json::json!({
                    "changes": changes,
                    "status": st,
                    "confidential_compliance": confidential_compliance,
                }))
            }
            Err(e) => err_internal::<serde_json::Value>(e),
        },
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(msg)),
        ),
    }
}

/// GET /api/dashboard/version — API and embedded UI build identifiers.
pub(crate) async fn api_dashboard_version() -> impl IntoResponse {
    ok_json(serde_json::json!({
        "api_version": env!("CARGO_PKG_VERSION"),
        "embedded_ui_build": env!("AETHER_EMBEDDED_UI_BUILD"),
    }))
}

/// POST /api/webhooks/channels — Register a webhook notification channel.
pub(crate) async fn api_webhook_channel_create(
    Json(req): Json<WebhookChannelCreateRequest>,
) -> impl IntoResponse {
    use crate::events::{ChannelType, EventBus, EventSeverity, NotificationChannel};

    let min_severity: EventSeverity = match req.severity.parse() {
        Ok(s) => s,
        Err(e) => return err_bad_request::<String>(e),
    };
    let path = EventBus::default_path();
    let mut bus = match EventBus::load(&path) {
        Ok(b) => b,
        Err(e) => return err_internal::<String>(e),
    };
    if bus.channels().iter().any(|c| c.name == req.name) {
        return err_bad_request::<String>(format!("Channel '{}' already exists", req.name));
    }
    bus.add_channel(NotificationChannel {
        name: req.name.clone(),
        channel_type: ChannelType::Webhook {
            url: req.url.clone(),
            method: req.method.clone(),
        },
        enabled: true,
        min_severity,
        categories: vec![],
    });
    if let Err(e) = bus.save(&path) {
        return err_internal::<String>(e);
    }
    ok_json(format!("Channel '{}' created", req.name))
}

/// DELETE /api/webhooks/channels/:name — Remove a notification channel.
pub(crate) async fn api_webhook_channel_delete(Path(name): Path<String>) -> impl IntoResponse {
    use crate::events::EventBus;

    let path = EventBus::default_path();
    let mut bus = match EventBus::load(&path) {
        Ok(b) => b,
        Err(e) => return err_internal::<String>(e),
    };
    if bus.remove_channel(&name) {
        if let Err(e) = bus.save(&path) {
            return err_internal::<String>(e);
        }
        ok_json(format!("Channel '{}' removed", name))
    } else {
        err_not_found::<String>(format!("Channel '{}' not found", name))
    }
}

/// POST /api/webhooks/test — Queue a test notification for a channel (see `aether webhook test`).
pub(crate) async fn api_webhook_test(Json(req): Json<WebhookTestRequest>) -> impl IntoResponse {
    use crate::events::{EventBus, EventCategory, EventSeverity};

    let path = EventBus::default_path();
    let mut bus = match EventBus::load(&path) {
        Ok(b) => b,
        Err(e) => return err_internal::<String>(e),
    };
    if !bus.channels().iter().any(|c| c.name == req.channel) {
        return err_not_found::<String>(format!("Channel '{}' not found", req.channel));
    }
    bus.emit_simple(
        EventSeverity::Info,
        EventCategory::SystemAlert,
        "api",
        None,
        "Webhook Test",
        &format!("Test notification for channel '{}'", req.channel),
    );
    if let Err(e) = bus.save(&path) {
        return err_internal::<String>(e);
    }
    ok_json(format!("Test event emitted for '{}'", req.channel))
}

/// GET /api/server — Process capabilities (auth modes, persistence).
pub(crate) async fn api_server_info(AxumState(app_state): AxumState<AppState>) -> impl IntoResponse {
    let oidc_issuer = std::env::var("AETHER_OIDC_ISSUER").ok().filter(|s| !s.is_empty());
    let oidc_ready = std::env::var("AETHER_OIDC_ISSUER").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_OIDC_CLIENT_ID").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_OIDC_REDIRECT_URI").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_SESSION_SECRET").ok().filter(|s| !s.is_empty()).is_some();
    let redis_configured = app_state.shared_cache.uses_redis();
    let postgres_configured = app_state.workload_state_pg.is_some();
    let state_poll_secs = std::env::var("AETHER_STATE_POLL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0 && *n <= 3600)
        .unwrap_or(2);
    let workload_backend = if postgres_configured {
        "postgresql"
    } else {
        "local-json"
    };
    let ha_mode = if postgres_configured && redis_configured {
        "multi-node-full"
    } else if postgres_configured {
        "multi-node-state"
    } else if redis_configured {
        "multi-node-cache"
    } else {
        "single"
    };
    let mutation_confirm = std::env::var("AETHER_REQUIRE_MUTATION_CONFIRM")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let cilium_status = crate::kubecluster::cilium::cilium_status(None, None)
        .await
        .ok()
        .map(|s| serde_json::to_value(s).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);
    ok_json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "persistence": workload_backend,
        "workload_state": {
            "backend": workload_backend,
            "configured": postgres_configured,
            "poll_secs": state_poll_secs,
            "env": "AETHER_STATE_DATABASE_URL",
            "poll_env": "AETHER_STATE_POLL_SECS",
        },
        "ha_mode": ha_mode,
        "ha_shared_cache": redis_configured,
        "tls": app_state.tls_active,
        "mtls": false,
        "oidc": {
            "enabled": oidc_ready,
            "issuer_configured": oidc_issuer.is_some(),
            "issuer": oidc_issuer,
            "login_path": "/api/auth/oidc/login",
            "callback_path": "/api/auth/oidc/callback",
            "role_mapping_env": "AETHER_OIDC_ROLE_MAP",
            "groups_claim_env": "AETHER_OIDC_GROUPS_CLAIM",
            "role_mapping_configured": std::env::var("AETHER_OIDC_ROLE_MAP").ok().filter(|s| !s.is_empty()).is_some(),
        },
        "rate_limit": { "type": "global_concurrency", "max": 200 },
        "observability": {
            "prometheus_metrics_path": "/api/metrics",
            "request_id_header": "x-request-id",
            "api_http_metrics": "aether_api_http_requests_total",
        },
        "safety": {
            "mutation_confirm_required": mutation_confirm,
            "mutation_confirm_header": "X-Aether-Confirm",
            "mutation_confirm_values": ["1", "true", "yes", "delete"],
        },
        "durability": {
            "state_store": "~/.aether/state.json",
            "backup_api": "/api/backups",
            "recommended": "Schedule POST /api/backups on a timer; copy ~/.aether off-host for DR.",
        },
        "plugins": {
            "ipc_timeout_env": "AETHER_PLUGIN_IPC_TIMEOUT_SEC",
            "ipc_max_output_env": "AETHER_PLUGIN_IPC_MAX_OUTPUT_BYTES",
        },
        "opa": {
            "configured": crate::opa::configured(),
            "enforce": crate::opa::enforce_enabled(),
            "url_env": "AETHER_OPA_URL",
            "package_env": "AETHER_OPA_PACKAGE",
            "enforce_env": "AETHER_OPA_ENFORCE",
            "check_path": "/api/policy/opa",
            "enforced_paths": ["POST /api/workloads", "PUT /api/workloads/:name", "POST /api/cluster/apply"],
        },
        "cost": {
            "pricing_path": "/api/cost/pricing",
            "chargeback_path": "/api/cost/chargeback",
            "provider_env": "AETHER_COST_PROVIDER",
            "region_env": "AETHER_COST_REGION",
            "live_pricing_env": "AETHER_PRICING_URL",
        },
        "embedded_ui_build": env!("AETHER_EMBEDDED_UI_BUILD"),
        "integrations": {
            "backup_remote_configured": std::env::var("AETHER_BACKUP_REMOTE_URL").ok().filter(|s| !s.is_empty()).is_some(),
            "audit_webhook_configured": std::env::var("AETHER_AUDIT_WEBHOOK_URL").ok().filter(|s| !s.is_empty()).is_some(),
            "grafana_url": std::env::var("AETHER_GRAFANA_URL").ok().filter(|s| !s.is_empty()),
            "prometheus_url": std::env::var("AETHER_PROMETHEUS_URL").ok().filter(|s| !s.is_empty()),
            "hubble_ui_url": std::env::var("AETHER_HUBBLE_UI_URL").ok().filter(|s| !s.is_empty()),
            "grafana_dashboard_uid": std::env::var("AETHER_GRAFANA_DASHBOARD_UID").ok().filter(|s| !s.is_empty()),
            "packetwolf_url": std::env::var("AETHER_PACKETWOLF_URL").ok().filter(|s| !s.is_empty()),
        },
        "kubernetes": {
            "cilium": cilium_status,
        },
    }))
}

/// GET /api/auth/providers — Advertised authentication mechanisms.
pub(crate) async fn api_auth_providers() -> impl IntoResponse {
    let issuer = std::env::var("AETHER_OIDC_ISSUER").ok().filter(|s| !s.is_empty());
    let oidc_ready = issuer.is_some()
        && std::env::var("AETHER_OIDC_CLIENT_ID").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_OIDC_REDIRECT_URI").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_SESSION_SECRET").ok().filter(|s| !s.is_empty()).is_some();
    let role_map = std::env::var("AETHER_OIDC_ROLE_MAP").ok().filter(|s| !s.is_empty());
    let saml_idp = std::env::var("AETHER_SAML_IDP_SSO_URL").ok().filter(|s| !s.is_empty());
    let saml_ready = saml_idp.is_some()
        && std::env::var("AETHER_SAML_IDP_ENTITY_ID").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_SAML_ACS_URL").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_SESSION_SECRET").ok().filter(|s| !s.is_empty()).is_some();
    let saml_role_map = std::env::var("AETHER_SAML_ROLE_MAP").ok().filter(|s| !s.is_empty());
    let mut methods = vec!["bearer", "legacy_env"];
    if oidc_ready || saml_ready {
        methods.push("oidc_session_cookie");
    }
    ok_json(serde_json::json!({
        "methods": methods,
        "oidc": {
            "enabled": oidc_ready,
            "issuer": issuer,
            "authorization_code_pkce": oidc_ready,
            "login_url": "/api/auth/oidc/login",
            "logout_url": "/api/auth/oidc/logout",
            "role_mapping_configured": role_map.is_some(),
            "groups_claim_env": "AETHER_OIDC_GROUPS_CLAIM",
            "role_mapping_env": "AETHER_OIDC_ROLE_MAP",
            "note": "Dashboard: Sign in with OIDC or use bearer token. Map IdP groups via AETHER_OIDC_ROLE_MAP (admin=group1;operator=group2)."
        },
        "saml": {
            "enabled": saml_ready,
            "idp_sso_url": saml_idp,
            "login_url": "/api/auth/saml/login",
            "acs_url": "/api/auth/saml/acs",
            "logout_url": "/api/auth/saml/logout",
            "role_mapping_configured": saml_role_map.is_some(),
            "role_mapping_env": "AETHER_SAML_ROLE_MAP",
            "note": "Dashboard: Sign in with SAML when configured. Map IdP group attributes via AETHER_SAML_ROLE_MAP."
        },
    }))
}

/// GET /api/openapi.json — Minimal OpenAPI 3 document (curated list of notable routes).
pub(crate) async fn serve_openapi() -> impl IntoResponse {
    const DOC: &str = include_str!("openapi.json");
    ([(header::CONTENT_TYPE, "application/json")], DOC)
}

/// POST /api/audit/events — Append an audit row (admin only; integrity hash applied).
pub(crate) async fn api_audit_append(Json(req): Json<AuditAppendRequest>) -> impl IntoResponse {
    use crate::audit::AuditLog;

    let path = AuditLog::default_path();
    let mut log = AuditLog::load(&path).unwrap_or_default();
    let action = parse_audit_action_from_str(&req.action);
    let result = parse_action_result_from_str(&req.result);
    log.record(
        action,
        &req.workload,
        req.runtime.as_deref(),
        result,
        &req.message,
        req.details.as_deref(),
    );
    if let Err(e) = log.save(&path) {
        return err_internal::<String>(e);
    }
    ok_json("audit event recorded".to_string())
}

/// GET /api/backups/:name — Metadata for a backup file in the default backup directory.
pub(crate) async fn get_backup(Path(name): Path<String>) -> impl IntoResponse {
    let manager = backup::BackupManager::new(backup::BackupManager::default_dir());
    let dir = backup::BackupManager::default_dir();
    let path = match resolve_backup_path(&dir, &name) {
        Ok(p) => p,
        Err(msg) => return err_bad_request::<String>(msg).into_response(),
    };
    if !path.exists() {
        return err_not_found::<String>(format!("Backup not found: {}", name)).into_response();
    }
    match manager.get_backup_info(&path) {
        Ok(meta) => ok_json(meta).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}

/// DELETE /api/backups/:name — Remove a backup file from disk.
pub(crate) async fn delete_backup_api(Path(name): Path<String>) -> impl IntoResponse {
    let manager = backup::BackupManager::new(backup::BackupManager::default_dir());
    let dir = backup::BackupManager::default_dir();
    let path = match resolve_backup_path(&dir, &name) {
        Ok(p) => p,
        Err(msg) => return err_bad_request::<String>(msg),
    };
    match manager.delete_backup(&path) {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse::success(format!("Deleted backup {}", name))),
        ),
        Err(e) => err_internal::<String>(e),
    }
}

/// POST /api/backups/restore — Restore or merge workload state from a backup.
pub(crate) async fn restore_backup(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<RestoreBackupRequest>,
) -> impl IntoResponse {
    let dir = backup::BackupManager::default_dir();
    let path = match resolve_backup_path(&dir, &req.name) {
        Ok(p) => p,
        Err(msg) => return err_bad_request::<String>(msg),
    };
    let backup = match backup::Backup::load(&path) {
        Ok(b) => b,
        Err(e) => return err_internal::<String>(e),
    };
    let state_path = app_state.state_path.clone();
    let res = if req.merge.unwrap_or(false) {
        backup.merge(&state_path)
    } else {
        backup.restore(&state_path)
    };
    if let Err(e) = res {
        return err_internal::<String>(e);
    }
    let new_state = match StateStore::load(&state_path) {
        Ok(s) => s,
        Err(e) => {
            return err_internal::<String>(format!("Restored file but failed to reload state: {}", e));
        }
    };
    *app_state.state.write().await = new_state.clone();
    if let Err(e) = persist_workload_api(&app_state, &new_state).await {
        return err_internal::<String>(format!(
            "State restored to disk but failed to sync shared store: {}",
            e
        ));
    }
    record_audit_event(
        crate::audit::AuditAction::BackupRestore,
        "state",
        None,
        crate::audit::ActionResult::Success,
        "state restored via API",
        Some(req.name.as_str()),
    );
    ok_json(format!(
        "Backup '{}' {}",
        req.name,
        if req.merge.unwrap_or(false) {
            "merged"
        } else {
            "restored"
        }
    ))
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
