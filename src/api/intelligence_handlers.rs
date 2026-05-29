// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Intelligence layer API handlers.

use super::handlers::{err_internal, lookup_workload, ok_json};
use super::types::{ApiResponse, AppState};
use axum::extract::{Path, State as AxumState};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use crate::config::Config;
use crate::intelligence::context::build_context_snapshot;
use crate::intelligence::evolution::EvolutionEngine;
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::placement::GlobalPlacementEngine;
use crate::intelligence::policy::AutonomyPolicy;
use crate::intelligence::predict::FailurePredictor;
use crate::intelligence::security::SecurityEngine;
use crate::intelligence::store::IntelligenceStore;
use crate::spec::Workload;

async fn workload_pairs(
    app_state: &AppState,
) -> Vec<(Workload, crate::state::WorkloadState)> {
    let store = app_state.state.read().await;
    store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|s| (s, (*ws).clone()))
        })
        .collect()
}

/// GET /api/context/snapshot
pub(crate) async fn api_context_snapshot(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match build_context_snapshot(&app_state.state_path).await {
        Ok(snap) => ok_json(snap).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/predictions
pub(crate) async fn api_intelligence_predictions(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(FailurePredictor::predict_fleet(&pairs))
}

/// GET /api/intelligence/predictions/:name
pub(crate) async fn api_intelligence_prediction_workload(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl axum::response::IntoResponse {
    let ws = match lookup_workload::<serde_json::Value>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e.into_response(),
    };
    let spec = match Workload::from_file(&ws.spec_path) {
        Ok(s) => s,
        Err(e) => return err_internal::<serde_json::Value>(e.to_string()).into_response(),
    };
    let health = crate::health::HealthHistory::load(&crate::health::HealthHistory::default_path())
        .unwrap_or_default();
    ok_json(FailurePredictor::predict_workload(&spec, &ws, &health)).into_response()
}

/// GET /api/intelligence/cost-optimize
pub(crate) async fn api_intelligence_cost_optimize(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(FinOpsEngine::optimize_fleet(&pairs))
}

/// GET /api/intelligence/threats
pub(crate) async fn api_intelligence_threats(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(SecurityEngine::scan_fleet(&pairs))
}

/// GET /api/intelligence/evolution/status
pub(crate) async fn api_intelligence_evolution_status(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let pairs = workload_pairs(&app_state).await;
    ok_json(EvolutionEngine::status_for_fleet(&pairs, &policy))
}

/// GET /api/intelligence/runtime-evolution/:name
pub(crate) async fn api_intelligence_runtime_evolution(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
) -> impl axum::response::IntoResponse {
    let ws = match lookup_workload::<serde_json::Value>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e.into_response(),
    };
    let spec = match Workload::from_file(&ws.spec_path) {
        Ok(s) => s,
        Err(e) => return err_internal::<serde_json::Value>(e.to_string()).into_response(),
    };
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(
        config.reconciliation.auto_reconcile,
        Some(&spec),
    );
    let intel = IntelligenceStore::load(&IntelligenceStore::default_path()).unwrap_or_default();
    let engine = crate::ai::scoring::ScoringEngine::new(config.engine)
        .with_history(intel.runtime_history_map());
    ok_json(EvolutionEngine::status_for_workload(
        &spec, &ws, &engine, &policy,
    ))
    .into_response()
}

/// POST /api/intelligence/place
pub(crate) async fn api_intelligence_place(
    Json(payload): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    let spec: Workload = if let Some(yaml) = payload.get("yaml").and_then(|v| v.as_str()) {
        match serde_yaml::from_str(yaml) {
            Ok(s) => s,
            Err(e) => return err_internal::<serde_json::Value>(e.to_string()).into_response(),
        }
    } else {
        match serde_json::from_value(payload.get("workload").cloned().unwrap_or(payload)) {
            Ok(s) => s,
            Err(e) => return err_internal::<serde_json::Value>(e.to_string()).into_response(),
        }
    };
    let clusters = crate::kubecluster::list_clusters().await.unwrap_or_default();
    ok_json(GlobalPlacementEngine::recommend(&spec, &clusters)).into_response()
}

/// GET /api/ai/migration-plan/:name/:target
pub(crate) async fn api_ai_migration_plan(
    AxumState(app_state): AxumState<AppState>,
    Path((name, target)): Path<(String, String)>,
) -> impl axum::response::IntoResponse {
    use crate::ai::migration::MigrationAdvisor;
    use crate::runtime::RuntimeKind;

    let ws = match lookup_workload::<serde_json::Value>(&app_state, &name).await {
        Ok(w) => w,
        Err(e) => return e.into_response(),
    };
    let target_runtime = match target.parse::<RuntimeKind>() {
        Ok(rt) => rt,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };
    let spec = match Workload::from_file(&ws.spec_path) {
        Ok(s) => s,
        Err(e) => return err_internal::<serde_json::Value>(e.to_string()).into_response(),
    };
    let config = Config::load();
    let advisor = MigrationAdvisor::new(config.migration);
    let plan = advisor.plan_proposal(&spec, ws.runtime, target_runtime);
    ok_json(plan).into_response()
}

/// GET /api/intelligence/remediation/plan
pub(crate) async fn api_intelligence_remediation_plan(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let store = app_state.state.read().await;
    let plan = crate::intelligence::remediation::build_remediation_plan(&store).await;
    ok_json(plan).into_response()
}

/// POST /api/intelligence/remediation/execute
pub(crate) async fn api_intelligence_remediation_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::remediation::RemediationExecuteRequest>,
) -> impl axum::response::IntoResponse {
    let store = app_state.state.read().await;
    let plan = crate::intelligence::remediation::build_remediation_plan(&store).await;
    let dry_run = body.dry_run.unwrap_or(true);
    let max_actions = body.max_actions.unwrap_or(10);
    match crate::intelligence::remediation::execute_remediation(&plan, dry_run, max_actions).await
    {
        Ok(result) => ok_json(result).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}
