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

export interface ClusterResourceDetail {
  cluster: string;
  namespace: string;
  kind: string;
  name: string;
  api_version: string | null;
  pods: ClusterPodSummary[];
  conditions: ClusterConditionSummary[];
  manifest: Record<string, unknown>;
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

export interface PlatformInfo {
  version: string;
  persistence: 'local-json' | 'postgresql';
  haMode: string;
  haSharedCache: boolean;
  tls: boolean;
  workloadState: WorkloadStateBackendInfo;
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

// ─── View types ──────────────────────────────────────────────────────
export type AppView =
  | 'overview'
  | 'workloads'
  | 'clusters'
  | 'compose'
  | 'ai'
  | 'cost'
  | 'affinity'
  | 'drift'
  | 'policy'
  | 'scheduler'
  | 'health'
  | 'events'
  | 'sla'
  | 'deps'
  | 'envs'
  | 'secrets'
  | 'backups'
  | 'templates'
  | 'plugins'
  | 'rbac'
  | 'audit'
  | 'metrics';
