// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/** Pure helpers for the Operations Desk page. */

import type { AttentionItem, Impact, Level, Preview, PreviewAction } from '../types/operator';

export const OPERATOR_TABS = ['attention', 'changes', 'incidents', 'workload', 'report'] as const;
export type OperatorTab = (typeof OPERATOR_TABS)[number];

export function parseTab(value: string): OperatorTab {
  return (OPERATOR_TABS as readonly string[]).includes(value) ? (value as OperatorTab) : 'attention';
}

export type Tone = 'green' | 'red' | 'yellow' | 'blue' | 'muted';

export function levelTone(level: Level): Tone {
  if (level === 'critical') return 'red';
  if (level === 'warning') return 'yellow';
  return 'blue';
}

export function verdictTone(verdict: Preview['verdict']): Tone {
  if (verdict === 'blocked') return 'red';
  if (verdict === 'caution') return 'yellow';
  return 'green';
}

export function impactLabel(impact: Impact): string {
  switch (impact) {
    case 'none':
      return 'no effect';
    case 'in_place':
      return 'update in place';
    case 'rolling':
      return 'rolling restart';
    case 'recreate':
      return 'replace';
  }
}

/** A missing measurement is shown as missing, never as 0% or 100%. */
export function formatReadyPct(pct: number | null | undefined): string {
  return pct === null || pct === undefined ? 'no data' : `${pct.toFixed(2)}%`;
}

export function formatMinutes(minutes: number | null | undefined): string {
  if (minutes === null || minutes === undefined) return 'no recovered incident';
  return `${minutes.toFixed(1)} min`;
}

export function formatSeconds(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
  return `${Math.floor(s / 86400)}d ${Math.floor((s % 86400) / 3600)}h`;
}

export function queryString(params: Record<string, string | number | undefined>): string {
  const qs = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== '') qs.set(key, String(value));
  }
  const out = qs.toString();
  return out ? `?${out}` : '';
}

export function workloadPath(name: string, rest: string): string {
  return `/workloads/${encodeURIComponent(name)}/${rest}`;
}

/** Where the dashboard shows more about an attention item. */
export function attentionTarget(item: AttentionItem): { tab: OperatorTab; workload?: string; incident?: string } | null {
  const workload = item.workload ?? undefined;
  if (item.kind === 'open-incident') {
    const incident = /Incident (\d+)/.exec(item.title)?.[1];
    return { tab: 'incidents', incident };
  }
  if (item.kind === 'failed-operation') return { tab: 'changes', workload };
  return workload ? { tab: 'workload', workload } : null;
}

export type PreviewKind = PreviewAction['action'];

export interface PreviewForm {
  kind: PreviewKind;
  yaml: string;
  target: string;
  revision: string;
}

export function buildPreviewAction(form: PreviewForm): { action: PreviewAction } | { error: string } {
  switch (form.kind) {
    case 'update':
      return form.yaml.trim()
        ? { action: { action: 'update', yaml: form.yaml } }
        : { error: 'Paste the edited spec first.' };
    case 'migrate':
      return form.target
        ? { action: { action: 'migrate', target: form.target } }
        : { error: 'Choose a target runtime.' };
    case 'rollback': {
      const text = form.revision.trim();
      if (!text) return { action: { action: 'rollback' } };
      const revision = Number(text);
      return Number.isInteger(revision) && revision > 0
        ? { action: { action: 'rollback', revision } }
        : { error: 'A revision is a positive whole number.' };
    }
    default:
      return { action: { action: form.kind } };
  }
}

/** `apiPost` reports an empty error body (a refused role) as a JSON parse error. */
export function friendlyError(error: string | null | undefined): string {
  if (!error) return 'The request failed.';
  if (error.startsWith('SyntaxError')) {
    return 'The server refused the request. Check that your role allows this action.';
  }
  return error;
}
