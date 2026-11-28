// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

export type WorkloadCondition = 'running' | 'failing' | 'degraded' | 'stopped' | 'unknown';

// "stopped unexpectedly" is a failure; plain "stopped" is somebody's choice.
const FAILING = ['failed', 'error', 'crashloop', 'oomkilled', 'imagepullbackoff', 'errimagepull', 'evicted', 'unexpectedly'];
const DEGRADED = ['degraded', 'unhealthy', 'notready', 'not ready', 'unschedulable', 'pending', 'terminating'];

/**
 * One reading of the status strings the server reports, shared by every surface.
 * Having been deployed says nothing about running now, so only an observed
 * state counts as running.
 */
export function workloadCondition(status: string | null | undefined): WorkloadCondition {
  const s = (status ?? '').toLowerCase();
  if (FAILING.some((word) => s.includes(word))) return 'failing';
  if (DEGRADED.some((word) => s.includes(word))) return 'degraded';
  if (s === 'stopped' || s === 'exited' || s === 'succeeded') return 'stopped';
  if (s === 'running' || s === 'healthy' || s === 'ready') return 'running';
  return 'unknown';
}

export function needsAttention(status: string | null | undefined): boolean {
  const condition = workloadCondition(status);
  return condition === 'failing' || condition === 'degraded';
}

export type StatusVariant = 'green' | 'red' | 'yellow' | 'muted';

const VARIANTS: Record<WorkloadCondition, StatusVariant> = {
  running: 'green',
  failing: 'red',
  degraded: 'yellow',
  stopped: 'muted',
  unknown: 'muted',
};

/** Badge colour for a status. Green is reserved for observed running. */
export function statusVariant(status: string | null | undefined): StatusVariant {
  return VARIANTS[workloadCondition(status)];
}

export function countByCondition(workloads: Array<{ status: string }>): Record<WorkloadCondition, number> {
  const counts: Record<WorkloadCondition, number> = { running: 0, failing: 0, degraded: 0, stopped: 0, unknown: 0 };
  for (const workload of workloads) counts[workloadCondition(workload.status)] += 1;
  return counts;
}
