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
use crate::intelligence::briefing::build_command_center_briefing;
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

/// GET /api/command-center/briefing
pub(crate) async fn api_command_center_briefing(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match build_command_center_briefing(&app_state.state_path) {
        Ok(briefing) => ok_json(briefing).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
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

/// POST /api/intelligence/digital-twin/simulate
pub(crate) async fn api_intelligence_digital_twin_simulate(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<crate::intelligence::twin::TwinSimulateRequest>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::twin::DigitalTwinEngine::simulate(&pairs, &req)).into_response()
}

/// GET /api/intelligence/security/policies
pub(crate) async fn api_intelligence_security_policies(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    let threats = SecurityEngine::scan_fleet(&pairs);
    ok_json(SecurityEngine::copilot_policies(&threats)).into_response()
}

/// GET /api/intelligence/autonomy/status
pub(crate) async fn api_intelligence_autonomy_status(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::autonomy::build_autonomy_status(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/knowledge-graph
pub(crate) async fn api_intelligence_knowledge_graph(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::graph::build_knowledge_graph(&app_state.state_path) {
        Ok(graph) => ok_json(graph).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/healer/preview
pub(crate) async fn api_intelligence_healer_preview(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let store = app_state.state.read().await;
    ok_json(crate::intelligence::healer::build_healer_preview(&store, &policy).await).into_response()
}

/// POST /api/intelligence/intent-pipeline
pub(crate) async fn api_intelligence_intent_pipeline(
    Json(req): Json<crate::intelligence::pipeline::IntentPipelineRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::pipeline::build_intent_pipeline(&req).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/sre/runbook
pub(crate) async fn api_intelligence_sre_runbook(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::sre::build_sre_runbook(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/multicloud/posture
pub(crate) async fn api_intelligence_multicloud_posture(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::multicloud::build_multicloud_posture(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/autonomous/placement
pub(crate) async fn api_intelligence_autonomous_placement(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::pipeline::build_autonomous_placement(&app_state.state_path) {
        Ok(status) => ok_json(status).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/command-center/next-actions
pub(crate) async fn api_command_center_next_actions(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::actions::build_next_actions(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/agents/status
pub(crate) async fn api_intelligence_agents_status(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::agents::build_agent_registry(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/healer/execute
pub(crate) async fn api_intelligence_healer_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::healer::HealerExecuteRequest>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let store = app_state.state.read().await;
    let report = crate::intelligence::healer::execute_healer(
        &store,
        &app_state.state_path,
        &policy,
        body.dry_run,
    )
    .await;
    ok_json(report).into_response()
}

/// GET /api/command-center/notifications
pub(crate) async fn api_command_center_notifications(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::notifications::build_critical_notifications(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/evolution/execute
pub(crate) async fn api_intelligence_evolution_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::evolution::EvolutionExecuteRequest>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let max = body.max_actions.unwrap_or(5);
    match crate::intelligence::evolution::execute_evolution(
        &app_state.state_path,
        &policy,
        body.dry_run,
        max,
    )
    .await
    {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/gitops/agent/plan
pub(crate) async fn api_intelligence_gitops_agent_plan(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::gitops_agent::build_gitops_agent_plan(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/gitops/agent/sync
pub(crate) async fn api_intelligence_gitops_agent_sync(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::gitops_agent::GitOpsAgentExecuteRequest>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    match crate::intelligence::gitops_agent::execute_gitops_agent(
        &app_state.state_path,
        &policy,
        body.dry_run,
    )
    .await
    {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/cost-optimize/apply
pub(crate) async fn api_intelligence_cost_optimize_apply(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::finops::CostApplyRequest>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let pairs = workload_pairs(&app_state).await;
    let patches = FinOpsEngine::build_cost_patches(&pairs);
    let report = FinOpsEngine::apply_cost_patches(&patches, body.dry_run, &policy);
    ok_json(report).into_response()
}

/// POST /api/intelligence/security/remediate
pub(crate) async fn api_intelligence_security_remediate(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::security::SecurityRemediateRequest>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    let report = SecurityEngine::remediate_fleet(&pairs, body.dry_run, body.confirm);
    ok_json(report).into_response()
}

/// GET /api/intelligence/capacity/scale-suggestions
pub(crate) async fn api_intelligence_capacity_scale_suggestions(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::capacity::build_scale_suggestions(&pairs)).into_response()
}

/// POST /api/intelligence/capacity/scale/execute
pub(crate) async fn api_intelligence_capacity_scale_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::capacity::CapacityScaleExecuteRequest>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let pairs = workload_pairs(&app_state).await;
    let suggestions = crate::intelligence::capacity::build_scale_suggestions(&pairs);
    let report =
        crate::intelligence::capacity::execute_scale_suggestions(&suggestions, &policy, body.dry_run);
    ok_json(report).into_response()
}

/// POST /api/intelligence/intent/nl-parse
pub(crate) async fn api_intelligence_intent_nl_parse(
    Json(body): Json<crate::intelligence::intent_os::NlIntentRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::parse_nl_intent(&body) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/intent-pipeline/deploy
pub(crate) async fn api_intelligence_intent_pipeline_deploy(
    Json(body): Json<crate::intelligence::intent_os::IntentDeployRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::deploy_intent_pipeline(&body).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/intent/violations
pub(crate) async fn api_intelligence_intent_violations(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::scan_intent_violations(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/intent/sla-breaches
pub(crate) async fn api_intelligence_intent_sla_breaches(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::build_intent_sla_breaches(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/intent/budget/enforce
pub(crate) async fn api_intelligence_intent_budget_enforce(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::intent_os::BudgetEnforceRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::enforce_intent_budget(&app_state.state_path, body.dry_run) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/intent/compliance/check
pub(crate) async fn api_intelligence_intent_compliance_check(
    Json(body): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    let spec: Workload = if let Ok(spec) = serde_json::from_value(body.clone()) {
        spec
    } else if let Some(yaml) = body.get("yaml").and_then(|v| v.as_str()) {
        match serde_yaml::from_str(yaml) {
            Ok(spec) => spec,
            Err(e) => return err_internal::<serde_json::Value>(e.to_string()).into_response(),
        }
    } else {
        return err_internal::<serde_json::Value>("expected workload JSON or yaml field".to_string())
            .into_response();
    };
    ok_json(crate::intelligence::intent_os::check_compliance_gate(&spec)).into_response()
}

/// GET /api/intelligence/intent/templates
pub(crate) async fn api_intelligence_intent_templates() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::intent_os::list_intent_templates()).into_response()
}

/// POST /api/intelligence/intent/bundles
pub(crate) async fn api_intelligence_intent_bundles(
    Json(body): Json<crate::intelligence::intent_os::IntentBundleRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::build_intent_bundle(&body) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/intent/gitops-diff
pub(crate) async fn api_intelligence_intent_gitops_diff(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::build_intent_gitops_diff(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/intent/versions/:name
pub(crate) async fn api_intelligence_intent_versions(
    Path(name): Path<String>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::list_intent_versions(&name) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/intent/versions/:name/rollback
pub(crate) async fn api_intelligence_intent_version_rollback(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
    Json(body): Json<crate::intelligence::intent_os::IntentRollbackRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::intent_os::rollback_intent_version(&app_state.state_path, &name, &body) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/federation/execute
pub(crate) async fn api_intelligence_federation_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::federation_os::FederationExecuteRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::execute_federation_sync(
        &app_state.state_path,
        body.dry_run,
        body.workload_name.as_deref(),
    )
    .await
    {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/multicloud/cost-arbitrage
pub(crate) async fn api_intelligence_multicloud_cost_arbitrage(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::build_cost_arbitrage(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/federation/health-mesh
pub(crate) async fn api_intelligence_federation_health_mesh() -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::build_cluster_health_mesh().await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/federation/unified-fabric
pub(crate) async fn api_intelligence_federation_unified_fabric(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::build_unified_fabric(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/migration/volume-status
pub(crate) async fn api_intelligence_migration_volume_status(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::build_volume_replication_status(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/migration/wave-plan
pub(crate) async fn api_intelligence_migration_wave_plan(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::build_migration_wave(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/federation/geo-placement
pub(crate) async fn api_intelligence_federation_geo_placement(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::build_geo_placement(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/multicloud/cloud-accounts
pub(crate) async fn api_intelligence_multicloud_cloud_accounts() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::federation_os::build_cloud_account_vault()).into_response()
}

/// GET /api/intelligence/federation/region-lock
pub(crate) async fn api_intelligence_federation_region_lock(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::build_region_lock_posture(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/federation/packetwolf-guard
pub(crate) async fn api_intelligence_federation_packetwolf_guard(
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::build_packetwolf_guard().await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/federation/packetwolf-guard/apply
pub(crate) async fn api_intelligence_federation_packetwolf_guard_apply(
    Json(body): Json<crate::intelligence::federation_os::PacketWolfGuardApplyRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::federation_os::apply_packetwolf_guard(body.dry_run).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/sre/schedule
pub(crate) async fn api_intelligence_sre_schedule() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::sre_os::build_runbook_schedule()).into_response()
}

/// GET /api/intelligence/sre/incident-timeline
pub(crate) async fn api_intelligence_sre_incident_timeline() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::sre_os::build_incident_timeline(50)).into_response()
}

/// GET /api/intelligence/sre/on-call
pub(crate) async fn api_intelligence_sre_on_call() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::sre_os::build_on_call_status()).into_response()
}

/// POST /api/intelligence/sre/on-call/test
pub(crate) async fn api_intelligence_sre_on_call_test(
    Json(body): Json<crate::intelligence::sre_os::OnCallTestRequest>,
) -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::sre_os::test_on_call_webhook(
        body.dry_run,
        body.provider.as_deref(),
    ))
    .into_response()
}

/// GET /api/intelligence/sre/postmortem
pub(crate) async fn api_intelligence_sre_postmortem(
    AxumState(app_state): AxumState<AppState>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let workload = query.get("workload").map(|s| s.as_str());
    ok_json(
        crate::intelligence::sre_os::build_postmortem(&app_state.state_path, workload).await,
    )
    .into_response()
}

/// GET /api/intelligence/sre/error-budgets
pub(crate) async fn api_intelligence_sre_error_budgets(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::sre_os::build_error_budget_dashboard(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/sre/chaos/experiments
pub(crate) async fn api_intelligence_sre_chaos_experiments(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::sre_os::build_chaos_catalog(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/sre/chaos/run
pub(crate) async fn api_intelligence_sre_chaos_run(
    Json(body): Json<crate::intelligence::sre_os::ChaosRunRequest>,
) -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::sre_os::run_chaos_experiment(body.dry_run, &body.experiment_id))
        .into_response()
}

/// GET /api/intelligence/sre/game-days
pub(crate) async fn api_intelligence_sre_game_days(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::sre_os::build_game_day_plan(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/sre/runbook/execute
pub(crate) async fn api_intelligence_sre_runbook_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::sre_os::RunbookExecuteRequest>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    match crate::intelligence::sre_os::execute_runbook(&app_state.state_path, &policy, body.dry_run).await
    {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/sre/escalation
pub(crate) async fn api_intelligence_sre_escalation(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::sre_os::build_escalation_policies(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/sre/mttr
pub(crate) async fn api_intelligence_sre_mttr(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::sre_os::build_mttr_report(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/interactive
pub(crate) async fn api_intelligence_graph_interactive(
    AxumState(app_state): AxumState<AppState>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let edge_kinds: Vec<String> = query
        .get("edge_kinds")
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let node_kinds: Vec<String> = query
        .get("node_kinds")
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .unwrap_or_default();
    match crate::intelligence::graph_os::build_interactive_graph(
        &app_state.state_path,
        &edge_kinds,
        &node_kinds,
    ) {
        Ok(graph) => ok_json(graph).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/fabric/topology — Application → Runtime → Cluster → Node → resources
pub(crate) async fn api_intelligence_fabric_topology(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::graph_os::build_runtime_fabric_topology(&app_state.state_path) {
        Ok(topology) => ok_json(topology).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/impact
pub(crate) async fn api_intelligence_graph_impact(
    AxumState(app_state): AxumState<AppState>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let workload = query.get("workload").cloned().unwrap_or_else(|| "api".into());
    match crate::intelligence::graph_os::build_impact_analysis(&app_state.state_path, &workload) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/blast-radius
pub(crate) async fn api_intelligence_graph_blast_radius(
    AxumState(app_state): AxumState<AppState>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let workload = query.get("workload").cloned().unwrap_or_else(|| "api".into());
    match crate::intelligence::graph_os::build_blast_radius(&app_state.state_path, &workload) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/graph/import-k8s
pub(crate) async fn api_intelligence_graph_import_k8s(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::graph_os::K8sImportRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::graph_os::import_k8s_services(&app_state.state_path, body.dry_run) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/threat-paths
pub(crate) async fn api_intelligence_graph_threat_paths(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::graph_os::build_threat_paths(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/search
pub(crate) async fn api_intelligence_graph_search(
    AxumState(app_state): AxumState<AppState>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let q = query.get("q").cloned().unwrap_or_default();
    match crate::intelligence::graph_os::search_graph(&app_state.state_path, &q) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/snapshots
pub(crate) async fn api_intelligence_graph_snapshots() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::graph_os::list_graph_snapshots()).into_response()
}

/// POST /api/intelligence/graph/snapshots
pub(crate) async fn api_intelligence_graph_snapshots_capture(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::graph_os::GraphSnapshotCaptureRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::graph_os::capture_graph_snapshot(
        &app_state.state_path,
        body.label.as_deref(),
    ) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/cmdb
pub(crate) async fn api_intelligence_graph_cmdb() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::graph_os::build_cmdb_inventory()).into_response()
}

/// POST /api/intelligence/graph/cmdb/sync
pub(crate) async fn api_intelligence_graph_cmdb_sync(
    Json(body): Json<crate::intelligence::graph_os::CmdbSyncRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::graph_os::sync_cmdb_to_graph(body.dry_run) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/placement
pub(crate) async fn api_intelligence_graph_placement(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::graph_os::build_graph_placement(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/graph/export
pub(crate) async fn api_intelligence_graph_export(
    AxumState(app_state): AxumState<AppState>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let format = query.get("format").cloned().unwrap_or_else(|| "neo4j".into());
    match crate::intelligence::graph_os::export_graph(&app_state.state_path, &format) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/macos/tray-sparkline
pub(crate) async fn api_intelligence_macos_tray_sparkline(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::macos_os::build_tray_sparkline(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/macos/live-activity
pub(crate) async fn api_intelligence_macos_live_activity(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::macos_os::build_live_activity(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/macos/dock-badge
pub(crate) async fn api_intelligence_macos_dock_badge(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::macos_os::build_dock_badge(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/macos/notifications
pub(crate) async fn api_intelligence_macos_notifications(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::macos_os::build_native_notifications(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/macos/spotlight
pub(crate) async fn api_intelligence_macos_spotlight(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::macos_os::build_spotlight_index(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/macos/shortcuts
pub(crate) async fn api_intelligence_macos_shortcuts() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::macos_os::build_shortcuts_manifest()).into_response()
}

/// GET /api/intelligence/macos/menu-extras
pub(crate) async fn api_intelligence_macos_menu_extras(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::macos_os::build_menu_extras(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/macos/offline-cache
pub(crate) async fn api_intelligence_macos_offline_cache_get() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::macos_os::read_offline_cache()).into_response()
}

/// POST /api/intelligence/macos/offline-cache
pub(crate) async fn api_intelligence_macos_offline_cache_write(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::macos_os::write_offline_cache(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/macos/universal-links
pub(crate) async fn api_intelligence_macos_universal_links() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::macos_os::link_registry()).into_response()
}

/// GET /api/intelligence/macos/universal-links/resolve
pub(crate) async fn api_intelligence_macos_universal_links_resolve(
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let url = query.get("url").cloned().unwrap_or_else(|| "aether://command".into());
    ok_json(crate::intelligence::macos_os::resolve_universal_link(&url)).into_response()
}

/// GET /api/intelligence/macos/release-pipeline
pub(crate) async fn api_intelligence_macos_release_pipeline() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::macos_os::build_release_pipeline_status()).into_response()
}

async fn finops_workload_refs(app_state: &AppState) -> Vec<(String, std::path::PathBuf)> {
    let store = app_state.state.read().await;
    store
        .list()
        .iter()
        .map(|ws| (ws.name.clone(), ws.spec_path.clone()))
        .collect()
}

/// GET /api/intelligence/finops/chargeback
pub(crate) async fn api_intelligence_finops_chargeback(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let rows = finops_workload_refs(&app_state).await;
    let refs: Vec<(&str, &std::path::PathBuf)> = rows.iter().map(|(n, p)| (n.as_str(), p)).collect();
    match crate::intelligence::finops_os::build_chargeback_automation(&refs) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/finops/spot-advisor
pub(crate) async fn api_intelligence_finops_spot_advisor(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::finops_os::build_spot_advisor(&pairs)).into_response()
}

/// GET /api/intelligence/finops/reserved-planner
pub(crate) async fn api_intelligence_finops_reserved_planner(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::finops_os::build_reserved_instance_planner(&pairs)).into_response()
}

/// GET /api/intelligence/finops/anomalies
pub(crate) async fn api_intelligence_finops_anomalies(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let rows = finops_workload_refs(&app_state).await;
    let refs: Vec<(&str, &std::path::PathBuf)> = rows.iter().map(|(n, p)| (n.as_str(), p)).collect();
    match crate::intelligence::finops_os::detect_cost_anomalies(&refs) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/finops/unit-economics
pub(crate) async fn api_intelligence_finops_unit_economics(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::finops_os::build_unit_economics(&pairs)).into_response()
}

/// POST /api/intelligence/finops/execute
pub(crate) async fn api_intelligence_finops_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::finops_os::FinOpsExecuteRequest>,
) -> impl axum::response::IntoResponse {
    let config = Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::finops_os::execute_finops_agent(
        &pairs, &policy, &body,
    ))
    .into_response()
}

/// GET /api/intelligence/finops/multicloud-compare
pub(crate) async fn api_intelligence_finops_multicloud_compare(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::finops_os::build_multicloud_cost_compare(&pairs)).into_response()
}

/// GET /api/intelligence/finops/carbon
pub(crate) async fn api_intelligence_finops_carbon(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::finops_os::build_carbon_footprint(&pairs)).into_response()
}

/// POST /api/intelligence/finops/budget-webhook
pub(crate) async fn api_intelligence_finops_budget_webhook(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::finops_os::BudgetWebhookRequest>,
) -> impl axum::response::IntoResponse {
    let rows = finops_workload_refs(&app_state).await;
    let refs: Vec<(&str, &std::path::PathBuf)> = rows.iter().map(|(n, p)| (n.as_str(), p)).collect();
    match crate::intelligence::finops_os::dispatch_budget_webhook(&refs, &body) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/finops/trends
pub(crate) async fn api_intelligence_finops_trends(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let rows = finops_workload_refs(&app_state).await;
    let refs: Vec<(&str, &std::path::PathBuf)> = rows.iter().map(|(n, p)| (n.as_str(), p)).collect();
    let pairs = workload_pairs(&app_state).await;
    match crate::intelligence::finops_os::build_finops_trends(&refs, &pairs) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

fn security_state_dir(app_state: &AppState) -> std::path::PathBuf {
    app_state
        .state_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| crate::resources::aether_path(""))
}

async fn confidential_workload_triples(
    app_state: &AppState,
) -> Vec<(String, Workload, String)> {
    let store = app_state.state.read().await;
    store
        .list()
        .iter()
        .filter_map(|w| {
            Workload::from_file(&w.spec_path)
                .ok()
                .filter(|s| s.confidential.as_ref().is_some_and(|c| c.enabled))
                .map(|s| (w.name.clone(), s, w.runtime.to_string()))
        })
        .collect()
}

/// POST /api/intelligence/security/policy-apply
pub(crate) async fn api_intelligence_security_policy_apply(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::security_os::PolicyAutoApplyRequest>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::security_os::apply_security_policies(&pairs, &body)).into_response()
}

/// GET /api/intelligence/security/sbom-drift
pub(crate) async fn api_intelligence_security_sbom_drift() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::security_os::detect_sbom_drift()).into_response()
}

/// GET /api/intelligence/security/confidential-fleet
pub(crate) async fn api_intelligence_security_confidential_fleet(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let svc = crate::ragnarok::attestation::AttestationService::new(security_state_dir(&app_state));
    let catalog = crate::ragnarok::image::ImageCatalog::load(&security_state_dir(&app_state));
    let triples = confidential_workload_triples(&app_state).await;
    let refs: Vec<(&str, &Workload, &str)> = triples
        .iter()
        .map(|(n, s, r)| (n.as_str(), s, r.as_str()))
        .collect();
    ok_json(crate::intelligence::security_os::build_confidential_fleet_dashboard(
        &refs, &svc, &catalog,
    ))
    .into_response()
}

/// GET /api/intelligence/security/zero-trust-wizard
pub(crate) async fn api_intelligence_security_zero_trust_wizard(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::security_os::build_zero_trust_wizard(&pairs)).into_response()
}

/// GET /api/intelligence/security/compliance-report
pub(crate) async fn api_intelligence_security_compliance_report(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::security_os::build_compliance_report(&pairs)).into_response()
}

/// POST /api/intelligence/security/rotation-agent
pub(crate) async fn api_intelligence_security_rotation_agent(
    Json(body): Json<crate::intelligence::security_os::SecretRotationAgentRequest>,
) -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::security_os::run_secret_rotation_agent(&body)).into_response()
}

/// GET /api/intelligence/security/image-enforcement
pub(crate) async fn api_intelligence_security_image_enforcement(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    let catalog = crate::ragnarok::image::ImageCatalog::load(&security_state_dir(&app_state));
    ok_json(crate::intelligence::security_os::build_image_signing_enforcement(
        &pairs, &catalog,
    ))
    .into_response()
}

/// POST /api/intelligence/security/threat-hunt
pub(crate) async fn api_intelligence_security_threat_hunt(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::security_os::ThreatHuntRequest>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(
        crate::intelligence::security_os::run_threat_hunt(&pairs, &body).await,
    )
    .into_response()
}

/// GET /api/intelligence/security/sovereign-audit
pub(crate) async fn api_intelligence_security_sovereign_audit_get() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::security_os::read_sovereign_audit(50)).into_response()
}

/// POST /api/intelligence/security/sovereign-audit
pub(crate) async fn api_intelligence_security_sovereign_audit_append(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    match crate::intelligence::security_os::append_sovereign_audit(&pairs) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/security/score-trend
pub(crate) async fn api_intelligence_security_score_trend(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::security_os::build_security_score_trend(&pairs)).into_response()
}

/// GET /v1/intelligence/briefing
pub(crate) async fn api_v1_intelligence_briefing(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    api_command_center_briefing(AxumState(app_state)).await.into_response()
}

/// GET /v1/intelligence/threats
pub(crate) async fn api_v1_intelligence_threats(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    api_intelligence_threats(AxumState(app_state)).await.into_response()
}

/// GET /v1/intelligence/cost-optimize
pub(crate) async fn api_v1_intelligence_cost_optimize(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    api_intelligence_cost_optimize(AxumState(app_state)).await.into_response()
}

/// GET /v1/intelligence/predictions
pub(crate) async fn api_v1_intelligence_predictions(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    api_intelligence_predictions(AxumState(app_state)).await.into_response()
}

/// GET /v1/intelligence/autonomy
pub(crate) async fn api_v1_intelligence_autonomy(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::autonomy::build_autonomy_status(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/platform/saas-tenants
pub(crate) async fn api_intelligence_platform_saas_tenants(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::platform_os::build_saas_tenant_dashboard(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/platform/plugin-marketplace
pub(crate) async fn api_intelligence_platform_plugin_marketplace() -> impl axum::response::IntoResponse {
    match crate::intelligence::platform_os::build_plugin_marketplace() {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/platform/helm-v2
pub(crate) async fn api_intelligence_platform_helm_v2(
    Json(body): Json<crate::intelligence::platform_os::HelmAiV2Request>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::platform_os::build_helm_ai_v2(&body).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/platform/terraform-export
pub(crate) async fn api_intelligence_platform_terraform_export(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    let Some((spec, _)) = pairs.into_iter().next() else {
        return err_internal::<serde_json::Value>("no workloads in fleet".to_string()).into_response();
    };
    ok_json(crate::intelligence::platform_os::build_terraform_export(&spec)).into_response()
}

/// GET /api/intelligence/platform/pulumi-bridge
pub(crate) async fn api_intelligence_platform_pulumi_bridge(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    let Some((spec, _)) = pairs.into_iter().next() else {
        return err_internal::<serde_json::Value>("no workloads in fleet".to_string()).into_response();
    };
    ok_json(crate::intelligence::platform_os::build_pulumi_bridge(&spec)).into_response()
}

/// GET /api/intelligence/platform/public-api
pub(crate) async fn api_intelligence_platform_public_api() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::platform_os::build_public_api_manifest()).into_response()
}

/// GET /api/intelligence/platform/mobile-companion
pub(crate) async fn api_intelligence_platform_mobile_companion() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::platform_os::build_mobile_companion_manifest()).into_response()
}

/// GET /api/intelligence/platform/ide-extensions
pub(crate) async fn api_intelligence_platform_ide_extensions() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::platform_os::build_ide_extension_manifest()).into_response()
}

/// GET /api/intelligence/platform/community-intents
pub(crate) async fn api_intelligence_platform_community_intents() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::platform_os::build_community_intent_library()).into_response()
}

/// GET /api/intelligence/platform/autonomous-sre
pub(crate) async fn api_intelligence_platform_autonomous_sre(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::platform_os::build_autonomous_sre_status(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/platform/autonomous-sre/execute
pub(crate) async fn api_intelligence_platform_autonomous_sre_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::platform_os::AutonomousSreExecuteRequest>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    match crate::intelligence::platform_os::execute_autonomous_sre_loop(
        &app_state.state_path,
        &pairs,
        &body,
    )
    .await
    {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/labs/overview
pub(crate) async fn api_intelligence_labs_overview() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::labs_os::build_labs_graduation_overview()).into_response()
}

/// POST /api/intelligence/labs/terraform-export
pub(crate) async fn api_intelligence_labs_terraform_export(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::labs_os::LabsTerraformRequest>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    match crate::intelligence::labs_os::build_labs_terraform_export(&pairs, &body).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/labs/pulumi-bridge
pub(crate) async fn api_intelligence_labs_pulumi_bridge(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::labs_os::LabsPulumiRequest>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    match crate::intelligence::labs_os::build_labs_pulumi_bridge(&pairs, &body).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/labs/mobile-companion
pub(crate) async fn api_intelligence_labs_mobile_companion() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::labs_os::build_labs_mobile_companion()).into_response()
}

/// GET /api/intelligence/labs/ide-extensions
pub(crate) async fn api_intelligence_labs_ide_extensions() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::labs_os::build_labs_ide_extensions()).into_response()
}

/// GET /api/intelligence/labs/community-intents
pub(crate) async fn api_intelligence_labs_community_intents() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::labs_os::build_labs_community_intents()).into_response()
}

/// POST /api/intelligence/labs/community-intents/import
pub(crate) async fn api_intelligence_labs_community_intents_import(
    Json(body): Json<crate::intelligence::labs_os::LabsCommunityIntentImportRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::labs_os::import_community_intent(&body) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => super::handlers::err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/labs/carbon
pub(crate) async fn api_intelligence_labs_carbon(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::labs_os::build_labs_carbon_report(&pairs)).into_response()
}

/// GET /api/intelligence/labs/compliance-report
pub(crate) async fn api_intelligence_labs_compliance_report(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let pairs = workload_pairs(&app_state).await;
    ok_json(crate::intelligence::labs_os::build_labs_compliance_report(&pairs)).into_response()
}

/// GET /api/intelligence/labs/voice-copilot
pub(crate) async fn api_intelligence_labs_voice_copilot() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::labs_os::build_labs_voice_copilot()).into_response()
}

/// GET /api/intelligence/labs/graph-export
pub(crate) async fn api_intelligence_labs_graph_export(
    AxumState(app_state): AxumState<AppState>,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let format = query.get("format").cloned().unwrap_or_else(|| "neo4j".into());
    match crate::intelligence::labs_os::build_labs_graph_export(&app_state.state_path, &format) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/extensions/overview
pub(crate) async fn api_intelligence_extensions_overview() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::extensions_os::build_extensions_graduation_overview()).into_response()
}

/// GET /api/intelligence/extensions/chaos/experiments
pub(crate) async fn api_intelligence_extensions_chaos_experiments(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::extensions_os::build_ship_chaos_catalog(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/extensions/chaos/run
pub(crate) async fn api_intelligence_extensions_chaos_run(
    Json(body): Json<crate::intelligence::sre_os::ChaosRunRequest>,
) -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::extensions_os::run_ship_chaos(
        body.dry_run,
        &body.experiment_id,
    ))
    .into_response()
}

/// GET /api/intelligence/extensions/game-days
pub(crate) async fn api_intelligence_extensions_game_days(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::extensions_os::build_ship_game_days(&app_state.state_path).await {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// POST /api/intelligence/extensions/game-days/execute
pub(crate) async fn api_intelligence_extensions_game_days_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<crate::intelligence::extensions_os::GameDayExecuteRequest>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::extensions_os::execute_game_day_scenario(&app_state.state_path, &body)
        .await
    {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => super::handlers::err_bad_request::<serde_json::Value>(&e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/extensions/live-activity
pub(crate) async fn api_intelligence_extensions_live_activity(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::extensions_os::build_ship_live_activity(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/extensions/spotlight
pub(crate) async fn api_intelligence_extensions_spotlight(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::extensions_os::build_ship_spotlight(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/extensions/shortcuts
pub(crate) async fn api_intelligence_extensions_shortcuts() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::extensions_os::build_ship_shortcuts()).into_response()
}

/// GET /api/intelligence/extensions/menu-extras
pub(crate) async fn api_intelligence_extensions_menu_extras(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::extensions_os::build_ship_menu_extras(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/extensions/native-bundle
pub(crate) async fn api_intelligence_extensions_native_bundle(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::extensions_os::build_native_extensions_bundle(&app_state.state_path) {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

/// GET /api/intelligence/extensions/sre-bundle
pub(crate) async fn api_intelligence_extensions_sre_bundle(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    match crate::intelligence::extensions_os::build_sre_extensions_bundle(&app_state.state_path).await
    {
        Ok(report) => ok_json(report).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e.to_string()).into_response(),
    }
}

async fn production_runtime_snapshot(
    app_state: &AppState,
) -> crate::intelligence::production_os::ProductionRuntimeSnapshot {
    crate::intelligence::production_os::ProductionRuntimeSnapshot {
        postgres_required: app_state.workload_state_pg.is_some(),
        postgres_ok: if let Some(ref pg) = app_state.workload_state_pg {
            pg.ping_ok().await
        } else {
            true
        },
        redis_required: app_state.shared_cache.uses_redis(),
        redis_ok: app_state.shared_cache.redis_ping_ok().await,
        tls_active: app_state.tls_active,
    }
}

/// GET /api/intelligence/production/overview
pub(crate) async fn api_intelligence_production_overview() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::production_os::build_production_overview()).into_response()
}

/// GET /api/intelligence/production/scorecard
pub(crate) async fn api_intelligence_production_scorecard(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let snap = production_runtime_snapshot(&app_state).await;
    ok_json(crate::intelligence::production_os::build_production_scorecard(&snap)).into_response()
}

/// GET /api/intelligence/production/auth-plane
pub(crate) async fn api_intelligence_production_auth_plane(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::production_os::build_auth_plane_report(
        app_state.saml.is_some(),
    ))
    .into_response()
}

/// GET /api/intelligence/production/opa-plane
pub(crate) async fn api_intelligence_production_opa_plane() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::production_os::build_opa_plane_report()).into_response()
}

/// GET /api/intelligence/production/ha-plane
pub(crate) async fn api_intelligence_production_ha_plane(
    AxumState(app_state): AxumState<AppState>,
) -> impl axum::response::IntoResponse {
    let snap = production_runtime_snapshot(&app_state).await;
    ok_json(crate::intelligence::production_os::build_ha_plane_report(&snap)).into_response()
}

/// GET /api/intelligence/production/durability-plane
pub(crate) async fn api_intelligence_production_durability_plane() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::production_os::build_durability_plane_report()).into_response()
}

/// GET /api/intelligence/production/hosted-plane
pub(crate) async fn api_intelligence_production_hosted_plane() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::production_os::build_hosted_plane_report()).into_response()
}

/// GET /api/intelligence/production/edge-fleet
pub(crate) async fn api_intelligence_production_edge_fleet() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::production_os::build_edge_fleet_plane_report()).into_response()
}

/// GET /api/intelligence/production/post-deploy-manifest
pub(crate) async fn api_intelligence_production_post_deploy_manifest() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::production_os::build_post_deploy_manifest()).into_response()
}

/// GET /api/intelligence/production/ci-smoke-manifest
pub(crate) async fn api_intelligence_production_ci_smoke_manifest() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::production_os::build_ci_smoke_manifest()).into_response()
}

/// GET /api/intelligence/livelabs/overview
pub(crate) async fn api_intelligence_livelabs_overview() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_livelabs_overview()).into_response()
}

/// GET /api/intelligence/livelabs/reference-runner
pub(crate) async fn api_intelligence_livelabs_reference_runner() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_reference_runner_report()).into_response()
}

/// GET /api/intelligence/livelabs/kind-fixture
pub(crate) async fn api_intelligence_livelabs_kind_fixture() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_kind_fixture_report()).into_response()
}

/// GET /api/intelligence/livelabs/live-smoke
pub(crate) async fn api_intelligence_livelabs_live_smoke() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_live_smoke_report()).into_response()
}

/// GET /api/intelligence/livelabs/post-deploy-verify
pub(crate) async fn api_intelligence_livelabs_post_deploy_verify() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_post_deploy_verify_report()).into_response()
}

/// GET /api/intelligence/livelabs/kubernetes-lab
pub(crate) async fn api_intelligence_livelabs_kubernetes_lab() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_kubernetes_lab_report()).into_response()
}

/// GET /api/intelligence/livelabs/advanced-runtime-labs
pub(crate) async fn api_intelligence_livelabs_advanced_runtime_labs() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_advanced_runtime_labs_report()).into_response()
}

/// GET /api/intelligence/livelabs/ci-pipeline
pub(crate) async fn api_intelligence_livelabs_ci_pipeline() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_ci_pipeline_report()).into_response()
}

/// GET /api/intelligence/livelabs/cluster-exec
pub(crate) async fn api_intelligence_livelabs_cluster_exec() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_cluster_exec_report()).into_response()
}

/// GET /api/intelligence/livelabs/confidential-lab
pub(crate) async fn api_intelligence_livelabs_confidential_lab() -> impl axum::response::IntoResponse {
    ok_json(crate::intelligence::livelabs_os::build_confidential_lab_report()).into_response()
}
