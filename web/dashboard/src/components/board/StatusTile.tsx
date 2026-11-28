// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { AlertOctagon, CheckCircle2, CircleDashed, CircleHelp, CirclePause, Lock, Network, TriangleAlert } from 'lucide-react';
import { RuntimeBadge } from '../Badge';
import type { LiveTile, BoardTone } from '../../utils/board';
import { STATE_META, readyLabel, stateDetail, stripSummary, tileTone } from '../../utils/board';
import { formatRelativeTime } from '../../utils/formatters';
import { formatSeconds } from '../../utils/operator';
import UptimeStrip from './UptimeStrip';

const ICONS: Record<BoardTone, typeof CheckCircle2> = {
  ok: CheckCircle2,
  bad: AlertOctagon,
  warn: TriangleAlert,
  idle: CirclePause,
  unknown: CircleDashed,
};

export function ToneIcon({ tone, className }: { tone: BoardTone; className?: string }) {
  const Icon = ICONS[tone] ?? CircleHelp;
  return <Icon className={className} aria-hidden />;
}

interface StatusTileProps {
  tile: LiveTile;
  bucketSecs: number;
  windowLabel: string;
  now: number;
  onOpen?: (name: string) => void;
  /** Extra controls shown under the tile. */
  footer?: React.ReactNode;
  /** Blast radius highlighting: this tile's part in it, or `dim` when it has none. */
  highlight?: 'source' | 'direct' | 'indirect' | 'dim';
  highlightLabel?: string;
}

export default function StatusTile({ tile, bucketSecs, windowLabel, now, onOpen, footer, highlight, highlightLabel }: StatusTileProps) {
  const tone = tileTone(tile);
  const incidents = tile.open_incidents ?? [];
  return (
    <article className="board-tile" data-tone={tone} data-highlight={highlight} data-testid={`tile-${tile.name}`} aria-label={`${tile.name}: ${STATE_META[tile.state].label}`}>
      <header className="board-tile-head">
        <h3>
          {onOpen ? (
            <button type="button" className="board-tile-open focus-ring" onClick={() => onOpen(tile.name)} title={`Open ${tile.name}`}>
              {tile.name}
            </button>
          ) : (
            tile.name
          )}
        </h3>
        <RuntimeBadge runtime={tile.runtime} />
      </header>
      {highlightLabel ? <p className="board-tile-radius">{highlightLabel}</p> : null}
      <p className="board-tile-state">
        <ToneIcon tone={tone} className="board-tile-icon" />
        <span>{STATE_META[tile.state].label}</span>
      </p>
      <p className="board-tile-detail">{stateDetail(tile, now)}</p>

      {tile.open_incidents === null ? <p className="board-chip" data-kind="warn">Incidents could not be read</p> : null}
      {incidents.map((i) => (
        <p key={i.id} className="board-chip" data-kind="incident">
          <AlertOctagon aria-hidden />
          <span>
            Incident {i.id} open {formatSeconds((now - Date.parse(i.opened_at)) / 1000)}
            {i.acknowledged ? ', acknowledged' : ', not acknowledged'}
          </span>
        </p>
      ))}
      {tile.hold ? (
        <p className="board-chip" data-kind="hold" title={`Held by ${tile.hold.actor}`}>
          <Lock aria-hidden />
          <span>On hold: {tile.hold.reason}</span>
        </p>
      ) : null}

      <div className="board-tile-history">
        <UptimeStrip cells={tile.strip} bucketSecs={bucketSecs} summary={stripSummary(tile)} />
        <div className="board-tile-axis">
          <span>{windowLabel} ago</span>
          <strong>{readyLabel(tile)}</strong>
          <span>now</span>
        </div>
      </div>

      <div className="board-tile-meta">
        <span>
          {tile.last_change
            ? `Rev ${tile.last_change.revision} ${tile.last_change.op === 'deploy' ? 'deployed' : 'updated'} ${formatRelativeTime(tile.last_change.at)}`
            : 'No recorded revision'}
        </span>
        {tile.dependents.length > 0 ? (
          <span title={`Needed by ${tile.dependents.join(', ')}`}>
            <Network aria-hidden />
            {tile.dependents.length} depend{tile.dependents.length === 1 ? 's' : ''} on it
          </span>
        ) : null}
      </div>
      {footer ? <div className="board-tile-actions">{footer}</div> : null}
    </article>
  );
}
