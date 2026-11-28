// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { Link } from 'react-router';
import type { LiveTile } from '../../utils/board';
import { STATE_META, readyLabel, stateDetail, stripSummary, tileTone } from '../../utils/board';
import type { TimelineCategory } from '../../types/timeline';
import type { TimelineRange } from '../../utils/timeline';
import Drawer from '../ui/Drawer';
import { ToneIcon } from './StatusTile';
import TimelineView from './TimelineView';
import UptimeStrip from './UptimeStrip';

interface WorkloadDrawerProps {
  name: string;
  /** Absent for a workload that is no longer managed. */
  tile?: LiveTile;
  bucketSecs: number;
  windowLabel: string;
  now: number;
  refreshKey: number;
  range: TimelineRange;
  only: TimelineCategory[];
  onRange: (range: TimelineRange) => void;
  onOnly: (only: TimelineCategory[]) => void;
  onClose: () => void;
  stepper?: { index: number; count: number; onStep: (delta: 1 | -1) => void };
}

/** One workload: its current state and everything recorded about it. */
export default function WorkloadDrawer({ name, tile, bucketSecs, windowLabel, now, onClose, stepper, ...timeline }: WorkloadDrawerProps) {
  return (
    <Drawer label={`${name} timeline`} onClose={onClose} contentKey={name} stepper={stepper} testId="board-drawer">
      <div className="board board-drawer">
        <header>
          <h2 className="text-xl font-semibold text-foreground">{name}</h2>
          {tile ? (
            <div data-tone={tileTone(tile)}>
              <p className="board-tile-state">
                <ToneIcon tone={tileTone(tile)} className="board-tile-icon" />
                <span>{STATE_META[tile.state].label}</span>
              </p>
              <p className="board-tile-detail">{stateDetail(tile, now)}</p>
              <p className="break-all font-mono text-xs text-subtle">
                {tile.runtime} · {tile.image}
              </p>
            </div>
          ) : (
            <p className="text-sm text-muted">Not managed now. Its recorded history is below.</p>
          )}
        </header>
        {tile ? (
          <div>
            <UptimeStrip cells={tile.strip} bucketSecs={bucketSecs} summary={stripSummary(tile)} />
            <div className="board-tile-axis">
              <span>{windowLabel} ago</span>
              <strong>{readyLabel(tile)}</strong>
              <span>now</span>
            </div>
          </div>
        ) : null}
        {tile ? (
          <Link className="timeline-link focus-ring" to={`/operations?tab=workload&workload=${encodeURIComponent(name)}`}>
            Explain, preview and roll back in the Operations Desk
          </Link>
        ) : null}
        <TimelineView key={name} workload={name} {...timeline} />
      </div>
    </Drawer>
  );
}
