// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Hosted SaaS API — tenants, billing, keys, and Stripe.

use super::handlers::{err_bad_request, err_not_found, ok_json};
use super::types::AppState;
use crate::hosted::billing::usage_summary;
use crate::hosted::keys::TenantKeyStore;
use crate::hosted::metering::UsageMeter;
use crate::hosted::stripe::{
    configured as stripe_configured, create_checkout_session, handle_webhook_event,
    verify_webhook_signature, StripeCheckoutRequest,
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
    let plan = match body.plan.as_deref().unwrap_or("free").to_lowercase().as_str() {
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
