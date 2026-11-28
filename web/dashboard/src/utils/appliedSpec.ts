// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { RevisionSummary } from '../types/operator';
import { apiFetchSettled } from './api';
import { workloadPath } from './operator';

/** A recorded revision as the server shows it: secret values are already redacted. */
export interface AppliedRevision extends RevisionSummary {
  spec_yaml: string;
}

export type AppliedSpecResult =
  | { state: 'recorded'; revision: AppliedRevision }
  | { state: 'none' }
  | { state: 'failed'; detail: string };

/** The spec Aether last applied successfully to a workload it manages. */
export async function loadAppliedSpec(name: string): Promise<AppliedSpecResult> {
  const failed = (detail?: string): AppliedSpecResult => ({ state: 'failed', detail: detail ?? 'the server did not answer' });
  const list = await apiFetchSettled<RevisionSummary[]>(workloadPath(name, 'revisions'));
  if (!list.ok) return failed(list.detail);
  const latest = list.data[list.data.length - 1];
  if (!latest) return { state: 'none' };
  const view = await apiFetchSettled<AppliedRevision>(workloadPath(name, `revisions/${latest.id}`));
  return view.ok ? { state: 'recorded', revision: view.data } : failed(view.detail);
}
