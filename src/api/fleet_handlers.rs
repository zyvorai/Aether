// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Fleet edge agent and federation API handlers.

use super::handlers::{err_bad_request, err_forbidden, err_internal, err_not_found, ok_json};
use super::types::AppState;
use crate::fleet::edge::{
    edge_token_ok, EdgeEnqueueRequest, EdgeHeartbeatRequest, EdgeRegisterRequest, EdgeStore,
};
use crate::fleet::federation::{self, FederationPlanRequest};
use crate::gitops::GitOpsConfig;
use crate::spec::Workload;
use axum::{
    extract::{Query, State as AxumState},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn edge_auth(headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    if std::env::var("AETHER_EDGE_TOKEN")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
    {
        let token = bearer_token(headers);
        if edge_token_ok(token.as_deref()) {
            Ok(())
        } else {
            Err((StatusCode::UNAUTHORIZED, "invalid edge token".into()))
        }
    } else {
        Ok(())
    }
}

pub(crate) async fn api_fleet_edge_register(
    headers: HeaderMap,
    Json(body): Json<EdgeRegisterRequest>,
) -> impl IntoResponse {
    if let Err((_, msg)) = edge_auth(&headers) {
        return err_forbidden(msg);
    }
    if body.site.trim().is_empty() {
        return err_bad_request("site is required");
    }
    let store = EdgeStore::load();
    match store.register(body) {
        Ok(rec) => ok_json(rec),
        Err(e) => err_internal(e),
    }
}

pub(crate) async fn api_fleet_edge_heartbeat(
    headers: HeaderMap,
    Json(body): Json<EdgeHeartbeatRequest>,
) -> impl IntoResponse {
    if let Err((_, msg)) = edge_auth(&headers) {
        return err_forbidden(msg);
    }
    let store = EdgeStore::load();
    match store.heartbeat(body) {
        Ok(rec) => ok_json(rec),
        Err(e) => err_not_found(e.to_string()),
    }
}

pub(crate) async fn api_fleet_edge_agents() -> impl IntoResponse {
    let store = EdgeStore::load();
    ok_json(store.list_agents())
}

pub(crate) async fn api_fleet_edge_enqueue(
    Json(body): Json<EdgeEnqueueRequest>,
) -> impl IntoResponse {
    if body.site.trim().is_empty() || body.action.trim().is_empty() {
        return err_bad_request("site and action are required");
    }
    let store = EdgeStore::load();
    match store.enqueue(body) {
        Ok(job) => ok_json(job),
        Err(e) => err_bad_request(e),
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct EdgeQueueQuery {
    pub site: String,
}

pub(crate) async fn api_fleet_edge_queue(
    headers: HeaderMap,
    Query(q): Query<EdgeQueueQuery>,
) -> impl IntoResponse {
    if let Err((_code, msg)) = edge_auth(&headers) {
        return err_forbidden(msg);
    }
    if q.site.trim().is_empty() {
        return err_bad_request("site query param required");
    }
    let store = EdgeStore::load();
    ok_json(store.poll_queue(&q.site))
}

pub(crate) async fn api_fleet_federation_policies() -> impl IntoResponse {
    ok_json(federation::federation_policies())
}

pub(crate) async fn api_fleet_federation_plan(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<FederationPlanRequest>,
) -> impl IntoResponse {
    let spec = if let Some(yaml) = body.workload_yaml.filter(|s| !s.trim().is_empty()) {
        match federation::parse_workload_yaml(&yaml) {
            Ok(s) => s,
            Err(e) => return err_bad_request(e),
        }
    } else if let Some(name) = body.workload_name.filter(|s| !s.trim().is_empty()) {
        let store = app_state.state.read().await;
        let ws = match store.get(&name) {
            Some(w) => w,
            None => return err_not_found(format!("workload '{name}' not found")),
        };
        let path = ws.spec_path.clone();
        drop(store);
        match Workload::from_file(&path) {
            Ok(s) => s,
            Err(e) => return err_bad_request(e),
        }
    } else {
        return err_bad_request("workload_yaml or workload_name required");
    };

    match federation::plan_placement(&spec).await {
        Ok(plan) => ok_json(plan),
        Err(e) => err_internal(e),
    }
}

pub(crate) async fn api_fleet_drift(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let store = app_state.state.read().await;
    ok_json(crate::drift::fleet::scan_fleet(&store))
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct GitOpsResolveBody {
    pub workload_name: Option<String>,
    pub workload_yaml: Option<String>,
    pub environment: Option<String>,
}

pub(crate) async fn api_gitops_resolve_target(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<GitOpsResolveBody>,
) -> impl IntoResponse {
    let spec = if let Some(yaml) = body.workload_yaml.filter(|s| !s.trim().is_empty()) {
        match federation::parse_workload_yaml(&yaml) {
            Ok(s) => s,
            Err(e) => return err_bad_request(e),
        }
    } else if let Some(name) = body.workload_name.filter(|s| !s.trim().is_empty()) {
        let store = app_state.state.read().await;
        let ws = match store.get(&name) {
            Some(w) => w,
            None => return err_not_found(format!("workload '{name}' not found")),
        };
        let path = ws.spec_path.clone();
        drop(store);
        match Workload::from_file(&path) {
            Ok(s) => s,
            Err(e) => return err_bad_request(e),
        }
    } else {
        return err_bad_request("workload_yaml or workload_name required");
    };

    let path = crate::resources::aether_path("gitops.json");
    let config = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<GitOpsConfig>(&s).ok())
            .or_else(|| {
                std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<crate::gitops::GitOpsStatus>(&s).ok())
                    .map(|st| GitOpsConfig {
                        repo_url: st.repo_url,
                        branch: st.branch,
                        ..Default::default()
                    })
            })
            .unwrap_or_default()
    } else {
        GitOpsConfig::default()
    };

    match config
        .resolve_deploy_target(body.environment.as_deref(), &spec)
        .await
    {
        Ok(target) => ok_json(target),
        Err(e) => err_bad_request(e),
    }
}
