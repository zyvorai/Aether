// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useCallback, useRef, useState } from 'react';

/**
 * Lets the viewer hold automatic refreshes while reading. Updates that arrive
 * while paused are counted and applied once on resume.
 */
export function useLiveUpdates(apply: () => void) {
  const [paused, setPaused] = useState(false);
  const [held, setHeld] = useState(0);
  const pausedRef = useRef(false);
  const heldRef = useRef(0);
  const applyRef = useRef(apply);
  applyRef.current = apply;

  const requestRefresh = useCallback(() => {
    if (!pausedRef.current) {
      applyRef.current();
      return;
    }
    heldRef.current += 1;
    setHeld(heldRef.current);
  }, []);

  const togglePause = useCallback(() => {
    pausedRef.current = !pausedRef.current;
    setPaused(pausedRef.current);
    if (!pausedRef.current && heldRef.current > 0) applyRef.current();
    heldRef.current = 0;
    setHeld(0);
  }, []);

  return { paused, held, requestRefresh, togglePause };
}
