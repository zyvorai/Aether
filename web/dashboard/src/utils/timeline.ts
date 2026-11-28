// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/** Pure logic of the workload timeline. */

import type { Timeline, TimelineCategory, TimelineEntry } from '../types/timeline';
import { queryString, workloadPath } from './operator';

export const TIMELINE_CATEGORIES: { key: TimelineCategory; label: string; param: string }[] = [
  { key: 'change', label: 'Changes', param: 'changes' },
  { key: 'health', label: 'Health', param: 'health' },
  { key: 'incident', label: 'Incidents', param: 'incidents' },
  { key: 'note', label: 'Notes', param: 'notes' },
  { key: 'hold', label: 'Holds', param: 'holds' },
];

export const TIMELINE_RANGES = ['1h', '24h', '7d', '30d'] as const;
export type TimelineRange = (typeof TIMELINE_RANGES)[number];

export function parseRange(value: string): TimelineRange {
  return (TIMELINE_RANGES as readonly string[]).includes(value) ? (value as TimelineRange) : '24h';
}

/** Read `health,incidents` from the address; unknown words are ignored. */
export function parseOnly(value: string): TimelineCategory[] {
  const wanted = new Set(value.split(',').map((w) => w.trim()));
  return TIMELINE_CATEGORIES.filter((c) => wanted.has(c.param) || wanted.has(c.key)).map((c) => c.key);
}

export function formatOnly(only: TimelineCategory[]): string {
  return TIMELINE_CATEGORIES.filter((c) => only.includes(c.key))
    .map((c) => c.param)
    .join(',');
}

/** Switch one category; nothing selected means everything. */
export function toggleCategory(only: TimelineCategory[], key: TimelineCategory): TimelineCategory[] {
  const next = only.includes(key) ? only.filter((k) => k !== key) : [...only, key];
  return next.length === TIMELINE_CATEGORIES.length ? [] : next;
}

export function timelinePath(name: string, range: TimelineRange, only: TimelineCategory[]): string {
  return workloadPath(name, `timeline${queryString({ since: range, only: formatOnly(only), limit: 300 })}`);
}

/** The change from the replaced revision to this one, when both are recorded. */
export function diffPath(name: string, entry: TimelineEntry): string | null {
  if (entry.revision === null || entry.previous_revision === null) return null;
  return workloadPath(name, `revision-diff${queryString({ from: entry.previous_revision, to: entry.revision })}`);
}

export interface TimelineDay {
  day: string;
  entries: TimelineEntry[];
}

function localDay(d: Date): string {
  const two = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())}`;
}

/** Entries under their day in the viewer's time zone, keeping their order. */
export function groupByDay(entries: TimelineEntry[], now: number): TimelineDay[] {
  const today = localDay(new Date(now));
  const yesterday = localDay(new Date(new Date(now).setDate(new Date(now).getDate() - 1)));
  const days: TimelineDay[] = [];
  for (const entry of entries) {
    const at = new Date(entry.at);
    const key = Number.isNaN(at.getTime()) ? 'Unknown date' : localDay(at);
    const day = key === today ? 'Today' : key === yesterday ? 'Yesterday' : key;
    const last = days[days.length - 1];
    if (last?.day === day) last.entries.push(entry);
    else days.push({ day, entries: [entry] });
  }
  return days;
}

export function entryTime(at: string): string {
  const d = new Date(at);
  return Number.isNaN(d.getTime()) ? at : d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
}

export type EntryTone = 'ok' | 'bad' | 'warn' | 'idle' | 'unknown' | 'info';

/** The marker colour of an entry; the title always says the same in words. */
export function entryTone(entry: TimelineEntry): EntryTone {
  if (entry.kind === 'checks-gap') return 'unknown';
  if (entry.level === 'critical') return 'bad';
  if (entry.level === 'warning') return 'warn';
  if (entry.kind === 'became-ready' || entry.kind === 'incident-resolved') return 'ok';
  if (entry.category === 'hold' || entry.kind === 'became-stopped') return 'idle';
  return 'info';
}

/** What stands behind the timeline, so an empty one is not read as a quiet one. */
export function coverageLine(t: Timeline): string {
  if (t.health.retained_since === null) return 'No health check has ever been recorded for this workload.';
  if (t.health.checks === 0) return 'No health check was recorded in this period.';
  return `${t.health.ready} of ${t.health.checks} health check${t.health.checks === 1 ? '' : 's'} in this period found it ready.`;
}
