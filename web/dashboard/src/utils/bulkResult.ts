// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

export interface BulkOutcome {
  name: string;
  success: boolean;
  error?: string | null;
}

/** What to tell the operator after acting on several workloads: failures are never rounded up. */
export function bulkSummary(action: 'start' | 'stop', outcomes: BulkOutcome[]): { message: string; type: 'success' | 'error' } {
  const failed = outcomes.filter((o) => !o.success);
  const total = outcomes.length;
  if (failed.length === 0) {
    return { message: `Bulk ${action} succeeded for ${total} workload(s)`, type: 'success' };
  }
  const first = failed[0];
  return {
    message: `Bulk ${action} failed for ${failed.length} of ${total}: ${failed.map((o) => o.name).join(', ')}. ${first.name}: ${first.error?.trim() || 'unknown error'}`,
    type: 'error',
  };
}
