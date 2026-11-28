// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useCallback, useEffect, useRef, useState } from 'react';
import { useEventStream } from '../../hooks/useEventStream';
import type { BoardData, BoardWindow } from '../../types/board';
import { apiFetchWithError } from '../../utils/api';

/** Refetch interval; covers changes made by the CLI, which emit no event. */
const POLL_MS = 20_000;
/** Events arrive in bursts (a deploy emits several); fetch once for each burst. */
const DEBOUNCE_MS = 400;
/** How often ages on screen are recomputed. */
const TICK_MS = 5_000;

/**
 * The board, kept current by the server's event stream and a slow poll. A failed
 * refresh keeps the last answer and reports the failure, so the screen can say
 * how old it is instead of going blank or looking live.
 */
export function useBoard(window_: BoardWindow, refreshKey: number | undefined, paused: boolean) {
  const [data, setData] = useState<BoardData | null>(null);
  const [fetchedAt, setFetchedAt] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [now, setNow] = useState(() => Date.now());
  const ticket = useRef(0);
  const timer = useRef<number | undefined>(undefined);

  const reload = useCallback(async () => {
    const mine = ++ticket.current;
    const asked = Date.now();
    const result = await apiFetchWithError<BoardData>(`/board?window=${window_}`);
    if (mine !== ticket.current) return;
    if (result.data) {
      setData(result.data);
      // The server answered no earlier than the request was sent.
      setFetchedAt(asked);
      setNow(Date.now());
    }
    setError(result.data ? null : (result.error ?? 'No answer from the server'));
    setLoading(false);
  }, [window_]);

  const soon = useCallback(() => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void reload(), DEBOUNCE_MS);
  }, [reload]);

  useEffect(() => {
    void reload();
  }, [reload, refreshKey]);

  useEffect(() => {
    if (paused) return;
    void reload();
    const poll = window.setInterval(() => void reload(), POLL_MS);
    return () => {
      window.clearInterval(poll);
      window.clearTimeout(timer.current);
    };
  }, [reload, paused]);

  useEffect(() => {
    const tick = window.setInterval(() => setNow(Date.now()), TICK_MS);
    return () => window.clearInterval(tick);
  }, []);

  const { connected } = useEventStream('', soon, !paused);

  return {
    // A response for another window must not be drawn under this one's labels.
    data: data && data.window === window_ ? data : null,
    fetchedAt,
    error,
    loading,
    now,
    connected,
    reload,
  };
}
