// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { SlaTarget } from '../types/api';
import { apiFetchSettled } from './api';

/**
 * Configured targets by workload name, from one request: a workload without a
 * target is a normal answer, not a 404 per workload. Null when they could not
 * be loaded.
 */
export async function loadSlaTargets(): Promise<Record<string, SlaTarget> | null> {
  const result = await apiFetchSettled<SlaTarget[]>('/sla');
  if (!result.ok) return null;
  return Object.fromEntries(result.data.map((target) => [target.workload, target]));
}
