// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/** Types for the status board API (`src/ops/board.rs`). */

export type BoardWindow = '1h' | '6h' | '24h' | '7d' | '30d';

export type CellState = 'up' | 'partial' | 'down' | 'planned' | 'no_data';

export interface BoardCell {
  start: string;
  state: CellState;
  checks: number;
  ready: number;
  /** Share of the elapsed part of the cell that is accounted for. */
  covered: number;
}

export type TileState = 'ready' | 'not_ready' | 'stopped' | 'stale' | 'unreadable' | 'no_data' | 'unknown';

export interface BoardTile {
  name: string;
  runtime: string;
  image: string;
  state: TileState;
  last_check: { at: string; state: string; ready: boolean; restart_count: number } | null;
  stopped_since: string | null;
  error: string | null;
  strip: BoardCell[];
  ready_pct: number | null;
  checks: number;
  /** `null` when incidents could not be read. */
  open_incidents: { id: number; opened_at: string; failed_checks: number; acknowledged: boolean }[] | null;
  hold: { workload: string; reason: string; actor: string; created_at: string; expires_at: string | null } | null;
  last_change: { revision: number; previous_revision: number | null; at: string; actor: string; op: 'deploy' | 'update' } | null;
  depends_on: string[];
  dependents: string[];
}

export interface BoardData {
  generated_at: string;
  window: BoardWindow;
  bucket_secs: number;
  observe_secs: number;
  stale_after_secs: number;
  incident_threshold: number;
  workloads: BoardTile[];
  unavailable: string[];
}
