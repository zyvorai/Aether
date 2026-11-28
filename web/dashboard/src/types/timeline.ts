// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/** Types for the workload timeline API (`src/ops/timeline.rs`). */

import type { Level } from './operator';

export type TimelineCategory = 'change' | 'health' | 'incident' | 'note' | 'hold';

export interface TimelineEntry {
  at: string;
  category: TimelineCategory;
  kind: string;
  level: Level;
  title: string;
  detail: string;
  actor: string | null;
  revision: number | null;
  previous_revision: number | null;
  incident: number | null;
  fields: string[];
}

export interface Timeline {
  workload: string;
  managed: boolean;
  since: string;
  until: string;
  counts: Record<TimelineCategory, number>;
  total: number;
  truncated: boolean;
  health: { checks: number; ready: number; retained_since: string | null; last_check_at: string | null };
  entries: TimelineEntry[];
  unavailable: string[];
}
