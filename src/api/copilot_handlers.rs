// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Copilot API handlers.

use super::handlers::{err_bad_request, ok_json};
use super::types::{ApiResponse, AppState};
use axum::extract::{Path, State as AxumState};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use crate::copilot::agent::{tool_context, CopilotAgent};
use crate::copilot::diagnose::{DiagnoseRequest, DiagnoseResponse};
use serde::Deserialize;
use std::sync::LazyLock;

static COPILOT: LazyLock<CopilotAgent> = LazyLock::new(CopilotAgent::new);

#[derive(Debug, Deserialize)]
pub(crate) struct CopilotChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub confirm_action_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TroubleshootRequest {
    pub workload: String,
    pub cluster: Option<String>,
    pub namespace: Option<String>,
    pub kind: Option<String>,
    pub include_copilot_summary: Option<bool>,
}

/// POST /api/copilot/troubleshoot — Live cluster evidence diagnosis.
pub(crate) async fn api_copilot_troubleshoot(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<TroubleshootRequest>,
) -> impl IntoResponse {
    if req.workload.trim().is_empty() {
        return err_bad_request::<DiagnoseResponse>("workload is required").into_response();
    }
    let store = app_state.state.read().await;
    let diagnose_req = DiagnoseRequest {
        workload: req.workload.clone(),
        cluster: req.cluster.clone(),
        namespace: req.namespace.clone(),
        kind: req.kind.clone(),
    };
    let mut report = match crate::copilot::diagnose::diagnose_workload(&diagnose_req, &store).await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<DiagnoseResponse> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };
    drop(store);

    if req.include_copilot_summary.unwrap_or(true) {
        let role = crate::rbac::Role::Admin;
        let ctx = tool_context(
            app_state.state.clone(),
            app_state.state_path.clone(),
            role,
        );
        let prompt = format!(
            "Summarize this workload diagnosis in 2-3 sentences with root cause and recommended fix:\n{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        if let Ok(resp) = COPILOT
            .chat(&prompt, None, None, &ctx)
            .await
        {
            report
                .evidence
                .insert(0, format!("AI summary: {}", resp.reply));
        }
    }

    ok_json(report).into_response()
}

/// GET /api/copilot/troubleshoot/fleet — Batch root-cause analysis for unhealthy workloads.
pub(crate) async fn api_copilot_troubleshoot_fleet(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let store = app_state.state.read().await;
    let report = crate::copilot::diagnose::diagnose_fleet(&store, 12).await;
    ok_json(report).into_response()
}

/// POST /api/copilot/chat
pub(crate) async fn api_copilot_chat(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<CopilotChatRequest>,
) -> impl IntoResponse {
    if req.message.trim().is_empty() {
        return err_bad_request::<serde_json::Value>("message is required").into_response();
    }
    let role = crate::rbac::Role::Admin;
    let ctx = tool_context(
        app_state.state.clone(),
        app_state.state_path.clone(),
        role,
    );
    match COPILOT
        .chat(
            &req.message,
            req.session_id.as_deref(),
            req.confirm_action_id.as_deref(),
            &ctx,
        )
        .await
    {
        Ok(resp) => ok_json(resp).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}

/// GET /api/copilot/sessions/:id
pub(crate) async fn api_copilot_session(
    Path(id): Path<String>,
) -> impl IntoResponse {
    match COPILOT.get_session(&id) {
        Some(s) => ok_json(s).into_response(),
        None => err_bad_request::<serde_json::Value>("session not found").into_response(),
    }
}

/// POST /api/copilot/confirm/:action_id
pub(crate) async fn api_copilot_confirm(
    AxumState(app_state): AxumState<AppState>,
    Path(action_id): Path<String>,
    Json(req): Json<CopilotChatRequest>,
) -> impl IntoResponse {
    let role = crate::rbac::Role::Operator;
    let ctx = tool_context(
        app_state.state.clone(),
        app_state.state_path.clone(),
        role,
    );
    match COPILOT
        .chat(
            "confirm action",
            req.session_id.as_deref(),
            Some(&action_id),
            &ctx,
        )
        .await
    {
        Ok(resp) => ok_json(resp).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}
