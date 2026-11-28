// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/** Pure logic of the status board: what to show, in what order, and how old it is. */

import type { BoardCell, BoardData, BoardTile, BoardWindow, CellState, TileState } from '../types/board';
import { formatSeconds } from './operator';

export const BOARD_WINDOWS: { key: BoardWindow; label: string }[] = [
  { key: '1h', label: '1 hour' },
  { key: '6h', label: '6 hours' },
  { key: '24h', label: '24 hours' },
  { key: '7d', label: '7 days' },
  { key: '30d', label: '30 days' },
];

export function parseWindow(value: string): BoardWindow {
  return BOARD_WINDOWS.find((w) => w.key === value)?.key ?? '24h';
}

export type BoardTone = 'ok' | 'bad' | 'warn' | 'idle' | 'unknown';

export const STATE_META: Record<TileState, { label: string; tone: BoardTone; rank: number }> = {
  not_ready: { label: 'Not ready', tone: 'bad', rank: 0 },
  unreadable: { label: 'Cannot be observed', tone: 'warn', rank: 2 },
  unknown: { label: 'Unknown', tone: 'warn', rank: 2 },
  stale: { label: 'Stale', tone: 'unknown', rank: 3 },
  no_data: { label: 'No data', tone: 'unknown', rank: 4 },
  stopped: { label: 'Stopped on purpose', tone: 'idle', rank: 5 },
  ready: { label: 'Ready', tone: 'ok', rank: 6 },
};

/** A tile as it is true right now, which may differ from when it was fetched. */
export interface LiveTile extends BoardTile {
  /** Seconds since the last check, or `null` when there never was one. */
  age: number | null;
}

/**
 * Seconds since the last check: its age when the server answered, plus the time
 * since then. Using the server's clock for the first part keeps a screen with a
 * wrong clock from inventing or hiding staleness.
 */
export function checkAge(tile: BoardTile, board: BoardData, fetchedAt: number, now: number): number | null {
  if (!tile.last_check) return null;
  const atServer = (Date.parse(board.generated_at) - Date.parse(tile.last_check.at)) / 1000;
  if (Number.isNaN(atServer)) return null;
  return Math.max(0, atServer + Math.max(0, now - fetchedAt) / 1000);
}

/** Age every tile. A check that has grown old stops counting as ready or not ready. */
export function liveTiles(board: BoardData, fetchedAt: number, now: number): LiveTile[] {
  return board.workloads.map((tile) => {
    const age = checkAge(tile, board, fetchedAt, now);
    const observed = tile.state === 'ready' || tile.state === 'not_ready';
    const stale = observed && (age === null || age > board.stale_after_secs);
    return { ...tile, age, state: stale ? 'stale' : tile.state };
  });
}

export function hasOpenIncident(tile: BoardTile): boolean {
  return (tile.open_incidents?.length ?? 0) > 0;
}

export function needsAttention(tile: BoardTile): boolean {
  return hasOpenIncident(tile) || STATE_META[tile.state].rank <= 4;
}

export function tileTone(tile: BoardTile): BoardTone {
  // An open incident outranks a single ready check that has not resolved it yet.
  if (hasOpenIncident(tile) && tile.state !== 'stopped') return 'bad';
  return STATE_META[tile.state].tone;
}

function tileRank(tile: BoardTile): number {
  if (hasOpenIncident(tile)) return -1;
  return STATE_META[tile.state].rank;
}

/** Most urgent first, then by name. */
export function sortTiles<T extends BoardTile>(tiles: T[]): T[] {
  return [...tiles].sort((a, b) => tileRank(a) - tileRank(b) || a.name.localeCompare(b.name));
}

export const BOARD_FILTERS = ['all', 'attention', 'incidents', 'held', 'ready', 'stopped'] as const;
export type BoardFilter = (typeof BOARD_FILTERS)[number];

export function parseFilter(value: string): BoardFilter {
  return (BOARD_FILTERS as readonly string[]).includes(value) ? (value as BoardFilter) : 'all';
}

export function matchesFilter(tile: BoardTile, filter: BoardFilter): boolean {
  switch (filter) {
    case 'all':
      return true;
    case 'attention':
      return needsAttention(tile);
    case 'incidents':
      return hasOpenIncident(tile);
    case 'held':
      return tile.hold !== null;
    case 'ready':
      return tile.state === 'ready' && !hasOpenIncident(tile);
    case 'stopped':
      return tile.state === 'stopped';
  }
}

export function filterCounts(tiles: BoardTile[]): Record<BoardFilter, number> {
  return Object.fromEntries(
    BOARD_FILTERS.map((f) => [f, tiles.filter((t) => matchesFilter(t, f)).length]),
  ) as Record<BoardFilter, number>;
}

export interface Verdict {
  tone: BoardTone;
  title: string;
  detail: string;
}

function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/**
 * One sentence for the whole fleet. It only says everything is ready when every
 * workload has a recent ready check and every record could be read.
 */
export function verdict(tiles: BoardTile[], unavailable: string[], offline = false): Verdict {
  const now = currentVerdict(tiles, unavailable);
  if (!offline) return now;
  // Without a connection this describes the past, so it must not look all clear.
  return { ...now, tone: now.tone === 'bad' ? 'bad' : 'warn', title: `Last known: ${now.title}` };
}

function currentVerdict(tiles: BoardTile[], unavailable: string[]): Verdict {
  if (tiles.length === 0) return { tone: 'unknown', title: 'No managed workloads', detail: 'Nothing is being watched.' };
  const failing = tiles.filter((t) => tileTone(t) === 'bad').length;
  const blind = tiles.filter((t) => !hasOpenIncident(t) && ['unreadable', 'unknown', 'stale', 'no_data'].includes(t.state)).length;
  const stopped = tiles.filter((t) => t.state === 'stopped').length;
  const ready = tiles.filter((t) => t.state === 'ready' && !hasOpenIncident(t)).length;
  const rest = [
    blind > 0 ? `${blind} without current data` : '',
    stopped > 0 ? `${stopped} stopped on purpose` : '',
  ].filter(Boolean);
  if (failing > 0) {
    return {
      tone: 'bad',
      title: `${plural(failing, 'workload')} ${failing === 1 ? 'needs' : 'need'} attention`,
      detail: [`${ready} ready`, ...rest].join(', '),
    };
  }
  if (blind > 0 || unavailable.length > 0) {
    return {
      tone: 'warn',
      title: unavailable.length > 0 ? 'The board is incomplete' : `No current data for ${plural(blind, 'workload')}`,
      detail: [`${ready} ready at the last check`, stopped > 0 ? `${stopped} stopped on purpose` : ''].filter(Boolean).join(', '),
    };
  }
  if (ready === 0) {
    return { tone: 'idle', title: 'Nothing is running', detail: `${stopped} stopped on purpose` };
  }
  return {
    tone: 'ok',
    title: stopped > 0 || ready === 1 ? `${plural(ready, 'workload')} ready` : `All ${ready} workloads ready`,
    detail: stopped > 0 ? `${stopped} stopped on purpose` : 'At the last check, with no open incident',
  };
}

export const CELL_LABELS: Record<CellState, string> = {
  up: 'Ready',
  partial: 'Partly ready',
  down: 'Not ready',
  planned: 'Stopped on purpose',
  no_data: 'No data',
};

/** A cell has gaps when part of it is covered by no check. */
export function hasGaps(cell: BoardCell): boolean {
  return cell.checks > 0 && cell.covered < 0.999;
}

function cellTime(iso: string, bucketSecs: number): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const two = (n: number) => String(n).padStart(2, '0');
  const day = `${d.getMonth() + 1}/${d.getDate()}`;
  if (bucketSecs >= 86_400) return day;
  const time = `${two(d.getHours())}:${two(d.getMinutes())}`;
  return bucketSecs >= 3_600 ? `${day} ${time}` : time;
}

export function cellTitle(cell: BoardCell, bucketSecs: number): string {
  const when = `${cellTime(cell.start, bucketSecs)} (${bucketLength(bucketSecs)})`;
  if (cell.state === 'no_data') return `${when}: no checks recorded`;
  if (cell.state === 'planned') return `${when}: stopped on purpose`;
  const gaps = hasGaps(cell) ? `, ${Math.round((1 - cell.covered) * 100)}% of the period has no data` : '';
  return `${when}: ${CELL_LABELS[cell.state].toLowerCase()}, ${cell.ready} of ${plural(cell.checks, 'check')} ready${gaps}`;
}

export function bucketLength(secs: number): string {
  if (secs % 86_400 === 0) return plural(secs / 86_400, 'day');
  if (secs % 3_600 === 0) return plural(secs / 3_600, 'hour');
  return `${Math.round(secs / 60)} min`;
}

/** What a screen reader hears instead of the coloured cells. */
export function stripSummary(tile: BoardTile): string {
  const count = (s: CellState) => tile.strip.filter((c) => c.state === s).length;
  const parts = (['up', 'partial', 'down', 'planned', 'no_data'] as const)
    .filter((s) => count(s) > 0)
    .map((s) => `${count(s)} ${CELL_LABELS[s].toLowerCase()}`);
  return `${tile.name} uptime strip, ${tile.strip.length} periods: ${parts.join(', ')}`;
}

/** Rounded down, so a failed check never rounds up to 100%. */
export function readyLabel(tile: BoardTile): string {
  if (tile.ready_pct === null) return 'no data';
  const pct = Math.floor(tile.ready_pct * 10) / 10;
  return `${pct === 100 ? '100' : pct.toFixed(1)}% of ${plural(tile.checks, 'check')} ready`;
}

export function ageLabel(age: number | null): string {
  if (age === null) return 'never checked';
  if (age < 5) return 'checked just now';
  return `checked ${formatSeconds(age).replace(/ 0[smh]$/, '')} ago`;
}

/** The line under a tile's state. */
export function stateDetail(tile: LiveTile, now: number): string {
  switch (tile.state) {
    case 'ready':
    case 'not_ready':
      return `${tile.last_check?.state ?? 'unknown'} · ${ageLabel(tile.age)}`;
    case 'stale':
      return `Last ${ageLabel(tile.age)} (${tile.last_check?.ready ? 'ready' : 'not ready'} then). Current state unknown.`;
    case 'stopped': {
      const since = tile.stopped_since ? (now - Date.parse(tile.stopped_since)) / 1000 : NaN;
      return Number.isNaN(since) ? 'Stopped through Aether' : `Stopped through Aether ${formatSeconds(since)} ago`;
    }
    case 'unreadable':
      return tile.error ?? 'The runtime did not answer';
    case 'no_data':
      return 'No health check recorded yet';
    case 'unknown':
      return 'The health records could not be read';
  }
}
