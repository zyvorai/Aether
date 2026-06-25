// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// ─── API Response Wrapper ────────────────────────────────────────────
export interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}

// ─── Workloads ───────────────────────────────────────────────────────
export interface WorkloadResponse {
  name: string;
  runtime: string;
  image: string;
  status: string;
  created_at: string;
  source?: string | null;
  cluster?: string | null;
  namespace?: string | null;
  kind?: string | null;
}

export interface ClusterInfo {
  name: string;
  server: string | null;
  version: string | null;
  reachable: boolean;
}

export interface ClusterSummary {
  enabled: boolean;
  connected: boolean;
  backend: string;
  cluster_count: number;
  healthy_clusters: number;
  workload_count: number;
  clusters: ClusterInfo[];
  error: string | null;
  summary_note?: string | null;
}

export interface ClusterPodSummary {
  name: string;
  phase: string;
  ready: number;
  total_containers: number;
  restarts: number;
  node: string | null;
}

export interface ClusterConditionSummary {
  type_: string;
  status: string;
  reason?: string | null;
  message?: string | null;
}

export interface ClusterOwnerReference {
  api_version: string;
  kind: string;
  name: string;
  uid: string | null;
  controller: boolean;
}

export interface ClusterOwnedResource {
  kind: string;
  name: string;
  api_version: string | null;
}

export interface ClusterResourceDetail {
  cluster: string;
  namespace: string;
  kind: string;
  name: string;
  api_version: string | null;
  uid?: string | null;
  pods: ClusterPodSummary[];
  conditions: ClusterConditionSummary[];
  owner_references?: ClusterOwnerReference[];
  owned_resources?: ClusterOwnedResource[];
  manifest: Record<string, unknown>;
}

export interface DriftReconcileResult {
  action_type: string;
  success: boolean;
  message: string;
}

export interface AlertChannelSummary {
  name: string;
  enabled: boolean;
  channel_type: string;
  min_severity: string;
  categories: string[];
}

export interface AlertRuleSummary {
  name: string;
  enabled: boolean;
  condition: string;
  severity: string;
  message_template: string;
  cooldown_seconds: number;
  last_triggered: string | null;
  workload?: string | null;
}

export interface AlertsStatus {
  channels: AlertChannelSummary[];
  rules: AlertRuleSummary[];
}

export interface ClusterRelatedEvent {
  type_: string;
  reason: string;
  message: string;
  timestamp: string;
}

export interface ClusterPortForwardSession {
  session_id: string;
  cluster: string;
  namespace: string;
  target_kind: string;
  target_name: string;
  local_port: number;
  remote_port: number;
  local_url: string;
}

export interface ClusterRolloutRevision {
  revision: string;
  change_cause: string;
}

export interface ClusterRolloutStatus {
  cluster: string;
  namespace: string;
  kind: string;
  name: string;
  status: string;
  history: ClusterRolloutRevision[];
}

export interface ClusterTopMetric {
  name: string;
  cpu: string;
  memory: string;
}

export interface ClusterMetricsSummary {
  scope: string;
  pod_count: number;
  total_cpu_millicores: number;
  total_memory_mib: number;
  pods: ClusterTopMetric[];
}

export interface ClusterDiffLine {
  kind: string;
  text: string;
}

export interface ClusterHealthSummary {
  level: string;
  summary: string;
  ready_pods: number;
  total_pods: number;
  warning_events: number;
}

export interface HelmRevisionEntry {
  revision: string;
  updated: string;
  status: string;
  chart: string;
  app_version?: string | null;
  description?: string | null;
}

export interface ClusterNamespaceSummary {
  name: string;
  status: string;
  created_at: string;
}

export interface ClusterBrowseItem {
  cluster: string;
  namespace: string;
  kind: string;
  name: string;
  status: string;
  created_at: string;
  detail?: string | null;
}

export interface AuthStatus {
  authenticated: boolean;
  username: string;
  role: string;
}

export interface BuildResponse {
  image_name: string;
  image_tag: string;
  full_name: string;
  runtime: string;
}

export interface ValidateResponse {
  valid: boolean;
  workload_name: string | null;
  errors: string[];
}

// ─── Cost ────────────────────────────────────────────────────────────
export interface CostEstimate {
  provider: string;
  cpu_cost_monthly: number;
  memory_cost_monthly: number;
  storage_cost_monthly: number;
  total_monthly: number;
  total_hourly: number;
  currency: string;
}

// ─── AI ──────────────────────────────────────────────────────────────
export interface RuntimeScore {
  runtime: string;
  total_score: number;
  cost_score: number;
  performance_score: number;
  reliability_score: number;
  availability_score: number;
  reasons: string[];
  warnings: string[];
}

export interface ScoringResult {
  recommended: string;
  scores: RuntimeScore[];
  workload_class: string;
  confidence: number;
}

export interface ScalingAdvice {
  action: string;
  current_replicas: number;
  recommended_replicas: number;
  reason: string;
  confidence: number;
  forecast?: {
    trend: string;
    predicted_value: number;
    lower_bound: number;
    upper_bound: number;
    horizon_minutes: number;
  };
  cost_impact?: {
    current_hourly: number;
    projected_hourly: number;
    delta_hourly: number;
    delta_monthly: number;
  };
}

export interface MigrationAdvice {
  workload_name: string;
  source_runtime: string;
  target_runtime: string;
  recommended_strategy: string;
  estimated_downtime_secs: number;
  risk_level: string;
  reasons: string[];
  warnings: string[];
  timing?: {
    recommendation: string;
    preferred_window: string;
    avoid_times: string[];
  };
  canary_config?: {
    steps: number[];
    step_interval_secs: number;
    error_threshold: number;
    latency_threshold_pct: number;
    min_observation_secs: number;
  };
}

export interface MigrationPlanProposal {
  advice: MigrationAdvice;
  blast_radius_score: number;
  rollback_probability: number;
  eta_secs: number;
  cost_impact_usd: number;
  auto_eligible: boolean;
}

export interface FleetRootCauseEntry {
  workload: string;
  likely_cause: string;
  confidence: number;
  evidence: string[];
  recommendation: string;
  health_level: string;
  summary: string;
}

export interface FleetRootCauseReport {
  generated_at: string;
  scanned: number;
  diagnoses: FleetRootCauseEntry[];
}

// ─── Scheduler ───────────────────────────────────────────────────────
export interface RuntimeUtilization {
  runtime: string;
  cpu_utilization: number;
  memory_utilization: number;
  workload_count: number;
  max_workloads: number;
  healthy: boolean;
  estimated_cost_per_day: number;
}

export interface OptimizeSuggestion {
  category: string;
  message: string;
  potential_saving: number | null;
}

// ─── Orchestrator / Health ───────────────────────────────────────────
export interface HealthSummary {
  healthy: number;
  degraded: number;
  unhealthy: number;
  unknown: number;
  circuits_open: number;
}

export interface ManagedWorkload {
  name: string;
  runtime: string;
  health: string;
  circuit: string;
  restart_count: number;
}

export interface HealthHistorySummary {
  total_checks: number;
  ready_checks: number;
  uptime_percent: number;
  last_state: string;
  last_restart_count: number;
}

// ─── Events ──────────────────────────────────────────────────────────
export interface Event {
  timestamp: string;
  severity: string;
  category: string;
  title: string;
  message: string;
  source: string;
  workload: string | null;
  acknowledged: boolean;
}

export interface EventSummary {
  total_events: number;
  unacknowledged: number;
  critical_unacked: number;
  by_severity: Record<string, number>;
  by_category: Record<string, number>;
}

// ─── Affinity ────────────────────────────────────────────────────────
export interface AffinityScore {
  runtime: string;
  composite_score: number;
  confidence: number;
  success_rate: number;
  avg_uptime: number;
  total_deployments: number;
}

// ─── Secrets ─────────────────────────────────────────────────────────
export interface SecretSummary {
  name: string;
  namespace: string;
  key_count: number;
  created_at: string;
  updated_at: string;
  needs_rotation: boolean;
}

export interface SecretDetail {
  name: string;
  namespace: string;
  key_count: number;
  keys: string[];
  created_at: string;
  updated_at: string;
  needs_rotation: boolean;
  rotation_policy: {
    interval_days: number;
    max_age_days: number;
    notify_before_days: number;
  } | null;
}

// ─── Environments ────────────────────────────────────────────────────
export interface Environment {
  name: string;
  tier: string;
  workloads: Record<string, unknown>;
  variables: Record<string, string>;
  updated_at: string;
}

// ─── Audit ───────────────────────────────────────────────────────────
export interface AuditSummary {
  total_events: number;
  successes: number;
  failures: number;
  unique_workloads: number;
  events_by_action: Record<string, number>;
}

export interface AuditEvent {
  id: number;
  timestamp: string;
  action: string;
  workload: string;
  runtime: string | null;
  result: string;
  message: string;
}

export interface AuditResponse {
  summary: AuditSummary;
  recent_events: AuditEvent[];
}

export interface AuditVerifyResponse {
  total: number;
  verified: number;
  tampered: number;
  integrity: string;
  tampered_events: Array<{
    id: number;
    timestamp: string;
    action: string;
    workload: string;
  }>;
}

// ─── Templates ───────────────────────────────────────────────────────
export interface Template {
  name: string;
  description: string;
  default_cpu: string;
  default_memory: string;
}

// ─── Backups ─────────────────────────────────────────────────────────
export interface BackupInfo {
  path: string;
  filename: string;
  workload_count: number;
  created_at: string;
  aether_version: string;
  description: string | null;
}

// ─── Plugins ─────────────────────────────────────────────────────────
export interface PluginInfo {
  name: string;
  version: string;
  runtime_kind: string;
  command: string;
  capabilities: string[];
}

// ─── Dependencies ────────────────────────────────────────────────────
export interface DependencyEdge {
  from: string;
  to: string;
}

export interface DependencyGraph {
  stats: {
    total_workloads: number;
    total_edges: number;
    root_workloads: number;
    leaf_workloads: number;
    max_depth: number;
    has_cycles: boolean;
  };
  startup_order: string[];
  issues: string[];
  nodes?: string[];
  edges?: DependencyEdge[];
}

// ─── SLA ─────────────────────────────────────────────────────────────
export interface SlaTarget {
  workload: string;
  uptime_target_pct: number;
  max_latency_ms: number | null;
  max_error_rate_pct: number | null;
  max_restarts_per_day: number | null;
}

// ─── Compose / RBAC ─────────────────────────────────────────────────
export interface ComposeValidationResult {
  valid: boolean;
  workload_count?: number;
  deploy_order?: string[];
}

export interface ApiKeySummary {
  name: string;
  role: string;
  created_at: string;
}

export interface CreateApiKeyResponse {
  name: string;
  role: string;
  key: string;
}

// ─── Drift ───────────────────────────────────────────────────────────
export interface DriftReport {
  workload_name: string;
  has_drift: boolean;
  drifts: DriftItem[];
  severity: string;
  reconciliation_plan: ReconcileAction[];
}

export interface DriftItem {
  field: string;
  expected: string;
  actual: string;
  severity: string;
  category: string;
}

export interface ReconcileAction {
  action_type: string;
  description: string;
  requires_restart: boolean;
  risk: string;
}

// ─── Policy ──────────────────────────────────────────────────────────
export interface PolicyResult {
  passed: boolean;
  violations: PolicyViolation[];
  warnings: PolicyWarning[];
  policies_evaluated: number;
}

export interface PolicyViolation {
  policy: string;
  rule: string;
  message: string;
  field: string;
  severity: string;
}

export interface PolicyWarning {
  policy: string;
  message: string;
  suggestion: string;
}

// ─── Health ──────────────────────────────────────────────────────────
export interface HealthResponse {
  status: string;
  version: string;
}

// ─── Platform / HA ───────────────────────────────────────────────────
export interface WorkloadStateBackendInfo {
  backend: 'local-json' | 'postgresql';
  configured: boolean;
  pollSecs: number;
  env: string;
  pollEnv: string;
}

export interface OpaEvaluation {
  configured: boolean;
  allowed: boolean;
  denials: string[];
}

export interface PlatformInfo {
  version: string;
  persistence: 'local-json' | 'postgresql';
  haMode: string;
  embeddedUiBuild?: string;
  integrations?: {
    backup_remote_configured?: boolean;
    audit_webhook_configured?: boolean;
    grafana_url?: string | null;
    prometheus_url?: string | null;
    hubble_ui_url?: string | null;
    grafana_dashboard_uid?: string | null;
    packetwolf_url?: string | null;
  };
  kubernetes?: {
    cilium?: CiliumStatusResponse | null;
  };
  haSharedCache: boolean;
  tls: boolean;
  workloadState: WorkloadStateBackendInfo;
  opa?: {
    configured: boolean;
    enforce: boolean;
    check_path?: string;
  };
  oidc: {
    enabled: boolean;
    issuer?: string | null;
  };
  safety: {
    mutation_confirm_required: boolean;
    mutation_confirm_header: string;
    mutation_confirm_values?: string[];
  };
}

export interface SystemReadyCheck {
  required: boolean;
  ok: boolean;
}

export interface SystemReadyStatus {
  ready: boolean;
  redis: boolean;
  ha_shared_cache?: boolean;
  postgres?: boolean;
  workload_state_backend?: string;
  checks?: {
    process?: boolean;
    redis?: SystemReadyCheck;
    postgres?: SystemReadyCheck;
  };
}

export interface PlatformRecommendation {
  id: string;
  category: string;
  severity: 'info' | 'warn' | 'critical' | string;
  title: string;
  detail: string;
  action: string;
}

export interface CiliumManagedPolicyStatus {
  name: string;
  scope: string;
  namespace?: string | null;
  exists: boolean;
  aether_managed: boolean;
}

export interface CiliumStatusResponse {
  cluster: string;
  namespace: string;
  cni: string;
  crds: {
    ciliumnetworkpolicies: boolean;
    ciliumclusterwidenetworkpolicies: boolean;
  };
  cilium_daemonset_ready: boolean;
  managed_policies: CiliumManagedPolicyStatus[];
  egress_mode: string;
  metrics_server: boolean;
  connectivity_check?: string;
  last_checked_at?: string | null;
  connectivity_detail?: string | null;
}

export interface ObservabilitySummary {
  api_http_requests_total: number;
  migrations_total: number;
  migration_rollbacks_total: number;
  workloads_running: Record<string, number>;
  cluster_metrics?: ClusterMetricsSummary | null;
  cilium?: CiliumStatusResponse | null;
  prometheus_configured: boolean;
}

// ─── View types ──────────────────────────────────────────────────────
export type AppView =
  | 'overview'
  | 'fabric'
  | 'migrations'
  | 'observability'
  | 'labs'
  | 'settings'
  | 'applications'
  | 'workloads'
  | 'clusters'
  | 'compose'
  | 'ai'
  | 'zeus'
  | 'copilot'
  | 'ai-providers'
  | 'cost'
  | 'affinity'
  | 'drift'
  | 'policy'
  | 'scheduler'
  | 'health'
  | 'events'
  | 'alerts'
  | 'platform'
  | 'sla'
  | 'deps'
  | 'envs'
  | 'secrets'
  | 'backups'
  | 'templates'
  | 'plugins'
  | 'rbac'
  | 'audit'
  | 'metrics'
  | 'gitops'
  | 'editor'
  | 'confidential'
  | 'intelligence'
  | 'fleet'
  | 'hosted'
  | 'activity'
  | 'security'
  | 'helm'
  | 'openapi'
  | 'license';

export interface LicenseStatusResponse {
  state: string;
  license_id: string | null;
  customer: string | null;
  customer_id: string | null;
  product: string | null;
  allowed_nodes: number | null;
  allowed_clusters: number | null;
  valid_from: string | null;
  valid_until: string | null;
  issued_at: string | null;
  license_version: number | null;
  days_remaining: number | null;
  current_node_count: number | null;
  warning_message: string | null;
}

export interface LicenseUsageNode {
  name: string;
  ready: boolean;
  billable: boolean;
}

export interface LicenseUsageResponse {
  allowed_nodes: number;
  current_node_count: number;
  utilization_pct: number;
  nodes: LicenseUsageNode[];
}

export interface HelmCatalogChart {
  id: string;
  name: string;
  category: string;
  chart: string;
  repo: string;
  version: string;
  description: string;
  storage_required: boolean;
  backup_supported: boolean;
  ha_available: boolean;
  monitoring_available: boolean;
}

export interface DiagnoseEvent {
  type_: string;
  reason: string;
  message: string;
  timestamp: string;
}

export interface DiagnoseRecommendation {
  title: string;
  summary: string;
  action: string;
  applyable?: boolean;
}

export interface DiagnoseResponse {
  workload: string;
  runtime?: string | null;
  source: string;
  health_level: string;
  summary: string;
  ready_pods: number;
  total_pods: number;
  warning_events: number;
  events: DiagnoseEvent[];
  log_excerpt?: string | null;
  pods: Array<{
    name: string;
    phase: string;
    ready: number;
    total_containers: number;
    restarts: number;
    node?: string | null;
  }>;
  recommendations: DiagnoseRecommendation[];
  evidence: string[];
}

// ─── Intelligence layer ────────────────────────────────────────────
export interface FailureSignal {
  kind: string;
  probability: number;
  horizon: string;
  reason: string;
}

export interface WorkloadPrediction {
  workload: string;
  risk_score: number;
  risk_level: string;
  predictions: FailureSignal[];
}

export interface PredictionReport {
  generated_at: string;
  fleet_risk_score: number;
  predictions: WorkloadPrediction[];
}

export interface ThreatEntry {
  workload: string;
  severity: string;
  category: string;
  score: number;
  reason: string;
  detected_at: string;
}

export interface ThreatReport {
  generated_at: string;
  threats: ThreatEntry[];
}

export interface CostOptimizeRecommendation {
  workload: string;
  current_runtime: string;
  suggested_runtime: string;
  savings_pct: number;
  savings_monthly_usd: number;
  risk: string;
  reason: string;
}

export interface CostOptimizeReport {
  generated_at: string;
  total_potential_savings_pct: number;
  recommendations: CostOptimizeRecommendation[];
}

export interface SecurityPolicySuggestion {
  workload: string;
  severity: string;
  title: string;
  policy_yaml: string;
  rationale: string;
}

export interface SecurityCopilotReport {
  generated_at: string;
  suggestions: SecurityPolicySuggestion[];
}

export interface TwinSnapshot {
  fleet_risk_score: number;
  avg_cpu_utilization: number;
  avg_memory_utilization: number;
  estimated_monthly_cost_usd: number;
  saturation_days: number;
}

export interface TwinDeltas {
  risk_delta: number;
  cpu_util_delta: number;
  memory_util_delta: number;
  cost_delta_usd: number;
}

export interface TwinSimulateReport {
  generated_at: string;
  scenario: string;
  baseline: TwinSnapshot;
  projected: TwinSnapshot;
  deltas: TwinDeltas;
  recommendations: string[];
}

export interface AutonomyPolicy {
  auto_restart: boolean;
  auto_reconcile_drift: boolean;
  auto_migrate: string;
  auto_evolve: string;
}

export interface AutonomyEnvFlags {
  aether_auto_restart: boolean;
  aether_auto_reconcile: boolean;
  reconciliation_auto_reconcile: boolean;
}

export interface WorkloadAutonomyOverride {
  workload: string;
  migration: string;
  healing: string;
  evolution: string;
}

export interface AutonomyStatusReport {
  generated_at: string;
  effective_policy: AutonomyPolicy;
  env: AutonomyEnvFlags;
  autonomy_enabled: boolean;
  workload_overrides: WorkloadAutonomyOverride[];
  recommendations: string[];
}

export interface HealerPreviewReport {
  generated_at: string;
  policy: AutonomyPolicy;
  orchestrator_actions: string[];
  drift_candidates: string[];
  would_execute: string[];
  would_skip: string[];
}

export interface HealerExecuteReport {
  dry_run: boolean;
  executed: string[];
  skipped: string[];
}

export interface NextAction {
  id: string;
  title: string;
  detail: string;
  priority: number;
  route: string;
  action_type: string;
}

export interface NextActionsReport {
  generated_at: string;
  actions: NextAction[];
}

export interface AgentStatusEntry {
  id: string;
  label: string;
  status: string;
  detail: string;
  route: string;
  pending_count: number;
}

export interface AgentRegistryReport {
  generated_at: string;
  agents: AgentStatusEntry[];
}

export interface CriticalNotification {
  id: string;
  severity: string;
  title: string;
  detail: string;
  workload?: string | null;
  route: string;
}

export interface CriticalNotificationsReport {
  generated_at: string;
  notifications: CriticalNotification[];
  notify_tray: boolean;
}

export interface EvolutionExecuteReport {
  dry_run: boolean;
  executed: string[];
  skipped: string[];
}

export interface GitOpsPullRequestLink {
  title: string;
  url: string;
  workload: string;
}

export interface GitOpsAgentSyncReport {
  generated_at: string;
  federation_enabled: boolean;
  federation_target?: string | null;
  drift_workloads: string[];
  planned_actions: string[];
  auto_safe_count: number;
  pr_links?: GitOpsPullRequestLink[];
}

export interface GitOpsAgentExecuteReport {
  dry_run: boolean;
  executed: string[];
  skipped: string[];
}

export interface CostApplyPatch {
  workload: string;
  field: string;
  before: string;
  after: string;
  savings_monthly_usd: number;
  patch_yaml: string;
}

export interface CostApplyReport {
  dry_run: boolean;
  patches: CostApplyPatch[];
  applied: string[];
  skipped: string[];
}

export interface SecurityRemediateReport {
  dry_run: boolean;
  applied: string[];
  skipped: string[];
  pending_confirmation: string[];
}

export interface ScaleSuggestion {
  workload: string;
  resource: string;
  current: string;
  suggested: string;
  reason: string;
  kind: string;
  auto_safe: boolean;
}

export interface CapacityScaleReport {
  generated_at: string;
  suggestions: ScaleSuggestion[];
}

export interface CapacityScaleExecuteReport {
  dry_run: boolean;
  executed: string[];
  skipped: string[];
}

export interface NlIntentReport {
  generated_at: string;
  parsed_goal: string;
  intent_yaml: string;
  spec_yaml: string;
  confidence: number;
  keywords: string[];
}

export interface IntentDeployReport {
  dry_run: boolean;
  workload_name: string;
  validated: boolean;
  policy_passed: boolean;
  compliance_passed: boolean;
  blocked: boolean;
  block_reason?: string | null;
  spec_path?: string | null;
  steps: string[];
}

export interface IntentViolationEntry {
  workload: string;
  violation_type: string;
  severity: string;
  detail: string;
  current_value: string;
  intent_target: string;
}

export interface IntentViolationsReport {
  generated_at: string;
  violations: IntentViolationEntry[];
  reconciliation_status: string;
}

export interface IntentSlaBreach {
  workload: string;
  metric: string;
  current: string;
  target: string;
  severity: string;
}

export interface IntentSlaBreachesReport {
  generated_at: string;
  breaches: IntentSlaBreach[];
}

export interface IntentTemplate {
  id: string;
  goal: string;
  title: string;
  description: string;
  spec_yaml: string;
}

export interface IntentTemplateLibrary {
  generated_at: string;
  templates: IntentTemplate[];
}

export interface IntentBundleWorkload {
  role: string;
  workload_name: string;
  spec_yaml: string;
}

export interface IntentBundleReport {
  generated_at: string;
  bundle_name: string;
  shared_intent_yaml: string;
  workloads: IntentBundleWorkload[];
}

export interface IntentGitOpsDiffEntry {
  workload: string;
  has_drift: boolean;
  live_intent_yaml: string;
  gitops_intent_yaml?: string | null;
  summary: string;
}

export interface IntentGitOpsDiffReport {
  generated_at: string;
  entries: IntentGitOpsDiffEntry[];
}

export interface IntentVersionEntry {
  version_id: string;
  recorded_at: string;
  intent_yaml: string;
  goal: string;
}

export interface IntentVersionHistory {
  workload: string;
  versions: IntentVersionEntry[];
}

export interface KnowledgeGraphNode {
  id: string;
  label: string;
  kind: string;
  severity?: string | null;
}

export interface KnowledgeGraphEdge {
  from: string;
  to: string;
  kind: string;
}

export interface KnowledgeGraphStats {
  workloads: number;
  dependencies: number;
  threats: number;
  drifted: number;
  clusters: number;
}

export interface KnowledgeGraphReport {
  generated_at: string;
  nodes: KnowledgeGraphNode[];
  edges: KnowledgeGraphEdge[];
  stats: KnowledgeGraphStats;
}

export interface AgentStatusSummary {
  id: string;
  label: string;
  status: 'active' | 'idle' | 'alert';
  detail: string;
  view: AppView;
}

export interface IntentPipelineStep {
  phase: string;
  title: string;
  detail: string;
  action: string;
}

export interface IntentPipelineReport {
  generated_at: string;
  workload_name: string;
  intent_yaml: string;
  spec_yaml: string;
  recommended_runtime: string;
  confidence: number;
  placement: PlacementRecommendation[];
  steps: IntentPipelineStep[];
}

export interface SreRunbookSection {
  title: string;
  severity: string;
  items: string[];
}

export interface SreRunbookReport {
  generated_at: string;
  summary: string;
  sections: SreRunbookSection[];
  runbook_markdown: string;
}

export interface ClusterPosture {
  cluster: string;
  reachable: boolean;
  server?: string | null;
  workload_count: number;
  runtimes: string[];
  anomaly_count: number;
  score: number;
}

export interface MultiCloudPostureReport {
  generated_at: string;
  federation_enabled: boolean;
  federation_clusters: string[];
  clusters: ClusterPosture[];
  fleet_workloads: number;
  reachable_clusters: number;
  recommended_actions: string[];
}

export interface EvolutionEntry {
  workload: string;
  current_runtime: string;
  recommended_runtime: string;
  improvement_pct: number;
  confidence: number;
  auto_eligible: boolean;
  trajectory: string[];
  reasons: string[];
}

export interface EvolutionStatus {
  generated_at: string;
  workloads: EvolutionEntry[];
}

export interface PlacementRecommendation {
  cluster: string | null;
  runtime: string;
  score: number;
  latency_score: number;
  cost_score: number;
  gpu_available: boolean;
  reasons: string[];
}

export interface SecurityProfileEntry {
  id: string;
  label: string;
  description: string;
  kata_runtime_class?: string | null;
}

export interface AttestationVerifyResponse {
  vm_id: string;
  verdict: AttestationVerdict;
  measurements: Record<string, string>;
  reason_codes: string[];
  verified_at: string;
}
export type AttestationVerdict = 'pass' | 'fail' | 'pending';

export interface RagnarokIntegration {
  mode: 'embedded' | 'composite' | string;
  remote_url?: string | null;
}

export interface HostTeeStatus {
  sev_device: boolean;
  sev_snp: boolean;
  tdx: boolean;
  dev_sev_path?: string | null;
  notes: string[];
}

export interface TeeCapabilities {
  host: HostTeeStatus;
  clusters: Array<{
    cluster: string;
    nodes_with_snp: number;
    nodes_with_tdx: number;
    total_nodes: number;
    labels: string[];
  }>;
  integration?: RagnarokIntegration;
}

export interface KataStatus {
  supported_runtime_classes: string[];
  operator_requirements: string[];
  hypervisor: string;
  host_tee: TeeCapabilities['host'];
  placement_ready: boolean;
  notes: string[];
}

export interface SovereignVerdict {
  compliant: boolean;
  violations: string[];
  hints: string[];
  config: SovereignConfig;
}

export interface ConfidentialNetworkStatus {
  workload: string;
  policy_count: number;
  cilium_auto_policy: boolean;
  packetwolf_hints: Record<string, string>;
  spiffe_id?: string | null;
  recommendations: string[];
}

export interface ConfidentialAnalysis {
  workload: string;
  runtime: string;
  trust_composite: number;
  risk_level: string;
  summary: string;
  findings: string[];
  recommendations: string[];
  attestation_passed: boolean;
  image_in_catalog: boolean;
  sovereign_compliant: boolean;
  kata_placement_score: number;
  migration_strategy?: string | null;
}

export interface ConfidentialFleetAnalysis {
  generated_at: string;
  workloads: ConfidentialAnalysis[];
  fleet_trust_avg: number;
  critical_count: number;
}

export interface ConfidentialMigrationPlan {
  workload: string;
  source_tee_capable: boolean;
  target_tee_capable: boolean;
  encrypted_channel_required: boolean;
  re_attestation_on_target: boolean;
  launch_digest?: string | null;
  launch_digest_preserved: boolean;
  migration_uri: string;
  encrypted_migration_uri: string;
  blockers: string[];
  phases: string[];
  hyper2kvm_hints: string[];
  recommended_strategy: string;
  ready_for_cutover: boolean;
}

export interface ConfidentialMigrationRecord {
  workload: string;
  phase: string;
  target_runtime: string;
  migration_uri: string;
  started_at: string;
  attestation_required: boolean;
  cutover_ready: boolean;
  error?: string | null;
}

export interface ConfidentialFleetRow {
  workload: string;
  runtime: string;
  tee: string;
  image_digest?: string | null;
  image_in_catalog: boolean;
  attestation_passed: boolean;
  trust: NetworkTrustScore;
}

export interface ImageVerifyResult {
  name: string;
  verified: boolean;
  image_hash?: string | null;
  launch_digest?: string | null;
  message: string;
}

export interface NetworkTrustScore {
  workload: string;
  attestation_score: number;
  network_policy_score: number;
  firmware_exposure_score: number;
  composite: number;
  spiffe_id?: string | null;
}

export interface AttestationStatus {
  vm_id: string;
  last_verdict: AttestationVerdict;
  baseline_digest?: string | null;
  drift: boolean;
  updated_at: string;
}

export interface AttestationExplain {
  vm_id: string;
  verdict: AttestationVerdict;
  summary: string;
  failure_reasons: Array<{ code: string; message: string; severity: string }>;
  measurement_diff: Record<string, { expected?: string | null; observed?: string | null; matched: boolean }>;
  guestkit?: GuestKitSummary | null;
}

export interface GuestKitSummary {
  last_mode: string;
  passed: boolean;
  findings: string[];
  repair_steps: string[];
  inspected_at: string;
}

export interface GuestKitResult {
  vm_id: string;
  mode: string;
  passed: boolean;
  findings: string[];
  chain_valid: boolean;
  repair_steps: string[];
  inspected_at: string;
}

export interface SovereignConfig {
  byok_signing_key?: string | null;
  offline_attestation: boolean;
  embedded_cert_bundle?: string | null;
  region_lock?: string | null;
}

export interface AttestGatedSecretStatus {
  secret_name: string;
  state: 'pending' | 'released' | 'revoked' | 'injected' | string;
  provider: string;
  expires_at?: string | null;
  k8s_secret_name?: string | null;
}

export interface SecretReleaseToken {
  vm_id: string;
  secret_name: string;
  token: string;
  expires_at: string;
  provider: string;
}

export interface MeasuredImageManifest {
  name: string;
  image_hash: string;
  kernel_hash?: string | null;
  initrd_hash?: string | null;
  launch_digest?: string | null;
  signing_key_id: string;
  signed_at: string;
}

export interface GitOpsConfidentialAudit {
  file_path: string;
  workload?: string | null;
  confidential_enabled: boolean;
  gitops_issues: string[];
  sovereign_compliant?: boolean | null;
  sovereign_violations: string[];
}

export interface IsolationVerdict {
  compliant: boolean;
  violations: string[];
  node_taints: string[];
  scheduler_hints: Record<string, string>;
}

export interface ConfidentialPlacementAdvice {
  workload: string;
  confidential_enabled: boolean;
  tee?: string | null;
  recommended_runtime: string;
  kata_runtime_class?: string | null;
  schedule_constraints: string[];
  scheduler_hints: Record<string, string>;
  host_tee_ready: boolean;
  placement_score_bonus: number;
  blockers: string[];
  gitops_issues: string[];
  isolation_compliant: boolean;
}

export interface PacketWolfStatus {
  configured: boolean;
  reachable: boolean;
  version?: string | null;
  hint?: string | null;
  cached?: boolean;
}

export interface EdgeAgentRecord {
  site: string;
  kube_context?: string | null;
  labels: Record<string, string>;
  registered_at: string;
  last_heartbeat: string;
  queue_depth: number;
  last_error?: string | null;
  online: boolean;
}

export interface FederationPlan {
  workload: string;
  recommended_runtime: string;
  recommended_cluster?: string | null;
  anomaly_signals_configured?: boolean;
  total_anomalies?: number;
  clusters: ClusterPlacementScore[];
}

export interface ClusterPlacementScore {
  cluster: string;
  score: number;
  reachable: boolean;
  server?: string | null;
  runtime_hint: string;
  reasons: string[];
  anomaly_count?: number;
  anomaly_penalty?: number;
}

export interface FleetDriftSummary {
  total_workloads: number;
  drifted: number;
  critical: number;
  warning: number;
  rows: FleetDriftRow[];
}

export interface FleetDriftRow {
  workload: string;
  cluster?: string | null;
  namespace?: string | null;
  runtime: string;
  has_drift: boolean;
  severity: string;
  drift_count: number;
}

export interface VolumeReplicationPlan {
  source_cluster: string;
  target_cluster: string;
  namespace: string;
  pvc_name: string;
  steps: { order: number; action: string; detail: string }[];
  warnings: string[];
}

export interface FleetMigrationPlan {
  target_cluster: string;
  strategy: string;
  items: {
    workload: string;
    source_runtime: string;
    target_runtime: string;
    target_cluster: string;
    strategy: string;
    volume_replication: boolean;
    status: string;
  }[];
  warnings: string[];
}

export interface FederationExecuteReport {
  dry_run: boolean;
  target_cluster: string | null;
  executed: string[];
  skipped: string[];
}

export interface CostArbitrageEntry {
  workload: string;
  current_provider: string;
  suggested_provider: string;
  current_monthly_usd: number;
  suggested_monthly_usd: number;
  savings_usd: number;
  reason: string;
}

export interface CostArbitrageReport {
  generated_at: string;
  entries: CostArbitrageEntry[];
  total_savings_usd: number;
}

export interface ClusterMeshNode {
  id: string;
  label: string;
  reachable: boolean;
  score: number;
  anomaly_count: number;
}

export interface ClusterMeshEdge {
  from: string;
  to: string;
  kind: string;
}

export interface ClusterHealthMeshReport {
  generated_at: string;
  nodes: ClusterMeshNode[];
  edges: ClusterMeshEdge[];
}

export interface UnifiedFabricNode {
  id: string;
  label: string;
  kind: string;
  online: boolean;
  detail: string;
}

export interface UnifiedFabricEdge {
  from: string;
  to: string;
}

export interface UnifiedFabricReport {
  generated_at: string;
  nodes: UnifiedFabricNode[];
  edges: UnifiedFabricEdge[];
}

export interface VolumeReplicationStatusEntry {
  workload: string;
  has_persistence: boolean;
  plan_ready: boolean;
  summary: string;
}

export interface VolumeReplicationStatusReport {
  generated_at: string;
  entries: VolumeReplicationStatusEntry[];
}

export interface MigrationWaveReport {
  generated_at: string;
  target_cluster: string;
  wave_size: number;
  plan: FleetMigrationPlan;
}

export interface GeoPlacementEntry {
  cluster: string;
  region_hint: string;
  estimated_rtt_ms: number;
  placement_score: number;
}

export interface GeoPlacementReport {
  generated_at: string;
  origin_region: string;
  clusters: GeoPlacementEntry[];
}

export interface CloudAccountLink {
  provider: string;
  configured: boolean;
  account_hint: string;
  env_var: string;
}

export interface CloudAccountVaultReport {
  generated_at: string;
  accounts: CloudAccountLink[];
}

export interface RegionLockEntry {
  workload: string;
  compliant: boolean;
  region_lock: string | null;
  violations: string[];
}

export interface RegionLockReport {
  generated_at: string;
  sovereign_region_lock: string | null;
  workloads: RegionLockEntry[];
}

export interface PacketWolfGuardEntry {
  cluster: string;
  anomaly_count: number;
  blocked: boolean;
  penalty: number;
}

export interface PacketWolfGuardReport {
  generated_at: string;
  configured: boolean;
  blocked_clusters: string[];
  entries: PacketWolfGuardEntry[];
}

export interface PacketWolfGuardApplyReport {
  dry_run: boolean;
  applied: string[];
  skipped: string[];
}

export interface SreScheduleEntry {
  id: string;
  cron: string;
  label: string;
  enabled: boolean;
  next_hint: string;
}

export interface SreRunbookScheduleReport {
  generated_at: string;
  scheduler_enabled: boolean;
  entries: SreScheduleEntry[];
}

export interface IncidentTimelineEntry {
  id: string;
  timestamp: string;
  source: string;
  severity: string;
  title: string;
  detail: string;
  workload: string | null;
}

export interface IncidentTimelineReport {
  generated_at: string;
  entries: IncidentTimelineEntry[];
}

export interface OnCallChannelStatus {
  provider: string;
  configured: boolean;
  channel_count: number;
  env_hint: string;
}

export interface OnCallIntegrationReport {
  generated_at: string;
  channels: OnCallChannelStatus[];
  webhook_ready: boolean;
}

export interface PostmortemSection {
  heading: string;
  bullets: string[];
}

export interface PostmortemReport {
  generated_at: string;
  title: string;
  sections: PostmortemSection[];
  markdown: string;
}

export interface ErrorBudgetEntry {
  workload: string;
  status: string;
  uptime_target_pct: number;
  uptime_actual_pct: number;
  error_budget: {
    total_minutes: number;
    consumed_minutes: number;
    remaining_minutes: number;
    consumed_pct: number;
    projected_exhaustion_days: number | null;
  };
  burn_rate: number;
}

export interface ErrorBudgetDashboardReport {
  generated_at: string;
  entries: ErrorBudgetEntry[];
}

export interface ChaosExperiment {
  id: string;
  label: string;
  target: string;
  risk: string;
  description: string;
}

export interface ChaosExperimentCatalog {
  generated_at: string;
  experiments: ChaosExperiment[];
}

export interface GameDayScenario {
  id: string;
  title: string;
  duration_minutes: number;
  steps: string[];
  participants: string[];
}

export interface GameDayPlanReport {
  generated_at: string;
  scenarios: GameDayScenario[];
}

export interface RunbookExecuteReport {
  dry_run: boolean;
  runbook_summary: string;
  executed: string[];
  skipped: string[];
}

export interface EscalationStep {
  order: number;
  agent: string;
  trigger: string;
  action: string;
}

export interface EscalationPolicyReport {
  generated_at: string;
  policies: EscalationStep[];
}

export interface MttrEntry {
  workload: string;
  incidents: number;
  avg_recovery_minutes: number;
  last_incident: string | null;
}

export interface MttrReport {
  generated_at: string;
  fleet_avg_mttr_minutes: number;
  entries: MttrEntry[];
}

export interface GraphImpactReport {
  generated_at: string;
  workload: string;
  impact: {
    workload: string;
    affected_workloads: string[];
    cascade_count: number;
    severity: string;
  };
  downstream: string[];
  summary: string;
}

export interface BlastRadiusReport {
  generated_at: string;
  workload: string;
  blast_score: number;
  impact: GraphImpactReport['impact'];
  twin_risk_delta: number;
  migration_blockers: string[];
  summary: string;
}

export interface ThreatPath {
  path: string[];
  severity: string;
  summary: string;
}

export interface ThreatPathsReport {
  generated_at: string;
  paths: ThreatPath[];
}

export interface GraphSearchHit {
  id: string;
  label: string;
  kind: string;
  score: number;
  route: string;
}

export interface GraphSearchReport {
  generated_at: string;
  query: string;
  hits: GraphSearchHit[];
}

export interface GraphSnapshot {
  id: string;
  captured_at: string;
  label: string;
  node_count: number;
  edge_count: number;
}

export interface GraphSnapshotsReport {
  generated_at: string;
  snapshots: GraphSnapshot[];
}

export interface CmdbItem {
  id: string;
  name: string;
  item_type: string;
  dependencies: string[];
}

export interface CmdbSyncReport {
  generated_at: string;
  source: string;
  items: CmdbItem[];
  synced_edges: number;
  dry_run: boolean;
}

export interface K8sServiceNode {
  service: string;
  workload: string;
  service_type: string;
  port: number;
}

export interface K8sImportReport {
  generated_at: string;
  services: K8sServiceNode[];
  edges_added: number;
  dry_run: boolean;
}

export interface GraphPlacementEntry {
  workload: string;
  startup_order: number;
  depends_on: string[];
  recommended_cluster: string | null;
  co_locate_with: string | null;
}

export interface GraphPlacementReport {
  generated_at: string;
  entries: GraphPlacementEntry[];
}

export interface GraphExportReport {
  generated_at: string;
  format: string;
  payload: string;
  node_count: number;
  edge_count: number;
}

export interface TraySparklineReport {
  generated_at: string;
  samples: number[];
  sparkline: string;
  fleet_health_pct: number;
}

export interface LiveActivityEntry {
  workload: string;
  phase: string;
  progress_pct: number;
  detail: string;
}

export interface LiveActivityReport {
  generated_at: string;
  active: LiveActivityEntry[];
}

export interface DockBadgeReport {
  generated_at: string;
  issue_count: number;
  badge_label: string;
}

export interface SpotlightIndexItem {
  id: string;
  title: string;
  subtitle: string;
  deep_link: string;
}

export interface SpotlightIndexReport {
  generated_at: string;
  items: SpotlightIndexItem[];
}

export interface ShortcutAction {
  name: string;
  phrase: string;
  url: string;
}

export interface ShortcutsManifestReport {
  generated_at: string;
  shortcuts: ShortcutAction[];
}

export interface MenuExtraToggle {
  id: string;
  label: string;
  enabled: boolean;
}

export interface MenuExtrasReport {
  generated_at: string;
  toggles: MenuExtraToggle[];
}

export interface OfflineCacheReport {
  generated_at: string;
  cached: boolean;
  briefing: Record<string, unknown> | null;
}

export interface UniversalLinkRoute {
  scheme: string;
  path: string;
  view: string;
}

export interface UniversalLinkRegistry {
  generated_at: string;
  routes: UniversalLinkRoute[];
}

export interface UniversalLinkResolveReport {
  url: string;
  view: string;
  query: Record<string, string>;
}

export interface ReleasePipelineStep {
  id: string;
  label: string;
  status: string;
}

export interface ReleasePipelineReport {
  generated_at: string;
  notarization_ready: boolean;
  signing_identity_configured: boolean;
  steps: ReleasePipelineStep[];
}

export interface HostedTenant {
  id: string;
  name: string;
  slug: string;
  plan: string;
  active: boolean;
}

export interface HostedFederationStatus {
  federation_enabled: boolean;
  tenant_count: number;
  policy: {
    clusters: string[];
    weights: Record<string, number>;
  };
}

export interface BillingSummary {
  period: string;
  total_workloads: number;
  stripe_configured?: boolean;
  tenants: { tenant_id: string; tenant_slug: string; plan: string; workload_count: number; api_requests_estimate: number }[];
}

export interface RemediationAction {
  action_type: string;
  target: string;
  cluster?: string | null;
  reason: string;
  auto_safe: boolean;
}

export interface RemediationPlan {
  generated_at: string;
  sources: string[];
  actions: RemediationAction[];
  warnings: string[];
}

export interface SbomMetadata {
  bom_format: string;
  spec_version: string;
  version: number;
  serial_number: string;
  component_count: number;
  aether_version: string;
  binary_sha256?: string | null;
  dashboard_sha256?: string | null;
}

export interface SignedImageManifest {
  name: string;
  image_hash: string;
  launch_digest?: string | null;
  signing_key_id: string;
  signed_at: string;
  sbom_digest?: string | null;
  signature?: string | null;
}
