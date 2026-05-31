// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Autonomous intelligence layer — learning, prediction, healing, and evolution.

pub mod briefing;
pub mod context;
pub mod evolution;
pub mod finops;
pub mod finops_os;
pub mod healer;
pub mod metrics;
pub mod placement;
pub mod actions;
pub mod agents;
pub mod anomaly;
pub mod autonomy;
pub mod capacity;
pub mod copilot_os;
pub mod federation_os;
pub mod gitops_agent;
pub mod multicloud;
pub mod notifications;
pub mod intent_os;
pub mod macos_os;
pub mod pipeline;
pub mod policy;
pub mod predict;
pub mod profile;
pub mod record;
pub mod remediation;
pub mod security;
pub mod sre;
pub mod sre_os;
pub mod store;
pub mod twin;
pub mod graph;
pub mod graph_os;

pub use actions::{build_next_actions, NextAction, NextActionsReport};
pub use agents::{build_agent_registry, AgentRegistryReport, AgentStatusEntry};
pub use autonomy::{build_autonomy_status, AutonomyStatusReport};
pub use briefing::{build_command_center_briefing, CommandCenterBriefing};
pub use capacity::{
    build_scale_suggestions, execute_scale_suggestions, CapacityScaleExecuteReport,
    CapacityScaleExecuteRequest, CapacityScaleReport, ScaleSuggestion,
};
pub use context::build_context_snapshot;
pub use evolution::{
    EvolutionEngine, EvolutionExecuteReport, EvolutionExecuteRequest, EvolutionStatus,
    execute_evolution,
};
pub use copilot_os::{
    append_copilot_audit, author_runbook, build_copilot_rbac_scopes, build_llm_provider_status,
    build_voice_copilot_lab, explain_policy_violations, read_copilot_audit, read_copilot_memory,
    route_copilot_agent, write_copilot_memory_entry, BatchConfirmReport, CopilotAuditEntry,
    CopilotAuditReport, CopilotMemoryEntry, CopilotMemoryReport, CopilotRbacScopesReport,
    CopilotToolScope, LlmProviderStatusReport, MultiAgentRouteReport, PolicyExplainerReport,
    PolicyExplainerRequest, RunbookAuthorReport, RunbookAuthorRequest, VoiceCopilotLabReport,
};
pub use federation_os::{
    apply_packetwolf_guard, build_cloud_account_vault, build_cluster_health_mesh,
    build_cost_arbitrage, build_geo_placement, build_migration_wave, build_packetwolf_guard,
    build_region_lock_posture, build_unified_fabric, build_volume_replication_status,
    execute_federation_sync, CloudAccountVaultReport, ClusterHealthMeshReport, CostArbitrageReport,
    FederationExecuteReport, FederationExecuteRequest, GeoPlacementReport, MigrationWaveReport,
    PacketWolfGuardApplyReport, PacketWolfGuardApplyRequest, PacketWolfGuardReport,
    RegionLockReport, UnifiedFabricReport, VolumeReplicationStatusReport,
};
pub use finops::{
    CostApplyPatch, CostApplyReport, CostApplyRequest, CostOptimizeReport, FinOpsEngine,
};
pub use finops_os::{
    build_carbon_footprint, build_chargeback_automation, build_finops_trends,
    build_multicloud_cost_compare, build_reserved_instance_planner, build_spot_advisor,
    build_unit_economics, detect_cost_anomalies, dispatch_budget_webhook, execute_finops_agent,
    BudgetWebhookReport, BudgetWebhookRequest, CarbonFootprintEntry, CarbonFootprintReport,
    ChargebackAutomationLine, ChargebackAutomationReport, CostAnomalyEntry, CostAnomalyReport,
    FinOpsExecuteReport, FinOpsExecuteRequest, FinOpsTrendPoint, FinOpsTrendsReport,
    MulticloudCostCompareReport, MulticloudCostRow, ReservedInstancePlannerReport,
    ReservedInstanceRecommendation, SpotAdvisorEntry, SpotAdvisorReport, UnitEconomicsEntry,
    UnitEconomicsReport,
};
pub use gitops_agent::{
    build_gitops_agent_plan, execute_gitops_agent, GitOpsAgentExecuteReport,
    GitOpsAgentExecuteRequest, GitOpsAgentSyncReport,
};
pub use macos_os::{
    build_dock_badge, build_live_activity, build_menu_extras, build_native_notifications,
    build_release_pipeline_status, build_shortcuts_manifest, build_spotlight_index, build_tray_sparkline,
    link_registry, read_offline_cache, resolve_universal_link, write_offline_cache, DockBadgeReport,
    LiveActivityReport, MenuExtrasReport, NativeNotificationsReport, OfflineCacheReport,
    ReleasePipelineReport, ShortcutsManifestReport, SpotlightIndexReport, TraySparklineReport,
    UniversalLinkRegistry, UniversalLinkResolveReport,
};
pub use graph::{build_knowledge_graph, KnowledgeGraphReport};
pub use graph_os::{
    build_blast_radius, build_cmdb_inventory, build_graph_placement, build_impact_analysis,
    build_interactive_graph, build_threat_paths, capture_graph_snapshot, export_graph,
    import_k8s_services, list_graph_snapshots, search_graph, sync_cmdb_to_graph, BlastRadiusReport,
    CmdbSyncReport, CmdbSyncRequest, GraphExportReport, GraphImpactReport, GraphPlacementReport,
    GraphSearchReport, GraphSnapshotCaptureReport, GraphSnapshotCaptureRequest, GraphSnapshotsReport,
    InteractiveGraphRequest, K8sImportReport, K8sImportRequest, ThreatPathsReport,
};
pub use healer::{
    build_healer_preview, execute_healer, execute_orchestrator_actions, HealerExecuteReport,
    HealerPreviewReport, HealerResult,
};
pub use multicloud::{build_multicloud_posture, MultiCloudPostureReport};
pub use notifications::{build_critical_notifications, CriticalNotification, CriticalNotificationsReport};
pub use intent_os::{
    build_intent_bundle, build_intent_gitops_diff, build_intent_sla_breaches, check_compliance_gate,
    deploy_intent_pipeline, enforce_intent_budget, list_intent_templates, list_intent_versions,
    parse_nl_intent, record_intent_version, rollback_intent_version, scan_intent_violations,
    BudgetEnforceReport, BudgetEnforceRequest, ComplianceGateReport, IntentBundleReport,
    IntentBundleRequest, IntentDeployReport, IntentDeployRequest, IntentGitOpsDiffReport,
    IntentRollbackReport, IntentRollbackRequest, IntentSlaBreachesReport, IntentSlaBreach,
    IntentTemplateLibrary, IntentVersionHistory, IntentViolationEntry, IntentViolationsReport,
    NlIntentReport, NlIntentRequest,
};
pub use pipeline::{build_autonomous_placement, build_intent_pipeline, IntentPipelineReport, IntentPipelineRequest};
pub use placement::{GlobalPlacementEngine, PlacementRecommendation};
pub use policy::{AutonomyPolicy, AutonomyTier};
pub use predict::{FailurePredictor, PredictionReport, WorkloadPrediction};
pub use profile::{BehaviorProfiler, WorkloadBehaviorProfile};
pub use record::{record_deployment_outcome, record_migration_outcome};
pub use security::{
    SecurityCopilotReport, SecurityEngine, SecurityRemediateReport, SecurityRemediateRequest,
    ThreatReport,
};
pub use sre::{build_sre_runbook, SreRunbookReport};
pub use sre_os::{
    build_chaos_catalog, build_error_budget_dashboard, build_escalation_policies, build_game_day_plan,
    build_incident_timeline, build_mttr_report, build_on_call_status, build_postmortem, build_runbook_schedule,
    execute_runbook, run_chaos_experiment, test_on_call_webhook, ChaosExperimentCatalog, ChaosRunReport,
    ChaosRunRequest, ErrorBudgetDashboardReport, EscalationPolicyReport, GameDayPlanReport,
    IncidentTimelineReport, MttrReport, OnCallIntegrationReport, OnCallTestReport, OnCallTestRequest,
    PostmortemReport, RunbookExecuteReport, RunbookExecuteRequest, SreRunbookScheduleReport,
};
pub use store::IntelligenceStore;
pub use twin::{DigitalTwinEngine, TwinSimulateReport, TwinSimulateRequest};
