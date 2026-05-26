// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { WorkloadResponse } from '../types/api';

export function isAetherManaged(workload: WorkloadResponse): boolean {
  return (workload.source ?? 'aether') === 'aether';
}

export function countAetherManaged(workloads: WorkloadResponse[]): number {
  return workloads.filter(isAetherManaged).length;
}

/** Aether-managed workloads first, then newest first within each group. */
export function sortWorkloadsForDisplay(workloads: WorkloadResponse[]): WorkloadResponse[] {
  return [...workloads].sort((a, b) => {
    const aManaged = isAetherManaged(a) ? 0 : 1;
    const bManaged = isAetherManaged(b) ? 0 : 1;
    if (aManaged !== bManaged) return aManaged - bManaged;
    const aTime = Date.parse(a.created_at) || 0;
    const bTime = Date.parse(b.created_at) || 0;
    return bTime - aTime;
  });
}

export function parseCreatedWorkloadName(message: string | undefined): string | null {
  if (!message) return null;
  const match = message.match(/Workload\s+(.+?)\s+created/i);
  return match?.[1] ?? null;
}
