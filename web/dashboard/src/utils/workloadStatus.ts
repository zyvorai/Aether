// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

export type WorkloadCondition = 'running' | 'failing' | 'degraded' | 'stopped' | 'unknown';

const FAILING = ['failed', 'error', 'crashloop', 'oomkilled', 'imagepullbackoff', 'errimagepull', 'evicted'];
const DEGRADED = ['degraded', 'unhealthy', 'notready', 'not ready', 'unschedulable', 'pending', 'terminating'];

/** One reading of the status strings the runtimes report, shared by lists and the Overview. */
export function workloadCondition(status: string | null | undefined): WorkloadCondition {
  const s = (status ?? '').toLowerCase();
  if (FAILING.some((word) => s.includes(word))) return 'failing';
  if (DEGRADED.some((word) => s.includes(word))) return 'degraded';
  if (s === 'stopped' || s === 'exited') return 'stopped';
  if (s === 'running' || s === 'healthy' || s === 'ready' || s.includes('deployed')) return 'running';
  return 'unknown';
}

export function needsAttention(status: string | null | undefined): boolean {
  const condition = workloadCondition(status);
  return condition === 'failing' || condition === 'degraded';
}
