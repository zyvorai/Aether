// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Autonomous intelligence layer — learning, prediction, healing, and evolution.

pub mod actions;
pub mod agents;
pub mod anomaly;
pub mod autonomy;
pub mod briefing;
pub mod capacity;
pub mod context;
pub mod copilot_os;
pub mod evolution;
pub mod extensions_os;
pub mod federation_os;
pub mod finops;
pub mod finops_os;
pub mod gitops_agent;
pub mod graph;
pub mod graph_os;
pub mod healer;
pub mod intent_os;
pub mod labs_os;
pub mod livelabs_os;
pub mod macos_os;
pub mod metrics;
pub mod multicloud;
pub mod notifications;
pub mod pipeline;
pub mod placement;
pub mod platform_os;
pub mod policy;
pub mod predict;
pub mod production_os;
pub mod profile;
pub mod record;
pub mod remediation;
pub mod security;
pub mod security_os;
pub mod sre;
pub mod sre_os;
pub mod store;
pub mod twin;
pub mod zyra_os;

pub use actions::{build_next_actions, NextAction, NextActionsReport};
pub use agents::{build_agent_registry, AgentRegistryReport, AgentStatusEntry};
pub use autonomy::{build_autonomy_status, AutonomyStatusReport};
pub use briefing::{build_command_center_briefing, CommandCenterBriefing};
pub use capacity::{
    build_scale_suggestions, execute_scale_suggestions, CapacityScaleExecuteReport,
    CapacityScaleExecuteRequest, CapacityScaleReport, ScaleSuggestion,
};
pub use context::build_context_snapshot;
pub use copilot_os::{
    append_copilot_audit, build_copilot_rbac_scopes, build_voice_copilot_lab, read_copilot_audit,
    read_copilot_memory, route_copilot_agent, write_copilot_memory_entry, CopilotAuditEntry,
    CopilotAuditReport, CopilotMemoryEntry, CopilotMemoryReport, CopilotRbacScopesReport,
    CopilotToolScope, VoiceCopilotLabReport,
};
pub use evolution::{
    execute_evolution, EvolutionEngine, EvolutionExecuteReport, EvolutionExecuteRequest,
    EvolutionStatus,
};
pub use extensions_os::{
    build_extensions_graduation_overview, build_native_extensions_bundle, build_ship_chaos_catalog,
    build_ship_game_days, build_ship_live_activity, build_ship_menu_extras, build_ship_shortcuts,
    build_ship_spotlight, build_sre_extensions_bundle, execute_game_day_scenario, run_ship_chaos,
    ExtensionsGraduatedFeature, ExtensionsGraduationOverview, GameDayExecuteReport,
    GameDayExecuteRequest, NativeExtensionsBundle, ShipChaosCatalogReport, ShipChaosRunReport,
    ShipGameDayReport, ShipLiveActivityReport, ShipMenuExtrasReport, ShipShortcutsReport,
    ShipSpotlightReport, SreExtensionsBundle,
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
pub use graph::{build_knowledge_graph, KnowledgeGraphReport};
pub use graph_os::{
    build_blast_radius, build_cmdb_inventory, build_graph_placement, build_impact_analysis,
    build_interactive_graph, build_runtime_fabric_topology, build_threat_paths,
    capture_graph_snapshot, export_graph, import_k8s_services, list_graph_snapshots, search_graph,
    sync_cmdb_to_graph, BlastRadiusReport, CmdbSyncReport, CmdbSyncRequest, FabricTopologyReport,
    GraphExportReport, GraphImpactReport, GraphPlacementReport, GraphSearchReport,
    GraphSnapshotCaptureReport, GraphSnapshotCaptureRequest, GraphSnapshotsReport,
    InteractiveGraphRequest, K8sImportReport, K8sImportRequest, ThreatPathsReport,
};
pub use healer::{
    build_healer_preview, execute_healer, execute_orchestrator_actions, HealerExecuteReport,
    HealerPreviewReport, HealerResult,
};
pub use intent_os::{
    build_intent_bundle, build_intent_gitops_diff, build_intent_sla_breaches,
    check_compliance_gate, deploy_intent_pipeline, enforce_intent_budget, list_intent_templates,
    list_intent_versions, parse_nl_intent, record_intent_version, rollback_intent_version,
    scan_intent_violations, BudgetEnforceReport, BudgetEnforceRequest, ComplianceGateReport,
    IntentBundleReport, IntentBundleRequest, IntentDeployReport, IntentDeployRequest,
    IntentGitOpsDiffReport, IntentRollbackReport, IntentRollbackRequest, IntentSlaBreach,
    IntentSlaBreachesReport, IntentTemplateLibrary, IntentVersionHistory, IntentViolationEntry,
    IntentViolationsReport, NlIntentReport, NlIntentRequest,
};
pub use labs_os::{
    build_labs_carbon_report, build_labs_community_intents, build_labs_compliance_report,
    build_labs_graduation_overview, build_labs_graph_export, build_labs_ide_extensions,
    build_labs_mobile_companion, build_labs_pulumi_bridge, build_labs_terraform_export,
    build_labs_voice_copilot, built_in_intent_library, import_community_intent, LabsCarbonReport,
    LabsCarbonWorkloadLine, LabsCommunityIntentImportRequest, LabsCommunityIntentReport,
    LabsComplianceReport, LabsGraduatedFeature, LabsGraduationOverview, LabsGraphExportReport,
    LabsIdeExtensionReport, LabsMobileCompanionReport, LabsPulumiReport, LabsPulumiRequest,
    LabsTerraformReport, LabsTerraformRequest, LabsVoiceCopilotReport,
};
pub use livelabs_os::{
    build_advanced_runtime_labs_report, build_ci_pipeline_report, build_cluster_exec_report,
    build_kind_fixture_report, build_kubernetes_lab_report, build_live_smoke_report,
    build_livelabs_overview, build_post_deploy_verify_report, build_reference_runner_report,
    AdvancedRuntimeLabReport, CiPipelineReport, ClusterExecReport, KindFixtureReport,
    KubernetesLabReport, LiveLabsFeature, LiveLabsOverview, LiveSmokeReport,
    PostDeployVerifyReport, ReferenceRunnerReport,
};
pub use macos_os::{
    build_dock_badge, build_live_activity, build_menu_extras, build_native_notifications,
    build_release_pipeline_status, build_shortcuts_manifest, build_spotlight_index,
    build_tray_sparkline, link_registry, read_offline_cache, resolve_universal_link,
    write_offline_cache, DockBadgeReport, LiveActivityReport, MenuExtrasReport,
    NativeNotificationsReport, OfflineCacheReport, ReleasePipelineReport, ShortcutsManifestReport,
    SpotlightIndexReport, TraySparklineReport, UniversalLinkRegistry, UniversalLinkResolveReport,
};
pub use multicloud::{build_multicloud_posture, MultiCloudPostureReport};
pub use notifications::{
    build_critical_notifications, CriticalNotification, CriticalNotificationsReport,
};
pub use pipeline::{
    build_autonomous_placement, build_intent_pipeline, IntentPipelineReport, IntentPipelineRequest,
};
pub use placement::{GlobalPlacementEngine, PlacementRecommendation};
pub use platform_os::{
    build_autonomous_sre_status, build_community_intent_library, build_helm_ai_v2,
    build_ide_extension_manifest, build_mobile_companion_manifest, build_plugin_marketplace,
    build_public_api_manifest, build_pulumi_bridge, build_saas_tenant_dashboard,
    build_terraform_export, execute_autonomous_sre_loop, AutonomousSreExecuteReport,
    AutonomousSreExecuteRequest, AutonomousSreStatusReport, CommunityIntentEntry,
    CommunityIntentLibraryReport, HelmAiV2Report, HelmAiV2Request, IdeExtensionEntry,
    IdeExtensionManifest, MobileCompanionReport, PluginMarketplaceEntry, PluginMarketplaceReport,
    PublicApiManifest, PublicApiRoute, PulumiBridgeReport, SaasTenantDashboardReport,
    SaasTenantLine, TerraformExportReport,
};
pub use policy::{AutonomyPolicy, AutonomyTier};
pub use predict::{FailurePredictor, PredictionReport, WorkloadPrediction};
pub use production_os::{
    build_auth_plane_report, build_ci_smoke_manifest, build_durability_plane_report,
    build_edge_fleet_plane_report, build_ha_plane_report, build_hosted_plane_report,
    build_opa_plane_report, build_post_deploy_manifest, build_production_overview,
    build_production_scorecard, AuthPlaneReport, CiSmokeManifest, DurabilityPlaneReport,
    EdgeFleetPlaneReport, HaPlaneReport, HostedPlaneReport, OpaPlaneReport, PostDeployManifest,
    ProductionCheck, ProductionFeature, ProductionOverview, ProductionRuntimeSnapshot,
    ProductionScorecard,
};
pub use profile::{BehaviorProfiler, WorkloadBehaviorProfile};
pub use record::{record_deployment_outcome, record_migration_outcome};
pub use security::{
    SecurityCopilotReport, SecurityEngine, SecurityRemediateReport, SecurityRemediateRequest,
    ThreatReport,
};
pub use security_os::{
    append_sovereign_audit, apply_security_policies, build_compliance_report,
    build_security_score_trend, build_zero_trust_wizard, detect_sbom_drift, read_sovereign_audit,
    run_secret_rotation_agent, run_threat_hunt, ComplianceReport, ComplianceReportSection,
    PolicyAutoApplyReport, PolicyAutoApplyRequest, SbomDriftAlert, SbomDriftReport,
    SecretRotationAction, SecretRotationAgentReport, SecretRotationAgentRequest,
    SecurityScoreTrendPoint, SecurityScoreTrendReport, SovereignAuditEntry, SovereignAuditReport,
    ThreatHuntFinding, ThreatHuntReport, ThreatHuntRequest, ZeroTrustWizardReport,
    ZeroTrustWizardStep,
};
pub use sre::{build_sre_runbook, SreRunbookReport};
pub use sre_os::{
    build_chaos_catalog, build_error_budget_dashboard, build_escalation_policies,
    build_game_day_plan, build_incident_timeline, build_mttr_report, build_on_call_status,
    build_postmortem, build_runbook_schedule, execute_runbook, run_chaos_experiment,
    test_on_call_webhook, ChaosExperimentCatalog, ChaosRunReport, ChaosRunRequest,
    ErrorBudgetDashboardReport, EscalationPolicyReport, GameDayPlanReport, IncidentTimelineReport,
    MttrReport, OnCallIntegrationReport, OnCallTestReport, OnCallTestRequest, PostmortemReport,
    RunbookExecuteReport, RunbookExecuteRequest, SreRunbookScheduleReport,
};
pub use store::IntelligenceStore;
pub use twin::{DigitalTwinEngine, TwinSimulateReport, TwinSimulateRequest};
pub use zyra_os::{
    append_zyra_audit, author_runbook, build_llm_provider_status, build_voice_zyra_lab,
    build_zyra_insights, build_zyra_rbac_scopes, explain_policy_violations, read_zyra_audit,
    read_zyra_memory, route_zyra_agent, write_zyra_memory_entry, BatchConfirmReport,
    LlmProviderStatusReport, MultiAgentRouteReport, PolicyExplainerReport, PolicyExplainerRequest,
    RunbookAuthorReport, RunbookAuthorRequest, VoiceZyraLabReport, ZyraAuditEntry, ZyraAuditReport,
    ZyraInsightsReport, ZyraMemoryEntry, ZyraMemoryReport, ZyraMemorySettings,
    ZyraRbacScopesReport, ZyraToolScope,
};
