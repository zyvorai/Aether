// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zeus API handlers.

use super::handlers::{err_bad_request, ok_json};
use super::types::{ApiResponse, AppState};
use crate::zeus::agent::{tool_context, ZeusAgent};
use crate::zeus::diagnose::{DiagnoseRequest, DiagnoseResponse};
use crate::zeus::marketplace::{build_marketplace, install_agent, uninstall_agent};
use crate::zeus::memory::{
    purge_memory, read_memory_settings, read_zeus_memory, save_memory_settings, ZeusMemorySettings,
};
use crate::zeus::prompts::{
    delete_prompt, export_yaml, import_yaml, list_prompts, upsert_prompt, ZeusPrompt,
};
use crate::zeus::providers::{
    build_provider_status, delete_provider, list_providers_public, test_provider, upsert_provider,
    ZeusProviderConfig, ZeusProviderRegistry,
};
use axum::extract::{Path, State as AxumState};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use std::sync::LazyLock;

static ZEUS: LazyLock<ZeusAgent> = LazyLock::new(ZeusAgent::new);

fn with_deprecation(resp: Response) -> Response {
    let mut resp = resp;
    if let Ok(v) = HeaderValue::from_str("use /api/zeus/*") {
        resp.headers_mut().insert("Deprecation", v);
    }
    resp
}

#[derive(Debug, Deserialize)]
pub(crate) struct ZeusChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub confirm_action_id: Option<String>,
    pub agent_focus: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ZeusBatchConfirmRequest {
    pub session_id: String,
    pub action_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ZeusRouteRequest {
    pub message: String,
    pub agent_focus: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RunbookAuthorBody {
    pub prompt: String,
    pub workload: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PolicyExplainerBody {
    pub workload: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TroubleshootRequest {
    pub workload: String,
    pub cluster: Option<String>,
    pub namespace: Option<String>,
    pub kind: Option<String>,
    pub include_zeus_summary: Option<bool>,
    #[serde(alias = "include_copilot_summary")]
    pub include_copilot_summary: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderUpsertBody {
    #[serde(flatten)]
    pub config: ZeusProviderConfig,
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PromptImportBody {
    pub yaml: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MarketplaceInstallBody {
    pub agent_id: String,
}

pub(crate) async fn api_zeus_troubleshoot(
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
    let mut report = match crate::zeus::diagnose::diagnose_workload(&diagnose_req, &store).await {
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

    let include = req
        .include_zeus_summary
        .or(req.include_copilot_summary)
        .unwrap_or(true);
    if include {
        let ctx = tool_context(
            app_state.state.clone(),
            app_state.state_path.clone(),
            crate::rbac::Role::Admin,
        );
        let prompt = format!(
            "Summarize this workload diagnosis in 2-3 sentences with root cause and recommended fix:\n{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        if let Ok(resp) = ZEUS.chat(&prompt, None, None, None, &ctx).await {
            report
                .evidence
                .insert(0, format!("Zeus summary: {}", resp.reply));
        }
    }
    ok_json(report).into_response()
}

pub(crate) async fn api_zeus_troubleshoot_fleet(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let store = app_state.state.read().await;
    ok_json(crate::zeus::diagnose::diagnose_fleet(&store, 12).await).into_response()
}

pub(crate) async fn api_zeus_chat(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<ZeusChatRequest>,
) -> impl IntoResponse {
    if req.message.trim().is_empty() {
        return err_bad_request::<serde_json::Value>("message is required").into_response();
    }
    let ctx = tool_context(
        app_state.state.clone(),
        app_state.state_path.clone(),
        crate::rbac::Role::Admin,
    );
    match ZEUS
        .chat(
            &req.message,
            req.session_id.as_deref(),
            req.confirm_action_id.as_deref(),
            req.agent_focus.as_deref(),
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

pub(crate) async fn api_zeus_session(Path(id): Path<String>) -> impl IntoResponse {
    match ZEUS.get_session(&id) {
        Some(s) => ok_json(s).into_response(),
        None => err_bad_request::<serde_json::Value>("session not found").into_response(),
    }
}

pub(crate) async fn api_zeus_confirm(
    AxumState(app_state): AxumState<AppState>,
    Path(action_id): Path<String>,
    Json(req): Json<ZeusChatRequest>,
) -> impl IntoResponse {
    let ctx = tool_context(
        app_state.state.clone(),
        app_state.state_path.clone(),
        crate::rbac::Role::Operator,
    );
    match ZEUS
        .chat(
            "confirm action",
            req.session_id.as_deref(),
            Some(&action_id),
            req.agent_focus.as_deref(),
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

pub(crate) async fn api_zeus_confirm_batch(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<ZeusBatchConfirmRequest>,
) -> impl IntoResponse {
    if req.session_id.trim().is_empty() || req.action_ids.is_empty() {
        return err_bad_request::<serde_json::Value>("session_id and action_ids required")
            .into_response();
    }
    let ctx = tool_context(
        app_state.state.clone(),
        app_state.state_path.clone(),
        crate::rbac::Role::Operator,
    );
    match ZEUS
        .confirm_batch(&req.session_id, &req.action_ids, &ctx)
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

pub(crate) async fn api_zeus_insights(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    match crate::intelligence::zeus_os::build_zeus_insights(&app_state.state_path).await {
        Ok(r) => ok_json(r).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_intelligence_zeus_memory() -> impl IntoResponse {
    ok_json(read_zeus_memory()).into_response()
}

pub(crate) async fn api_intelligence_zeus_memory_settings(
    Json(settings): Json<ZeusMemorySettings>,
) -> impl IntoResponse {
    match save_memory_settings(&settings) {
        Ok(()) => ok_json(read_memory_settings()).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_intelligence_zeus_memory_purge() -> impl IntoResponse {
    match purge_memory() {
        Ok(()) => ok_json(serde_json::json!({"purged": true})).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_intelligence_zeus_route(
    Json(req): Json<ZeusRouteRequest>,
) -> impl IntoResponse {
    ok_json(crate::zeus::routing::route_message(
        &req.message,
        req.agent_focus.as_deref(),
    ))
    .into_response()
}

pub(crate) async fn api_intelligence_zeus_llm_status() -> impl IntoResponse {
    ok_json(build_provider_status()).into_response()
}

pub(crate) async fn api_intelligence_zeus_voice_lab() -> impl IntoResponse {
    ok_json(crate::intelligence::zeus_os::build_voice_zeus_lab()).into_response()
}

pub(crate) async fn api_intelligence_zeus_runbook(
    Json(req): Json<RunbookAuthorBody>,
) -> impl IntoResponse {
    if req.prompt.trim().is_empty() {
        return err_bad_request::<serde_json::Value>("prompt is required").into_response();
    }
    ok_json(crate::intelligence::zeus_os::author_runbook(
        &crate::intelligence::zeus_os::RunbookAuthorRequest {
            prompt: req.prompt,
            workload: req.workload,
        },
    ))
    .into_response()
}

pub(crate) async fn api_intelligence_zeus_policy_explain(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<PolicyExplainerBody>,
) -> impl IntoResponse {
    match crate::intelligence::zeus_os::explain_policy_violations(
        &app_state.state_path,
        &crate::intelligence::zeus_os::PolicyExplainerRequest {
            workload: req.workload,
        },
    ) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_intelligence_zeus_audit() -> impl IntoResponse {
    ok_json(crate::intelligence::zeus_os::read_zeus_audit(50)).into_response()
}

pub(crate) async fn api_intelligence_zeus_rbac_scopes() -> impl IntoResponse {
    ok_json(crate::intelligence::zeus_os::build_zeus_rbac_scopes(
        crate::rbac::Role::Admin,
    ))
    .into_response()
}

pub(crate) async fn api_zeus_providers_list() -> impl IntoResponse {
    ok_json(list_providers_public()).into_response()
}

pub(crate) async fn api_zeus_providers_upsert(
    Json(body): Json<ProviderUpsertBody>,
) -> impl IntoResponse {
    match upsert_provider(body.config, body.api_key.as_deref()) {
        Ok(p) => ok_json(p).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_providers_delete(Path(id): Path<String>) -> impl IntoResponse {
    match delete_provider(&id) {
        Ok(deleted) => ok_json(serde_json::json!({"deleted": deleted})).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_providers_test(Path(id): Path<String>) -> impl IntoResponse {
    match test_provider(&id).await {
        Ok(r) => ok_json(r).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_providers_status() -> impl IntoResponse {
    ok_json(build_provider_status()).into_response()
}

pub(crate) async fn api_zeus_providers_save_registry(
    Json(reg): Json<ZeusProviderRegistry>,
) -> impl IntoResponse {
    match crate::zeus::providers::registry::save_registry(&reg) {
        Ok(()) => ok_json(list_providers_public()).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_prompts_list() -> impl IntoResponse {
    ok_json(list_prompts()).into_response()
}

pub(crate) async fn api_zeus_prompts_upsert(Json(prompt): Json<ZeusPrompt>) -> impl IntoResponse {
    match upsert_prompt(prompt) {
        Ok(p) => ok_json(p).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_prompts_delete(Path(id): Path<String>) -> impl IntoResponse {
    match delete_prompt(&id) {
        Ok(deleted) => ok_json(serde_json::json!({"deleted": deleted})).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_prompts_export() -> impl IntoResponse {
    match export_yaml() {
        Ok(yaml) => ok_json(serde_json::json!({"yaml": yaml})).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_prompts_import(
    Json(body): Json<PromptImportBody>,
) -> impl IntoResponse {
    match import_yaml(&body.yaml) {
        Ok(lib) => ok_json(lib).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_marketplace() -> impl IntoResponse {
    ok_json(build_marketplace()).into_response()
}

pub(crate) async fn api_zeus_marketplace_install(
    Json(body): Json<MarketplaceInstallBody>,
) -> impl IntoResponse {
    match install_agent(&body.agent_id) {
        Ok(r) => ok_json(r).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_marketplace_uninstall(
    Json(body): Json<MarketplaceInstallBody>,
) -> impl IntoResponse {
    match uninstall_agent(&body.agent_id) {
        Ok(r) => ok_json(r).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

pub(crate) async fn api_zeus_agents_list() -> impl IntoResponse {
    ok_json(crate::zeus::agents::all_agents()).into_response()
}

// Deprecated /api/copilot/* aliases
pub(crate) async fn api_copilot_chat(
    state: AxumState<AppState>,
    req: Json<ZeusChatRequest>,
) -> impl IntoResponse {
    with_deprecation(api_zeus_chat(state, req).await.into_response())
}

pub(crate) async fn api_copilot_troubleshoot(
    state: AxumState<AppState>,
    req: Json<TroubleshootRequest>,
) -> impl IntoResponse {
    with_deprecation(api_zeus_troubleshoot(state, req).await.into_response())
}

pub(crate) async fn api_copilot_troubleshoot_fleet(
    state: AxumState<AppState>,
) -> impl IntoResponse {
    with_deprecation(api_zeus_troubleshoot_fleet(state).await.into_response())
}

pub(crate) async fn api_copilot_session(id: Path<String>) -> impl IntoResponse {
    with_deprecation(api_zeus_session(id).await.into_response())
}

pub(crate) async fn api_copilot_confirm(
    state: AxumState<AppState>,
    action_id: Path<String>,
    req: Json<ZeusChatRequest>,
) -> impl IntoResponse {
    with_deprecation(
        api_zeus_confirm(state, action_id, req)
            .await
            .into_response(),
    )
}

pub(crate) async fn api_copilot_confirm_batch(
    state: AxumState<AppState>,
    req: Json<ZeusBatchConfirmRequest>,
) -> impl IntoResponse {
    with_deprecation(api_zeus_confirm_batch(state, req).await.into_response())
}

pub(crate) async fn api_intelligence_copilot_memory() -> impl IntoResponse {
    with_deprecation(api_intelligence_zeus_memory().await.into_response())
}

pub(crate) async fn api_intelligence_copilot_route(
    req: Json<ZeusRouteRequest>,
) -> impl IntoResponse {
    with_deprecation(api_intelligence_zeus_route(req).await.into_response())
}

pub(crate) async fn api_intelligence_copilot_llm_status() -> impl IntoResponse {
    with_deprecation(api_intelligence_zeus_llm_status().await.into_response())
}

pub(crate) async fn api_intelligence_copilot_voice_lab() -> impl IntoResponse {
    with_deprecation(api_intelligence_zeus_voice_lab().await.into_response())
}

pub(crate) async fn api_intelligence_copilot_runbook(
    req: Json<RunbookAuthorBody>,
) -> impl IntoResponse {
    with_deprecation(api_intelligence_zeus_runbook(req).await.into_response())
}

pub(crate) async fn api_intelligence_copilot_policy_explain(
    state: AxumState<AppState>,
    req: Json<PolicyExplainerBody>,
) -> impl IntoResponse {
    with_deprecation(
        api_intelligence_zeus_policy_explain(state, req)
            .await
            .into_response(),
    )
}

pub(crate) async fn api_intelligence_copilot_audit() -> impl IntoResponse {
    with_deprecation(api_intelligence_zeus_audit().await.into_response())
}

pub(crate) async fn api_intelligence_copilot_rbac_scopes() -> impl IntoResponse {
    with_deprecation(api_intelligence_zeus_rbac_scopes().await.into_response())
}
