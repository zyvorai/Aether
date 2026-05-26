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
use serde::Deserialize;
use std::sync::LazyLock;

static COPILOT: LazyLock<CopilotAgent> = LazyLock::new(CopilotAgent::new);

#[derive(Debug, Deserialize)]
pub(crate) struct CopilotChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub confirm_action_id: Option<String>,
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
