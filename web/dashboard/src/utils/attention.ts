// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/**
 * What an operator should look at first, derived only from API responses that
 * actually arrived. A source that failed is reported as unchecked; it is never
 * treated as "nothing wrong".
 */

import type { AppView, Event, HealthSummary, SecretSummary, WorkloadResponse } from '../types/api';
import { workloadCondition } from './workloadStatus';

export type AttentionSeverity = 'critical' | 'warning';

export interface AttentionItem {
  id: string;
  severity: AttentionSeverity;
  title: string;
  detail: string;
  view: AppView;
  query?: Record<string, string>;
}

/** Pass null for a source that could not be loaded. */
export interface AttentionSources {
  workloads: WorkloadResponse[] | null;
  events: Event[] | null;
  secrets: SecretSummary[] | null;
  health: HealthSummary | null;
}

export const SOURCE_LABELS: Record<keyof AttentionSources, string> = {
  workloads: 'workloads',
  events: 'events',
  secrets: 'secrets',
  health: 'health checks',
};

function eventSeverity(severity: string): AttentionSeverity | null {
  const s = severity.toLowerCase();
  if (s.startsWith('crit') || s === 'error') return 'critical';
  if (s.startsWith('warn')) return 'warning';
  return null;
}

const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`;

export function buildAttention({ workloads, events, secrets, health }: AttentionSources): AttentionItem[] {
  const items: AttentionItem[] = [];
  const open = (events ?? []).filter((event) => !event.acknowledged && eventSeverity(event.severity));
  const covered = new Set<Event>();

  for (const workload of workloads ?? []) {
    const condition = workloadCondition(workload.status);
    if (condition !== 'failing' && condition !== 'degraded') continue;
    const related = open.filter((event) => event.workload === workload.name);
    related.forEach((event) => covered.add(event));
    const latest = related[0];
    items.push({
      id: `workload-${workload.name}`,
      severity: condition === 'failing' ? 'critical' : 'warning',
      title: `${workload.name} is ${workload.status}`,
      detail: latest
        ? `${latest.title}: ${latest.message}${related.length > 1 ? ` (+${plural(related.length - 1, 'more open event')})` : ''}`
        : `${workload.runtime} workload`,
      view: 'workloads',
      query: { workload: workload.name },
    });
  }

  const due = (secrets ?? []).filter((secret) => secret.needs_rotation);
  // The rotation entry below already covers events that only announce the same thing.
  const aboutDueSecret = (event: Event) =>
    due.some((secret) => event.message.includes(secret.name)) && /rotat/i.test(`${event.title} ${event.message}`);
  // Remaining open events, one entry per title and workload so a repeating alert is one line.
  const grouped = new Map<string, Event[]>();
  for (const event of open) {
    if (covered.has(event) || aboutDueSecret(event)) continue;
    const key = `${event.title}|${event.workload ?? ''}`;
    grouped.set(key, [...(grouped.get(key) ?? []), event]);
  }
  for (const [key, group] of grouped) {
    const severity = group.some((e) => eventSeverity(e.severity) === 'critical') ? 'critical' : 'warning';
    const first = group[0];
    items.push({
      id: `event-${key}`,
      severity,
      title: group.length > 1 ? `${first.title} (${group.length} times)` : first.title,
      detail: first.message,
      view: 'events',
      query: first.workload ? { workload: first.workload } : { q: first.title },
    });
  }

  if (due.length > 0) {
    items.push({
      id: 'secrets-rotation',
      severity: 'warning',
      title: `${plural(due.length, 'secret')} due for rotation`,
      detail: due.slice(0, 3).map((s) => s.name).join(', ') + (due.length > 3 ? ` and ${due.length - 3} more` : ''),
      view: 'secrets',
    });
  }

  if (health && health.circuits_open > 0) {
    items.push({
      id: 'circuits-open',
      severity: 'critical',
      title: `${plural(health.circuits_open, 'circuit breaker')} open`,
      detail: 'The orchestrator has stopped restarting these workloads.',
      view: 'health',
    });
  }

  const rank = (item: AttentionItem) => (item.severity === 'critical' ? 0 : 1);
  return items.sort((a, b) => rank(a) - rank(b));
}

export interface AttentionVerdict {
  tone: 'critical' | 'warning' | 'clear' | 'unknown';
  headline: string;
  checked: string[];
  unchecked: string[];
}

export function attentionVerdict(items: AttentionItem[], sources: AttentionSources): AttentionVerdict {
  const keys = Object.keys(SOURCE_LABELS) as Array<keyof AttentionSources>;
  const checked = keys.filter((key) => sources[key] !== null).map((key) => SOURCE_LABELS[key]);
  const unchecked = keys.filter((key) => sources[key] === null).map((key) => SOURCE_LABELS[key]);
  if (items.length > 0) {
    return {
      tone: items.some((i) => i.severity === 'critical') ? 'critical' : 'warning',
      headline: `${plural(items.length, 'thing')} ${items.length === 1 ? 'needs' : 'need'} attention`,
      checked,
      unchecked,
    };
  }
  if (unchecked.length > 0) {
    return { tone: 'unknown', headline: 'Status is incomplete', checked, unchecked };
  }
  return { tone: 'clear', headline: 'Nothing needs attention', checked, unchecked };
}
