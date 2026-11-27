// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/** Types for the operator API (`src/api/operator_handlers.rs`). */

export type Level = 'info' | 'warning' | 'critical';

export interface AttentionItem {
  level: Level;
  workload: string | null;
  kind: string;
  title: string;
  detail: string;
  next: string | null;
}

export interface AttentionQueue {
  generated_at: string;
  workloads_checked: number;
  items: AttentionItem[];
}

export interface Change {
  at: string;
  workload: string | null;
  kind: 'deploy' | 'operation' | 'event' | 'incident-opened' | 'incident-resolved' | 'note';
  level: Level;
  title: string;
  detail: string;
  actor: string | null;
  revision: number | null;
  incident: number | null;
  fields: string[];
}

export interface ChangeFeed {
  since: string;
  until: string;
  total: number;
  truncated: boolean;
  changes: Change[];
  unavailable: string[];
}

export interface Recovery {
  incidents: number;
  recovered: number;
  mean_minutes: number | null;
  last_opened_at: string | null;
}

export interface Incident {
  id: number;
  workload: string;
  runtime: string;
  opened_at: string;
  detected_at: string;
  last_failed_at: string;
  resolved_at: string | null;
  resolution: 'recovered' | 'stopped' | 'deleted' | null;
  failed_checks: number;
  last_state: string;
  last_message: string | null;
  restarts: number;
  acknowledged: { at: string; actor: string } | null;
  notes: { at: string; actor: string; text: string }[];
  duration_secs: number;
}

export interface IncidentList {
  incidents: Incident[];
  recovery: Recovery;
  observation_threshold: number;
}

export interface Suspect {
  change: Change;
  seconds_before: number;
  relation: string;
}

export interface IncidentDetail extends Incident {
  suspects: Suspect[];
  timeline: Change[];
  window_secs: number;
}

export type Impact = 'none' | 'in_place' | 'rolling' | 'recreate';

export interface PlanReport {
  changes: {
    path: string;
    kind: 'added' | 'removed' | 'changed';
    old: string | null;
    new: string | null;
    impacts: { target: string; impact: Impact; reason: string }[];
  }[];
  summary: { target: string; worst: Impact; affected_fields: number }[];
}

export interface Preview {
  workload: string;
  action: string;
  runtime: string;
  verdict: 'blocked' | 'caution' | 'clear';
  blockers: string[];
  warnings: string[];
  effects: string[];
  plan: PlanReport | null;
  target_revision: number | null;
  affected: string[];
}

export type PreviewAction =
  | { action: 'update'; yaml: string }
  | { action: 'delete' }
  | { action: 'stop' }
  | { action: 'restart' }
  | { action: 'migrate'; target: string }
  | { action: 'rollback'; revision?: number };

export interface RevisionSummary {
  id: number;
  at: string;
  op: 'deploy' | 'update';
  runtime: string;
  image: string;
  actor: string;
  reason: string | null;
  spec_hash: string;
  changed_fields: number | null;
}

export interface Hold {
  workload: string;
  reason: string;
  actor: string;
  created_at: string;
  expires_at: string | null;
}

export interface DecisionRule {
  rule: string;
  matched: boolean;
  applied: boolean;
  detail: string;
}

export interface Explanation {
  workload: string;
  summary: string;
  runtime: string;
  image: string;
  spec_source: string | null;
  decision: { selected: string | null; automatic: boolean; rules: DecisionRule[] } | null;
  placement: string;
  health: {
    observed_at: string | null;
    state: string | null;
    ready: boolean | null;
    restarts: number | null;
    checks_24h: number;
    ready_pct_24h: number | null;
    error: string | null;
    expected_down: string | null;
  };
  open_incident: number | null;
  recovery: Recovery;
  dependencies: string[];
  dependents: string[];
  hold: Hold | null;
  ignored_fields: { path: string; support: string; note: string }[];
  recent_changes: Change[];
  attention: AttentionItem[];
}

export interface ReportRow {
  workload: string;
  runtime: string;
  checks: number;
  ready_checks: number;
  ready_pct: number | null;
  incidents: number;
  incident_secs: number;
  deploys: number;
  failed_operations: number;
}

export interface OpsReport {
  generated_at: string;
  since: string;
  workloads: ReportRow[];
  incidents: { id: number; workload: string; status: string; duration_secs: number }[];
  recovery: Recovery;
  changes: ChangeFeed;
  attention: AttentionItem[];
  audit: { events_in_window: number; events_total: number; failed_integrity: number };
  caveats: string[];
}

export interface ObserveReport {
  observed: number;
  opened: number[];
  resolved: number[];
  errors: string[];
}
