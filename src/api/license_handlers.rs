// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

//! API handlers for Zeus OS license status and management.

use axum::extract::State as AxumState;
use axum::response::IntoResponse;

use super::handlers::ok_json;
use super::types::AppState;
use crate::license::status::{LicenseStatusResponse, LicenseUsageResponse};

/// `GET /api/license/status`
///
/// Returns the current license state and claims summary using the cached
/// node count. Does not make a live K8s API call (see `/api/license/usage`).
pub(crate) async fn api_license_status(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let store = app_state.license.read().await;
    let response = LicenseStatusResponse::from_state_and_claims(
        &store.state,
        store.claims.as_ref(),
        store.last_node_count,
    );
    ok_json(response)
}

/// `GET /api/license/usage`
///
/// Calls the Kubernetes API to get a live node count, updates the cached
/// count, and returns a per-node breakdown. Falls back to cached data if
/// no cluster is reachable.
pub(crate) async fn api_license_usage(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let count_result = crate::license::usage::count_managed_nodes().await;

    match count_result {
        Err(e) => {
            tracing::warn!("License usage: failed to count nodes: {}", e);
            let store = app_state.license.read().await;
            let allowed = store.claims.as_ref().map(|c| c.allowed_nodes).unwrap_or(0);
            let count = store.last_node_count.unwrap_or(0);
            ok_json(LicenseUsageResponse {
                allowed_nodes: allowed,
                current_node_count: count,
                utilization_pct: 0.0,
                nodes: vec![],
            })
            .into_response()
        }
        Ok(result) => {
            let count = result.managed_count;

            {
                let mut store = app_state.license.write().await;
                store.refresh_state(Some(count));
            }

            let store = app_state.license.read().await;
            let allowed = store.claims.as_ref().map(|c| c.allowed_nodes).unwrap_or(0);
            let utilization_pct = if allowed > 0 {
                (count as f64 / allowed as f64) * 100.0
            } else {
                0.0
            };

            ok_json(LicenseUsageResponse {
                allowed_nodes: allowed,
                current_node_count: count,
                utilization_pct,
                nodes: result.all_nodes,
            })
            .into_response()
        }
    }
}

/// `POST /api/license/reload`  (Admin only — enforced by RBAC middleware)
///
/// Reloads the license file from disk and replaces the in-memory store.
pub(crate) async fn api_license_reload(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let path = crate::license::license_path();
    let new_store = crate::license::LicenseStore::load_from_path(path);
    let new_state = new_store.state.clone();

    {
        let mut store = app_state.license.write().await;
        *store = new_store;
    }

    {
        use crate::audit::{ActionResult, AuditLog};
        let audit_path = AuditLog::default_path();
        let mut log = AuditLog::load(&audit_path).unwrap_or_default();
        log.record(
            crate::audit::AuditAction::LicenseReloaded,
            "zeus-os",
            Some("license"),
            ActionResult::Success,
            &format!("License reloaded via API; new state: {}", new_state),
            None,
        );
        if let Err(e) = log.save(&audit_path) {
            tracing::warn!("Failed to save audit log after license reload: {}", e);
        }
    }

    ok_json(serde_json::json!({
        "reloaded": true,
        "state": new_state.to_string(),
    }))
}
