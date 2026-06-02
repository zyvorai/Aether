// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! PacketWolf ecosystem API handlers.

use super::handlers::{err_bad_request, err_service_unavailable, ok_json};
use axum::{extract::Query, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct DeeplinkQuery {
    pub namespace: Option<String>,
    pub pod: Option<String>,
    pub workload: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnomaliesQuery {
    pub limit: Option<u32>,
}

pub(crate) async fn api_packetwolf_status() -> impl IntoResponse {
    let status = crate::ecosystem::packetwolf::status().await;
    if status.configured && !status.reachable {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(super::types::ApiResponse {
                success: true,
                data: Some(status),
                error: None,
            }),
        );
    }
    ok_json(status)
}

pub(crate) async fn api_packetwolf_flow_stats() -> impl IntoResponse {
    if !crate::ecosystem::packetwolf::config().configured {
        return ok_json(serde_json::json!({"configured": false}));
    }
    match crate::ecosystem::packetwolf::flow_stats().await {
        Ok(v) => ok_json(v),
        Err(e) => err_service_unavailable(e),
    }
}

pub(crate) async fn api_packetwolf_verify_egress(
    Json(body): Json<crate::ecosystem::packetwolf::VerifyEgressRequest>,
) -> impl IntoResponse {
    if body.namespace.trim().is_empty() || body.pod.trim().is_empty() {
        return err_bad_request("namespace and pod are required");
    }
    if !crate::ecosystem::packetwolf::config().configured {
        return ok_json(serde_json::json!({"configured": false, "skipped": true}));
    }
    match crate::ecosystem::packetwolf::verify_egress(&body).await {
        Ok(v) => ok_json(v),
        Err(e) => err_service_unavailable(e),
    }
}

pub(crate) async fn api_packetwolf_anomalies(Query(q): Query<AnomaliesQuery>) -> impl IntoResponse {
    if !crate::ecosystem::packetwolf::config().configured {
        return ok_json(serde_json::json!({"configured": false, "anomalies": []}));
    }
    match crate::ecosystem::packetwolf::anomalies(q.limit).await {
        Ok(v) => ok_json(v),
        Err(e) => err_service_unavailable(e),
    }
}

pub(crate) async fn api_packetwolf_deeplink(Query(q): Query<DeeplinkQuery>) -> impl IntoResponse {
    ok_json(crate::ecosystem::packetwolf::deeplink(
        q.namespace.as_deref(),
        q.pod.as_deref(),
        q.workload.as_deref(),
    ))
}
