// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useState } from 'react';
import { Pause, Play, RefreshCw } from 'lucide-react';
import { liveState } from '../../../utils/liveState';
import NavMenu from './NavMenu';
import styles from './GlobalNav.module.css';

export interface LiveStatusProps {
  connected: boolean;
  paused: boolean;
  heldUpdates: number;
  lastRefreshed: Date;
  onTogglePause: () => void;
  onRefresh: () => void;
}

/** Re-render every few seconds so "12s ago" stays true. */
function useTick(intervalMs: number) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(timer);
  }, [intervalMs]);
  return now;
}

/** Whether the data on screen is moving, and the controls to hold it still. */
export default function LiveStatus({ connected, paused, heldUpdates, lastRefreshed, onTogglePause, onRefresh }: LiveStatusProps) {
  const now = useTick(5000);
  const state = liveState({ connected, paused, heldUpdates, lastRefreshed, now });

  return (
    <NavMenu
      kind="dialog"
      title="Data freshness"
      label={state.description}
      triggerClassName={styles.live}
      trigger={
        <>
          <span className={styles.statusDot} data-state={state.tone} aria-hidden />
          <span className={styles.liveText}>{state.label}</span>
        </>
      }
    >
      {() => (
        <div className="w-72 p-2">
          <p className="text-sm font-semibold text-foreground">{state.label}</p>
          <p className="mt-1 text-xs leading-relaxed text-muted">{state.explanation}</p>
          <dl className="mt-3 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
            <dt className="text-subtle">Event stream</dt>
            <dd className="text-foreground">{connected ? 'Connected' : 'Reconnecting'}</dd>
            <dt className="text-subtle">Last refresh</dt>
            <dd className="text-foreground">{lastRefreshed.toLocaleTimeString()}</dd>
          </dl>
          <div className="mt-3 flex gap-2">
            <button type="button" className="btn-quiet" onClick={onTogglePause} aria-pressed={paused}>
              {paused ? <Play className="h-3.5 w-3.5" aria-hidden /> : <Pause className="h-3.5 w-3.5" aria-hidden />}
              {paused ? 'Resume updates' : 'Pause updates'}
            </button>
            <button type="button" className="btn-quiet" onClick={onRefresh}>
              <RefreshCw className="h-3.5 w-3.5" aria-hidden />
              Refresh now
            </button>
          </div>
        </div>
      )}
    </NavMenu>
  );
}
