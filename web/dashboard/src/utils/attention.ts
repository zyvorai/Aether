// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/**
 * What an operator should look at first, derived only from API responses that
 * actually arrived. A source that failed is reported as unchecked; it is never
 * treated as "nothing wrong".
 */

import type { AppView, Event, HealthSummary, SecretSummary, WorkloadResponse } from '../types/api';
import type { AttentionItem as AttentionEntry, AttentionQueue } from '../types/operator';
import { attentionTarget } from './operator';
import { countByCondition, workloadCondition } from './workloadStatus';

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
  /** The server's queue from recorded data: incidents, failed operations, unapplied specs. */
  operator: AttentionQueue | null;
}

export const SOURCE_LABELS: Record<keyof AttentionSources, string> = {
  workloads: 'workloads',
  events: 'events',
  secrets: 'secrets',
  health: 'health checks',
  operator: 'recorded operations',
};

function eventSeverity(severity: string): AttentionSeverity | null {
  const s = severity.toLowerCase();
  if (s.startsWith('crit') || s === 'error') return 'critical';
  if (s.startsWith('warn')) return 'warning';
  return null;
}

const incidentNumber = (event: Event) => /^incident (\d+):/i.exec(event.message)?.[1];

/**
 * Drops events that no longer describe a problem: recoveries, and openings of
 * incidents that have since ended. The server queue lists every open incident,
 * so when it loaded, an opening it does not list is over.
 */
function stillCurrent(events: Event[], operator: AttentionQueue | null): Event[] {
  const ended = new Set(events.filter((e) => /^incident resolved$/i.test(e.title)).map(incidentNumber));
  const stillOpen = new Set((operator?.items ?? []).filter((i) => i.kind === 'open-incident').map((i) => i.workload));
  return events.filter((event) => {
    if (/^incident resolved$/i.test(event.title)) return false;
    if (!/^incident opened$/i.test(event.title)) return true;
    const number = incidentNumber(event);
    if (number !== undefined && ended.has(number)) return false;
    return operator === null || stillOpen.has(event.workload);
  });
}

const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`;

/** Which events only announce something a server queue entry already reports. */
const ANNOUNCEMENTS: Record<string, RegExp> = {
  'open-incident': /^incident opened$/i,
  'failed-operation': /^(build|deploy|update|stop|delete) failed$/i,
};

function announces(event: Event, entry: AttentionEntry): boolean {
  return event.workload === entry.workload && (ANNOUNCEMENTS[entry.kind]?.test(event.title) ?? false);
}

export function buildAttention({ workloads, events, secrets, health, operator }: AttentionSources): AttentionItem[] {
  const items: AttentionItem[] = [];
  // Informational entries (holds, stale data) stay on the Operations Desk.
  const recorded = (operator?.items ?? []).filter((entry) => entry.level !== 'info');
  // Recoveries count as evidence even when somebody has acknowledged them.
  const allOpen = stillCurrent(events ?? [], operator).filter((event) => !event.acknowledged && eventSeverity(event.severity));
  // One problem, one row: the server's entry says more than the event that announced it.
  const open = allOpen.filter((event) => !recorded.some((entry) => announces(event, entry)));
  const escalates = (entry: AttentionEntry) =>
    entry.level === 'critical' || allOpen.some((event) => announces(event, entry) && eventSeverity(event.severity) === 'critical');
  const covered = new Set<Event>();
  const listed = new Set<AttentionEntry>();

  for (const workload of workloads ?? []) {
    const condition = workloadCondition(workload.status);
    if (condition !== 'failing' && condition !== 'degraded') continue;
    const related = open.filter((event) => event.workload === workload.name);
    related.forEach((event) => covered.add(event));
    const latest = related[0];
    const known = recorded.filter((entry) => entry.workload === workload.name);
    known.forEach((entry) => listed.add(entry));
    const detail = [
      ...known.map((entry) => `${entry.title}: ${entry.detail}`),
      ...(latest
        ? [`${latest.title}: ${latest.message}${related.length > 1 ? ` (+${plural(related.length - 1, 'more open event')})` : ''}`]
        : []),
    ].join('; ');
    items.push({
      id: `workload-${workload.name}`,
      severity: condition === 'failing' || known.some(escalates) ? 'critical' : 'warning',
      title: `${workload.name} is ${workload.status}`,
      detail: detail || (workload.status_detail ?? `${workload.runtime} workload`),
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

  // What the server recorded about workloads that have no row of their own yet.
  for (const entry of recorded) {
    if (listed.has(entry)) continue;
    const target = attentionTarget(entry);
    items.push({
      id: `operator-${entry.kind}-${entry.workload ?? ''}-${entry.title}`,
      severity: escalates(entry) ? 'critical' : 'warning',
      title: entry.workload ? `${entry.workload}: ${entry.title}` : entry.title,
      detail: entry.detail,
      view: target ? 'operations' : 'audit',
      query: target
        ? {
            tab: target.tab,
            ...(target.workload ? { workload: target.workload } : {}),
            ...(target.incident ? { incident: target.incident } : {}),
          }
        : undefined,
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
  // A list that loaded can still hold workloads nobody has looked at lately.
  const blind = countByCondition(sources.workloads ?? []).unknown;
  if (blind > 0) {
    return { tone: 'unknown', headline: `${plural(blind, 'workload')} ${blind === 1 ? 'has' : 'have'} no current status`, checked, unchecked };
  }
  return { tone: 'clear', headline: 'Nothing needs attention', checked, unchecked };
}
