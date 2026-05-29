// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { reconcileDrift, restartWorkload, rollbackWorkload } from './workloadActions';
import type { ApiResponse, DiagnoseRecommendation, WorkloadResponse } from '../types/api';

export const TROUBLESHOOT_ACTION_LABELS: Record<string, string> = {
  open_logs: 'Open Logs',
  fix_image: 'Fix Image',
  check_network: 'Check Network',
  view_nodes: 'View Nodes',
  rollback_secret: 'Rollback Secret',
  increase_memory: 'Increase Memory',
  reconcile_drift: 'Reconcile Drift',
  restart: 'Restart Workload',
};

export function isApplyableRecommendation(rec: DiagnoseRecommendation): boolean {
  if (rec.applyable === false) return false;
  if (rec.applyable === true) return true;
  return ['restart', 'reconcile_drift', 'rollback_secret'].includes(rec.action);
}

export async function applyTroubleshootAction(
  workload: WorkloadResponse,
  action: string,
): Promise<ApiResponse<unknown>> {
  switch (action) {
    case 'restart':
      return restartWorkload(workload);
    case 'reconcile_drift':
      return reconcileDrift(workload);
    case 'rollback_secret':
      return rollbackWorkload(workload);
    default:
      return { success: false, data: null, error: `Action "${action}" is not applyable` };
  }
}
