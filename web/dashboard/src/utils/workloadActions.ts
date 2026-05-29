// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { apiDelete, apiPost } from './api';
import { parseClusterWorkload } from './parseClusterWorkload';
import type { ApiResponse, WorkloadResponse } from '../types/api';

export function isAetherManagedWorkload(w: WorkloadResponse): boolean {
  return (w.source ?? 'aether') === 'aether';
}

export async function restartWorkload(w: WorkloadResponse): Promise<ApiResponse<unknown>> {
  if (isAetherManagedWorkload(w)) {
    return apiPost(`/workloads/${encodeURIComponent(w.name)}/restart`);
  }
  const ref = parseClusterWorkload(w);
  if (!ref) {
    return { success: false, data: null, error: 'Cannot resolve cluster workload reference' };
  }
  return apiPost('/cluster/action', { ...ref, action: 'restart' });
}

export async function rollbackWorkload(w: WorkloadResponse): Promise<ApiResponse<unknown>> {
  return apiPost(`/workloads/${encodeURIComponent(w.name)}/rollback`, {});
}

export async function reconcileDrift(w: WorkloadResponse): Promise<ApiResponse<unknown>> {
  return apiPost(`/drift/${encodeURIComponent(w.name)}/reconcile`, {});
}

export async function deleteWorkload(w: WorkloadResponse): Promise<ApiResponse<unknown>> {
  if (isAetherManagedWorkload(w)) {
    return apiDelete(`/workloads/${encodeURIComponent(w.name)}`, { label: `Delete workload "${w.name}"` });
  }
  const ref = parseClusterWorkload(w);
  if (!ref) {
    return { success: false, data: null, error: 'Cannot resolve cluster workload reference' };
  }
  return apiPost('/cluster/action', { ...ref, action: 'delete' });
}
