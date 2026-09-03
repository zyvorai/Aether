// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/*
  "Worst first" ordering for workload/resource status reasons — a deliberate
  product opinion (crash-like states sort above pending above healthy), not a
  generic log-severity color map (see getSeverityColor in formatters.ts for
  that — a different domain: CRITICAL/WARNING/INFO log levels, not pod/VM
  status reasons like CrashLoopBackOff/OOMKilled/Pending/Running).
*/

const SEVERITY_RANK: Record<string, number> = {
  CrashLoopBackOff: 0,
  OOMKilled: 1,
  ImagePullBackOff: 2,
  ErrImagePull: 2,
  Error: 3,
  Failed: 3,
  Unschedulable: 4,
  Evicted: 5,
  Terminating: 6,
  NotReady: 7,
  Degraded: 7,
  Pending: 8,
  ContainerCreating: 9,
  Running: 10,
  Healthy: 10,
  Completed: 11,
  Succeeded: 11,
};

const DEFAULT_RANK = 10;

export function severityRank(status: string | undefined | null): number {
  if (!status) return DEFAULT_RANK;
  return SEVERITY_RANK[status] ?? DEFAULT_RANK;
}

export type SeverityTone = 'danger' | 'warn' | 'neutral';

export function severityTone(status: string | undefined | null): SeverityTone {
  const rank = severityRank(status);
  if (rank <= 3) return 'danger';
  if (rank <= 7) return 'warn';
  return 'neutral';
}

const TONE_VAR: Record<SeverityTone, string> = {
  danger: 'var(--danger)',
  warn: 'var(--warning)',
  neutral: 'var(--subtle)',
};

export function severityColorVar(status: string | undefined | null): string {
  return TONE_VAR[severityTone(status)];
}

export function sortBySeverity<T>(items: T[], getStatus: (item: T) => string | undefined | null): T[] {
  return [...items].sort((a, b) => severityRank(getStatus(a)) - severityRank(getStatus(b)));
}
