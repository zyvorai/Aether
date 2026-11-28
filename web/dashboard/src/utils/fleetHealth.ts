// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/** How the Command Center words the briefing's observed fleet health. */

export interface FleetCounts {
  total: number;
  running: number;
  unhealthy: number;
  stopped: number;
  unknown: number;
}

/** A percentage only exists when at least one workload has a current check. */
export function fleetPercent(pct: number | null | undefined): string {
  return typeof pct === 'number' ? `${pct.toFixed(0)}%` : 'Not checked';
}

export function fleetBreakdown(fleet: FleetCounts | undefined): string {
  if (!fleet || fleet.total === 0) return 'Deploy workloads to begin monitoring';
  const parts = [`${fleet.running} running`];
  if (fleet.unhealthy > 0) parts.push(`${fleet.unhealthy} in trouble`);
  if (fleet.stopped > 0) parts.push(`${fleet.stopped} stopped`);
  if (fleet.unknown > 0) parts.push(`${fleet.unknown} not checked`);
  return parts.join(' · ');
}

export function fleetSubtitle(issueCount: number, pct: number | null | undefined, fleet: FleetCounts | undefined): string {
  const at = typeof pct === 'number' ? `Fleet at ${pct.toFixed(0)}% health` : 'Fleet health not checked';
  if (issueCount > 0) {
    return `${issueCount} signal${issueCount === 1 ? '' : 's'} need attention · ${at}`;
  }
  if (!fleet || fleet.total === 0) {
    return 'Connect workloads to activate fleet intelligence and proactive monitoring';
  }
  if (fleet.unknown > 0) {
    return `${fleet.unknown} of ${fleet.total} workload${fleet.total === 1 ? '' : 's'} ${fleet.unknown === 1 ? 'has' : 'have'} no current health check`;
  }
  if (typeof pct !== 'number') return 'Every workload is stopped on request';
  if (pct >= 99) return 'Every checked workload is running';
  return `Fleet operating at ${pct.toFixed(0)}% health`;
}
