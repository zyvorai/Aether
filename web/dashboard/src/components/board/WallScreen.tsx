// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

type WakeLock = { release: () => Promise<void> };
type WakeLockNavigator = Navigator & { wakeLock?: { request: (type: 'screen') => Promise<WakeLock> } };

/** Ask the browser for full screen. Allowed only from a click; refusal is fine. */
export function enterFullscreen() {
  document.documentElement.requestFullscreen?.().catch(() => {});
}

/**
 * Covers the whole window for a shared monitor, keeps the screen awake where the
 * browser allows it, and leaves on Escape.
 */
export default function WallScreen({ onExit, children }: { onExit: () => void; children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  const exit = useRef(onExit);
  exit.current = onExit;

  useEffect(() => {
    const app = document.getElementById('root');
    const opener = document.activeElement as HTMLElement | null;
    app?.setAttribute('inert', '');
    document.body.classList.add('board-wall-open');
    ref.current?.focus();

    const onKey = (e: KeyboardEvent) => {
      // A drawer or dialog above the wall handles its own Escape.
      if (e.key !== 'Escape' || e.defaultPrevented || document.querySelector('.drawer-root')) return;
      exit.current();
    };
    // In full screen the browser takes Escape for itself; follow it out.
    let wasFullscreen = Boolean(document.fullscreenElement);
    const onFullscreen = () => {
      const is = Boolean(document.fullscreenElement);
      if (wasFullscreen && !is) exit.current();
      wasFullscreen = is;
    };

    let lock: WakeLock | undefined;
    let closed = false;
    const keepAwake = () => {
      if (document.visibilityState !== 'visible') return;
      (navigator as WakeLockNavigator).wakeLock
        ?.request('screen')
        .then((l) => {
          if (closed) void l.release();
          else lock = l;
        })
        .catch(() => {});
    };
    keepAwake();

    document.addEventListener('keydown', onKey);
    document.addEventListener('fullscreenchange', onFullscreen);
    document.addEventListener('visibilitychange', keepAwake);
    return () => {
      closed = true;
      void lock?.release().catch(() => {});
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('fullscreenchange', onFullscreen);
      document.removeEventListener('visibilitychange', keepAwake);
      document.body.classList.remove('board-wall-open');
      app?.removeAttribute('inert');
      if (document.fullscreenElement) void document.exitFullscreen?.().catch(() => {});
      if (opener?.isConnected) opener.focus();
    };
  }, []);

  return createPortal(
    <div ref={ref} className="board-wall" role="dialog" aria-modal="true" aria-label="Status wall" tabIndex={-1} data-testid="board-wall">
      {children}
    </div>,
    document.body,
  );
}
