// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import type { BoardCell, BoardData, BoardTile } from '../types/board';
import {
  ageLabel,
  cellTitle,
  checkAge,
  filterCounts,
  hasGaps,
  liveTiles,
  parseFilter,
  parseWindow,
  readyLabel,
  sortTiles,
  stateDetail,
  stripSummary,
  tileTone,
  verdict,
} from './board';

const NOW = Date.parse('2026-03-01T12:00:00Z');

function tile(name: string, over: Partial<BoardTile> = {}): BoardTile {
  return {
    name,
    runtime: 'podman',
    image: `${name}:latest`,
    state: 'ready',
    last_check: { at: '2026-03-01T11:59:30Z', state: 'running', ready: true, restart_count: 0 },
    stopped_since: null,
    error: null,
    strip: [],
    ready_pct: 100,
    checks: 10,
    open_incidents: [],
    hold: null,
    last_change: null,
    depends_on: [],
    dependents: [],
    ...over,
  };
}

function board(workloads: BoardTile[]): BoardData {
  return {
    generated_at: '2026-03-01T12:00:00Z',
    window: '24h',
    bucket_secs: 3600,
    observe_secs: 60,
    stale_after_secs: 300,
    incident_threshold: 2,
    workloads,
    unavailable: [],
  };
}

const incident = [{ id: 3, opened_at: '2026-03-01T11:00:00Z', failed_checks: 4, acknowledged: false }];
const cell = (over: Partial<BoardCell>): BoardCell => ({
  start: '2026-03-01T11:00:00Z',
  state: 'up',
  checks: 60,
  ready: 60,
  covered: 1,
  ...over,
});

describe('status board', () => {
  it('falls back for unknown query values', () => {
    expect(parseWindow('7d')).toBe('7d');
    expect(parseWindow('9y')).toBe('24h');
    expect(parseFilter('held')).toBe('held');
    expect(parseFilter('nope')).toBe('all');
  });

  it('measures age from the server clock plus time since the fetch', () => {
    const b = board([tile('web')]);
    // The viewer's clock may be far off; only the elapsed time is taken from it.
    expect(checkAge(b.workloads[0], b, 1_000, 11_000)).toBe(40);
    expect(checkAge(tile('new', { last_check: null }), b, 0, 0)).toBeNull();
  });

  it('turns an old ready or not-ready check into stale when updates stop', () => {
    const b = board([
      tile('web'),
      tile('api', { state: 'not_ready' }),
      tile('db', { state: 'stopped' }),
    ]);
    expect(liveTiles(b, NOW, NOW).map((t) => t.state)).toEqual(['ready', 'not_ready', 'stopped']);
    expect(liveTiles(b, NOW, NOW + 600_000).map((t) => t.state)).toEqual(['stale', 'stale', 'stopped']);
  });

  it('puts incidents and failures first', () => {
    const tiles = [
      tile('a-ready'),
      tile('b-stopped', { state: 'stopped' }),
      tile('c-nodata', { state: 'no_data' }),
      tile('d-down', { state: 'not_ready' }),
      tile('e-incident', { state: 'not_ready', open_incidents: incident }),
    ];
    expect(sortTiles(tiles).map((t) => t.name)).toEqual(['e-incident', 'd-down', 'c-nodata', 'b-stopped', 'a-ready']);
    expect(filterCounts(tiles)).toMatchObject({ all: 5, attention: 3, incidents: 1, ready: 1, stopped: 1, held: 0 });
  });

  it('keeps a workload with an open incident red even after one ready check', () => {
    expect(tileTone(tile('web', { open_incidents: incident }))).toBe('bad');
    expect(tileTone(tile('web'))).toBe('ok');
  });

  it('only says all ready when there is current data for everything', () => {
    expect(verdict([tile('a'), tile('b')], []).title).toBe('All 2 workloads ready');
    expect(verdict([tile('a'), tile('b', { state: 'stale' })], [])).toMatchObject({ tone: 'warn', title: 'No current data for 1 workload' });
    expect(verdict([tile('a')], ['holds: bad file'])).toMatchObject({ tone: 'warn', title: 'The board is incomplete' });
    expect(verdict([tile('a', { state: 'not_ready' }), tile('b', { state: 'no_data' })], [])).toEqual({
      tone: 'bad',
      title: '1 workload needs attention',
      detail: '0 ready, 1 without current data',
    });
    expect(verdict([tile('a', { state: 'stopped' })], []).tone).toBe('idle');
    expect(verdict([], []).tone).toBe('unknown');
  });

  it('describes cells without calling missing data healthy', () => {
    expect(cellTitle(cell({ state: 'no_data', checks: 0, ready: 0, covered: 0 }), 3600)).toContain('no checks recorded');
    const gappy = cell({ covered: 0.25 });
    expect(hasGaps(gappy)).toBe(true);
    expect(cellTitle(gappy, 3600)).toContain('75% of the period has no data');
    expect(hasGaps(cell({}))).toBe(false);
    expect(stripSummary(tile('web', { strip: [cell({}), cell({ state: 'no_data', checks: 0 })] }))).toBe(
      'web uptime strip, 2 periods: 1 ready, 1 no data',
    );
  });

  it('never rounds a failure up to 100%', () => {
    expect(readyLabel(tile('web', { ready_pct: 99.999, checks: 100000 }))).toBe('99.9% of 100000 checks ready');
    expect(readyLabel(tile('web', { ready_pct: 100, checks: 1 }))).toBe('100% of 1 check ready');
    expect(readyLabel(tile('web', { ready_pct: null, checks: 0 }))).toBe('no data');
  });

  it('explains each state', () => {
    expect(ageLabel(2)).toBe('checked just now');
    expect(ageLabel(120)).toBe('checked 2m ago');
    expect(stateDetail({ ...tile('web', { state: 'stale' }), age: 7200 }, NOW)).toBe(
      'Last checked 2h ago (ready then). Current state unknown.',
    );
    expect(stateDetail({ ...tile('web', { state: 'unreadable', error: 'no podman' }), age: 10 }, NOW)).toBe('no podman');
  });
});
