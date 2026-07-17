// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Hosted SaaS API — tenants, billing, keys, and Stripe.

use super::handlers::{err_bad_request, err_not_found, ok_json};
use super::types::AppState;
use crate::fleet::federation::{self, FederationPlanRequest};
use crate::hosted::billing::usage_summary;
use crate::hosted::keys::TenantKeyStore;
use crate::hosted::metering::UsageMeter;
use crate::hosted::stripe::{
    configured as stripe_configured, create_checkout_session, create_portal_session,
    handle_webhook_event, verify_webhook_signature, StripeCheckoutRequest, StripePortalRequest,
};
use crate::hosted::tenant::{TenantPlan, TenantStore};
use axum::{
    body::Bytes,
    extract::{Path, State as AxumState},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub(crate) struct CreateTenantBody {
    pub name: String,
    pub slug: String,
    pub plan: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IssueTenantKeyBody {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpgradeTenantBody {
    pub plan: String,
}

pub(crate) async fn api_hosted_tenants_list() -> impl IntoResponse {
    let store = TenantStore::load();
    ok_json(store.list())
}

pub(crate) async fn api_hosted_tenants_create(
    Json(body): Json<CreateTenantBody>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() || body.slug.trim().is_empty() {
        return err_bad_request("name and slug are required");
    }
    let plan = match body
        .plan
        .as_deref()
        .unwrap_or("free")
        .to_lowercase()
        .as_str()
    {
        "team" => TenantPlan::Team,
        "enterprise" => TenantPlan::Enterprise,
        _ => TenantPlan::Free,
    };
    let mut store = TenantStore::load();
    match store.create(&body.name, &body.slug, plan) {
        Ok(t) => ok_json(t),
        Err(e) => err_bad_request(e),
    }
}

pub(crate) async fn api_hosted_tenants_deactivate(Path(id): Path<String>) -> impl IntoResponse {
    let mut store = TenantStore::load();
    match store.deactivate(&id) {
        Ok(()) => ok_json(serde_json::json!({"id": id, "active": false})),
        Err(e) => err_not_found(e.to_string()),
    }
}

pub(crate) async fn api_hosted_tenants_upgrade(
    Path(id): Path<String>,
    Json(body): Json<UpgradeTenantBody>,
) -> impl IntoResponse {
    let plan = match body.plan.to_lowercase().as_str() {
        "team" => TenantPlan::Team,
        "enterprise" => TenantPlan::Enterprise,
        _ => TenantPlan::Free,
    };
    let mut store = TenantStore::load();
    match store.upgrade_plan(&id, plan) {
        Ok(()) => match store.get(&id) {
            Some(t) => ok_json(t),
            None => err_not_found("tenant not found"),
        },
        Err(e) => err_bad_request(e),
    }
}

pub(crate) async fn api_hosted_upgrades_status() -> impl IntoResponse {
    ok_json(serde_json::json!({
        "current_version": env!("CARGO_PKG_VERSION"),
        "channel": "stable",
        "managed_upgrades": true,
        "upgrade_available": false,
        "notes": "Hosted control plane receives rolling upgrades during maintenance windows.",
    }))
}

pub(crate) async fn api_hosted_billing_usage(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let store = app_state.state.read().await;
    let tenants = TenantStore::load();
    let mut summary = usage_summary(&store, &tenants);
    let meter = UsageMeter::load();
    for line in &mut summary.tenants {
        line.api_requests_estimate = meter.requests_for(&line.tenant_id);
    }
    ok_json(summary)
}

pub(crate) async fn api_hosted_metering_usage() -> impl IntoResponse {
    let meter = UsageMeter::load();
    ok_json(meter.snapshot())
}

pub(crate) async fn api_hosted_tenant_keys_list(Path(id): Path<String>) -> impl IntoResponse {
    let store = TenantKeyStore::load();
    ok_json(store.list_for_tenant(&id))
}

pub(crate) async fn api_hosted_tenant_keys_issue(
    Path(id): Path<String>,
    Json(body): Json<IssueTenantKeyBody>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return err_bad_request("name is required");
    }
    let tenants = TenantStore::load();
    if tenants.get(&id).is_none() {
        return err_not_found("tenant not found");
    }
    let mut store = TenantKeyStore::load();
    match store.issue(&id, &body.name) {
        Ok((plain, rec)) => ok_json(serde_json::json!({
            "key": plain,
            "record": rec,
        })),
        Err(e) => err_bad_request(e),
    }
}

pub(crate) async fn api_hosted_tenant_keys_revoke(
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let mut store = TenantKeyStore::load();
    match store.revoke(&id, &name) {
        Ok(()) => ok_json(serde_json::json!({"tenant_id": id, "name": name, "active": false})),
        Err(e) => err_bad_request(e),
    }
}

pub(crate) async fn api_hosted_billing_stripe_checkout(
    Json(body): Json<StripeCheckoutRequest>,
) -> impl IntoResponse {
    if !stripe_configured() {
        return err_bad_request("Stripe is not configured (set AETHER_STRIPE_SECRET_KEY)");
    }
    match create_checkout_session(&body).await {
        Ok(resp) => ok_json(resp),
        Err(e) => err_bad_request(e),
    }
}

pub(crate) async fn api_hosted_billing_stripe_portal(
    Json(body): Json<StripePortalRequest>,
) -> impl IntoResponse {
    if !stripe_configured() {
        return err_bad_request("Stripe is not configured (set AETHER_STRIPE_SECRET_KEY)");
    }
    match create_portal_session(&body).await {
        Ok(resp) => ok_json(resp),
        Err(e) => err_bad_request(e),
    }
}

pub(crate) async fn api_hosted_billing_stripe_webhook(
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if let Err(e) = verify_webhook_signature(&body, sig) {
        return err_bad_request(e);
    }
    match handle_webhook_event(&body) {
        Ok(result) => ok_json(result),
        Err(e) => err_bad_request(e),
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct HostedFederationStatus {
    pub federation_enabled: bool,
    pub tenant_count: usize,
    pub policy: federation::FederationPolicy,
}

pub(crate) async fn api_hosted_federation_status() -> impl IntoResponse {
    let policy = federation::federation_policies();
    let tenants = TenantStore::load().list();
    ok_json(HostedFederationStatus {
        federation_enabled: !policy.clusters.is_empty(),
        tenant_count: tenants.len(),
        policy,
    })
}

pub(crate) async fn api_hosted_tenant_federation_plan(
    AxumState(app_state): AxumState<AppState>,
    Path(id): Path<String>,
    Json(body): Json<FederationPlanRequest>,
) -> impl IntoResponse {
    let store = TenantStore::load();
    if store.get(&id).is_none() {
        return err_not_found(format!("tenant {id} not found"));
    }
    let spec = if let Some(yaml) = body.workload_yaml.filter(|s| !s.trim().is_empty()) {
        match federation::parse_workload_yaml(&yaml) {
            Ok(spec) => spec,
            Err(e) => return err_bad_request(e),
        }
    } else if let Some(name) = body.workload_name.filter(|s| !s.trim().is_empty()) {
        let state = app_state.state.read().await;
        let ws = match state.get(&name) {
            Some(w) => w,
            None => {
                return err_bad_request(format!(
                    "workload {name} not found — pass workload_yaml for tenant federation plan"
                ));
            }
        };
        let path = ws.spec_path.clone();
        drop(state);
        match crate::spec::Workload::from_file(&path) {
            Ok(spec) => spec,
            Err(e) => return err_bad_request(e),
        }
    } else {
        return err_bad_request("workload_yaml or workload_name is required");
    };
    match federation::plan_placement(&spec).await {
        Ok(plan) => ok_json(plan),
        Err(e) => err_bad_request(e.to_string()),
    }
}
