// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import type { Timeline, TimelineEntry } from '../types/timeline';
import {
  coverageLine,
  diffPath,
  entryTone,
  formatOnly,
  groupByDay,
  parseOnly,
  parseRange,
  timelinePath,
  toggleCategory,
} from './timeline';

function entry(over: Partial<TimelineEntry>): TimelineEntry {
  return {
    at: '2026-03-01T12:00:00Z',
    category: 'change',
    kind: 'deploy',
    level: 'info',
    title: 'Deploy revision 2 on podman',
    detail: '',
    actor: null,
    revision: null,
    previous_revision: null,
    incident: null,
    fields: [],
    ...over,
  };
}

describe('workload timeline', () => {
  it('reads filters and ranges from the address', () => {
    expect(parseRange('7d')).toBe('7d');
    expect(parseRange('1y')).toBe('24h');
    expect(parseOnly('health, incidents,bogus')).toEqual(['health', 'incident']);
    expect(formatOnly(['incident', 'change'])).toBe('changes,incidents');
    expect(parseOnly('')).toEqual([]);
  });

  it('treats selecting every category as no filter', () => {
    expect(toggleCategory([], 'note')).toEqual(['note']);
    expect(toggleCategory(['note'], 'note')).toEqual([]);
    expect(toggleCategory(['change', 'health', 'incident', 'note'], 'hold')).toEqual([]);
  });

  it('builds API paths', () => {
    expect(timelinePath('web', '24h', [])).toBe('/workloads/web/timeline?since=24h&limit=300');
    expect(timelinePath('web', '7d', ['hold'])).toBe('/workloads/web/timeline?since=7d&only=holds&limit=300');
    expect(diffPath('web', entry({ revision: 3, previous_revision: 2 }))).toBe('/workloads/web/revision-diff?from=2&to=3');
    expect(diffPath('web', entry({ revision: 1 }))).toBeNull();
  });

  it('groups by day without reordering', () => {
    const now = new Date(2026, 2, 3, 15, 0).getTime();
    const at = (day: number, hour: number) => new Date(2026, 2, day, hour).toISOString();
    const days = groupByDay([entry({ at: at(3, 14) }), entry({ at: at(3, 1) }), entry({ at: at(2, 23) }), entry({ at: at(1, 8) })], now);
    expect(days.map((d) => [d.day, d.entries.length])).toEqual([
      ['Today', 2],
      ['Yesterday', 1],
      ['2026-03-01', 1],
    ]);
  });

  it('marks gaps as unknown, not as good or bad', () => {
    expect(entryTone(entry({ kind: 'checks-gap', level: 'warning', category: 'health' }))).toBe('unknown');
    expect(entryTone(entry({ kind: 'became-not-ready', level: 'critical' }))).toBe('bad');
    expect(entryTone(entry({ kind: 'became-ready' }))).toBe('ok');
    expect(entryTone(entry({ category: 'hold', kind: 'hold-placed' }))).toBe('idle');
  });

  it('does not let an empty period read as healthy', () => {
    const t = { health: { checks: 0, ready: 0, retained_since: null, last_check_at: null } } as Timeline;
    expect(coverageLine(t)).toContain('No health check has ever');
    expect(coverageLine({ ...t, health: { ...t.health, retained_since: 'x' } })).toContain('in this period');
    expect(coverageLine({ ...t, health: { checks: 1, ready: 0, retained_since: 'x', last_check_at: 'x' } })).toBe(
      '0 of 1 health check in this period found it ready.',
    );
  });
});
