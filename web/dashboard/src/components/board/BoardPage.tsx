// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useMemo, useState } from 'react';
import { useSearchParams } from 'react-router';
import { Activity, Maximize2, Minimize2, RefreshCw } from 'lucide-react';
import { useAuth } from '../../contexts/AuthContext';
import { apiPost } from '../../utils/api';
import {
  BOARD_FILTERS,
  BOARD_WINDOWS,
  filterCounts,
  liveTiles,
  matchesFilter,
  parseFilter,
  parseWindow,
  sortTiles,
  verdict,
  type BoardFilter,
} from '../../utils/board';
import { friendlyError } from '../../utils/operator';
import EmptyState from '../EmptyState';
import PageLoadError from '../PageLoadError';
import PageLoading from '../PageLoading';
import { withAuroraPage } from '../layout/AuroraPage';
import { Button } from '../ui/Button';
import SegmentedControl from '../ui/SegmentedControl';
import StatusTile, { ToneIcon } from './StatusTile';
import { StripLegend } from './UptimeStrip';
import { useBoard } from './useBoard';
import WallScreen, { enterFullscreen } from './WallScreen';
import './board.css';

const FILTER_LABELS: Record<BoardFilter, string> = {
  all: 'All',
  attention: 'Attention',
  incidents: 'Incidents',
  held: 'Held',
  ready: 'Ready',
  stopped: 'Stopped',
};

function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

function BoardPage({ refreshKey, paused = false }: { refreshKey?: number; paused?: boolean }) {
  const [params, setParams] = useSearchParams();
  const window_ = parseWindow(params.get('window') ?? '');
  const filter = parseFilter(params.get('show') ?? '');
  const wall = params.get('wall') === '1';
  const board = useBoard(window_, refreshKey, paused);
  const { canMutate } = useAuth();
  const [checking, setChecking] = useState(false);
  const [checkError, setCheckError] = useState<string | null>(null);

  function set(changes: Record<string, string>) {
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        for (const [key, value] of Object.entries(changes)) {
          if (value) next.set(key, value);
          else next.delete(key);
        }
        return next;
      },
      { replace: true },
    );
  }

  const tiles = useMemo(
    () => (board.data ? sortTiles(liveTiles(board.data, board.fetchedAt, board.now)) : []),
    [board.data, board.fetchedAt, board.now],
  );

  async function checkNow() {
    setChecking(true);
    setCheckError(null);
    const res = await apiPost('/observe');
    setChecking(false);
    if (!res.success) setCheckError(friendlyError(res.error));
    void board.reload();
  }

  if (!board.data) {
    if (board.error && !board.loading) {
      return <PageLoadError title="Status board unavailable" description={board.error} onRetry={() => void board.reload()} />;
    }
    return <PageLoading rows={4} />;
  }

  const data = board.data;
  const summary = verdict(tiles, data.unavailable);
  const counts = filterCounts(tiles);
  const shown = tiles.filter((t) => matchesFilter(t, filter));
  const windowLabel = BOARD_WINDOWS.find((w) => w.key === window_)?.label ?? window_;
  const link = paused ? 'paused' : board.error ? 'offline' : board.connected ? 'live' : 'polling';
  const LINK_LABELS = { paused: 'Updates paused', offline: 'Not updating', live: 'Live', polling: 'Polling every 20s' };

  const notices = (
    <>
      {board.error ? (
        <p className="board-notice" data-tone="bad" role="alert">
          Cannot reach the server ({board.error}). Showing what it reported at {clock(board.fetchedAt)}; ages keep counting.
        </p>
      ) : null}
      {data.observe_secs === 0 ? (
        <p className="board-notice" data-tone="warn">
          This server does not run health checks (AETHER_OBSERVE_SECS=0). Run <code>aether health collect</code> from a timer, or the board goes stale.
        </p>
      ) : null}
      {data.unavailable.map((u) => (
        <p key={u} className="board-notice" data-tone="warn" role="alert">
          Could not read {u}
        </p>
      ))}
      {checkError ? (
        <p className="board-notice" data-tone="bad" role="alert">
          {checkError}
        </p>
      ) : null}
    </>
  );

  const headline = (
    <div className="board-verdict" data-tone={summary.tone} data-testid="board-verdict" role="status">
      <ToneIcon tone={summary.tone} className="board-verdict-icon" />
      <div>
        <p className="board-verdict-title">{summary.title}</p>
        <p className="board-verdict-detail">{summary.detail}</p>
      </div>
      <div className="board-link" data-link={link}>
        <span className="board-link-dot" aria-hidden />
        <span>{LINK_LABELS[link]}</span>
        <span className="board-link-time">Updated {clock(board.fetchedAt)}</span>
      </div>
    </div>
  );

  const controls = (
    <div className="board-controls">
      <div className="board-scroll">
        <SegmentedControl
          label="History window"
          value={window_}
          onChange={(w) => set({ window: w === '24h' ? '' : w })}
          items={BOARD_WINDOWS.map((w) => ({ key: w.key, label: w.key }))}
        />
      </div>
      <div className="board-scroll">
        <SegmentedControl
          label="Show"
          value={filter}
          onChange={(f) => set({ show: f === 'all' ? '' : f })}
          items={BOARD_FILTERS.map((f) => ({ key: f, label: FILTER_LABELS[f], count: counts[f] }))}
        />
      </div>
      <div className="board-buttons">
        {canMutate && !wall ? (
          <Button size="sm" variant="secondary" disabled={checking} onClick={() => void checkNow()}>
            <RefreshCw className={`mr-1.5 h-4 w-4 ${checking ? 'animate-spin' : ''}`} aria-hidden />
            {checking ? 'Checking…' : 'Check now'}
          </Button>
        ) : null}
        <Button
          size="sm"
          variant={wall ? 'secondary' : 'primary'}
          onClick={() => {
            if (!wall) enterFullscreen();
            set({ wall: wall ? '' : '1' });
          }}
        >
          {wall ? <Minimize2 className="mr-1.5 h-4 w-4" aria-hidden /> : <Maximize2 className="mr-1.5 h-4 w-4" aria-hidden />}
          {wall ? 'Exit wall mode' : 'Wall mode'}
        </Button>
      </div>
    </div>
  );

  const grid =
    tiles.length === 0 ? (
      <EmptyState icon={<Activity size={40} />} title="No managed workloads yet" description="Deploy one with aether run and it appears here after its first health check." />
    ) : shown.length === 0 ? (
      <EmptyState icon={<Activity size={40} />} title="Nothing matches this filter" action={<Button size="sm" variant="secondary" onClick={() => set({ show: '' })}>Show all</Button>} />
    ) : (
      <div className="board-grid" data-testid="board-grid">
        {shown.map((tile) => (
          <StatusTile key={tile.name} tile={tile} bucketSecs={data.bucket_secs} windowLabel={windowLabel} now={board.now} />
        ))}
      </div>
    );

  const content = (
    <div className="board">
      {headline}
      {notices}
      {controls}
      {grid}
      {tiles.length > 0 ? <StripLegend /> : null}
    </div>
  );

  if (wall) {
    return (
      <WallScreen onExit={() => set({ wall: '' })}>
        <header className="board-wall-head">
          <h2>Aether status</h2>
          <time dateTime={new Date(board.now).toISOString()}>{clock(board.now)}</time>
        </header>
        {content}
      </WallScreen>
    );
  }
  return content;
}

export default withAuroraPage('board', BoardPage);
