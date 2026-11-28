// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { BoardCell, CellState } from '../../types/board';
import { CELL_LABELS, cellTitle, hasGaps } from '../../utils/board';

/**
 * One cell per period, oldest on the left. Each state differs in shape or
 * pattern as well as colour; a period without checks is an empty dashed slot.
 */
export default function UptimeStrip({ cells, bucketSecs, summary }: { cells: BoardCell[]; bucketSecs: number; summary: string }) {
  return (
    <div className="uptime-strip" role="img" aria-label={summary}>
      {cells.map((c) => (
        <span key={c.start} className="uptime-cell" data-state={c.state} data-gaps={hasGaps(c) || undefined} title={cellTitle(c, bucketSecs)} />
      ))}
    </div>
  );
}

const LEGEND: CellState[] = ['up', 'partial', 'down', 'planned', 'no_data'];

export function StripLegend() {
  return (
    <ul className="uptime-legend" aria-label="Uptime strip legend">
      {LEGEND.map((s) => (
        <li key={s}>
          <span className="uptime-cell" data-state={s} aria-hidden />
          {CELL_LABELS[s]}
        </li>
      ))}
      <li>
        <span className="uptime-cell" data-state="up" data-gaps aria-hidden />
        Has gaps
      </li>
    </ul>
  );
}
